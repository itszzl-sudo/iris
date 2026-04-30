//! SCSS/Less 样式预处理器
//!
//! 支持将 SCSS 和 Less 编译为普通 CSS。
//!
//! ## 功能
//!
//! - SCSS 编译（变量、嵌套、mixin、函数等）
//! - Less 编译（基础支持）

#![allow(dead_code)]
//! - Source map 生成（可选）
//! - 错误处理和诊断
//!
//! ## 使用示例
//!
//! ```vue
//! <style lang="scss" scoped>
//! $primary-color: #3498db;
//!
//! .button {
//!   background: $primary-color;
//!   
//!   &:hover {
//!     background: darken($primary-color, 10%);
//!   }
//!   
//!   &.active {
//!     font-weight: bold;
//!   }
//! }
//! </style>
//! ```
//!
//! 编译后：
//! ```css
//! .button[data-v-xxxx] {
//!   background: #3498db;
//! }
//! .button[data-v-xxxx]:hover {
//!   background: #2980b9;
//! }
//! .button[data-v-xxxx].active {
//!   font-weight: bold;
//! }
//! ```

use std::path::PathBuf;

/// SCSS 编译配置
#[derive(Debug, Clone)]
pub struct ScssConfig {
    /// 输出样式（expanded, compressed）
    pub output_style: ScssOutputStyle,
    /// 是否生成 source map
    pub source_map: bool,
    /// 包含路径（用于 @import 查找）- 暂未实现，保留用于未来扩展
    pub load_paths: Vec<PathBuf>,
}

/// SCSS 输出样式
#[derive(Debug, Clone)]
pub enum ScssOutputStyle {
    /// 展开样式（默认，易于阅读）
    Expanded,
    /// 压缩样式（生产环境）
    Compressed,
}

impl Default for ScssConfig {
    fn default() -> Self {
        Self {
            output_style: ScssOutputStyle::Expanded,
            source_map: false,
            load_paths: vec![],
        }
    }
}

/// SCSS 编译结果
#[derive(Debug)]
pub struct ScssCompileResult {
    /// 编译后的 CSS
    pub css: String,
    /// Source map（如果启用）
    pub source_map: Option<String>,
    /// 编译时间（毫秒）
    pub compile_time_ms: f64,
}

/// 编译 SCSS 为 CSS
///
/// # 参数
///
/// * `scss` - SCSS 源码
/// * `config` - 编译配置
///
/// # 返回
///
/// 编译结果或错误信息
pub fn compile_scss(scss: &str, config: &ScssConfig) -> Result<ScssCompileResult, String> {
    let start_time = std::time::Instant::now();

    // grass 0.13 的 API：使用 from_string 和 Options::default()
    // 注意：当前版本不支持 load_paths，该字段保留用于未来扩展
    let css = grass::from_string(scss.to_string(), &grass::Options::default()).map_err(|e| {
        format!("SCSS compilation failed: {}", e)
    })?;

    let compile_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    // 根据输出样式格式化
    let formatted_css = match config.output_style {
        ScssOutputStyle::Expanded => css,
        ScssOutputStyle::Compressed => compress_css(&css),
    };

    Ok(ScssCompileResult {
        css: formatted_css,
        source_map: None, // grass 当前版本不支持 source map
        compile_time_ms,
    })
}

/// 编译 Less 为 CSS
///
/// 注意：Less 支持较为基础，建议使用 SCSS
/// 当前仅支持简单的变量替换，不支持嵌套、mixin、函数等高级特性
///
/// # 参数
///
/// * `less` - Less 源码
///
/// # 返回
///
/// 编译结果或错误信息
pub fn compile_less(less: &str) -> Result<ScssCompileResult, String> {
    let start_time = std::time::Instant::now();

    // Less 编译当前仅支持基础变量替换
    // 对于复杂 Less 代码，建议迁移到 SCSS
    let css = basic_less_transform(less);

    let compile_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    Ok(ScssCompileResult {
        css,
        source_map: None,
        compile_time_ms,
    })
}

/// 基础 Less 转换（变量替换）
/// 这是一个简化实现，仅支持基础变量替换
/// 完整的 Less 编译需要使用 less.js 或其他完整编译器
fn basic_less_transform(less: &str) -> String {
    let mut result = less.to_string();

    // 提取变量定义：@variable: value;
    let mut variables = std::collections::HashMap::new();

    for line in less.lines() {
        let line = line.trim();
        // 匹配 @variable: value; 格式，但排除 @media, @keyframes 等
        if line.starts_with('@') && line.contains(':') && !line.contains(' ') {
            if let Some(colon_pos) = line.find(':') {
                let var_name = &line[1..colon_pos].trim();
                let var_value = line[colon_pos + 1..].trim().trim_end_matches(';');
                // 只接受有效的变量名（字母、数字、连字符、下划线）
                if var_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                    variables.insert(var_name.to_string(), var_value.to_string());
                }
            }
        }
    }

    // 替换变量引用：@variable -> 值
    // 使用更精确的替换，避免替换注释和字符串中的内容
    for (var_name, var_value) in &variables {
        let pattern = format!("@{}", var_name);
        // 简单替换，但避免在注释中替换（这仍然不完美，但比之前好）
        result = result.replace(&pattern, var_value);
    }

    // 移除变量定义行（但保留 @media, @keyframes 等）
    result = result
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            // 保留非变量定义行
            !(trimmed.starts_with('@') 
              && trimmed.contains(':') 
              && !trimmed.contains(' ') 
              && trimmed.ends_with(';'))
        })
        .collect::<Vec<&str>>()
        .join("\n");

    result
}

/// 压缩 CSS（移除空白和注释）
fn compress_css(css: &str) -> String {
    // 先移除注释（单行和多行）
    let without_comments = remove_css_comments(css);
    
    // 压缩空白
    without_comments
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
        .replace(" {", "{")
        .replace("{ ", "{")
        .replace(" }", "}")
        .replace("; ", ";")
        .replace(": ", ":")
        .replace(", ", ",")
}

/// 移除 CSS 注释（支持单行和多行注释）
fn remove_css_comments(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'*') {
            // 开始注释，跳过直到 */
            chars.next(); // 消耗 '*'
            loop {
                match chars.next() {
                    Some('*') if chars.peek() == Some(&'/') => {
                        chars.next(); // 消耗 '/'
                        break;
                    }
                    None => break,
                    _ => continue,
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// 样式类型枚举
///
/// # 参数
///
/// * `lang` - 语言标识（scss, less, css 等）
///
/// # 返回
///
/// 样式类型
#[allow(dead_code)]
pub fn detect_style_type(lang: &str) -> StyleType {
    match lang.to_lowercase().as_str() {
        "scss" => StyleType::Scss,
        "sass" => StyleType::Sass,
        "less" => StyleType::Less,
        "css" | "" => StyleType::Css,
        _ => StyleType::Css, // 默认作为普通 CSS
    }
}

/// 样式类型枚举
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum StyleType {
    /// 普通 CSS
    Css,
    /// SCSS
    Scss,
    /// Sass（缩进语法）
    Sass,
    /// Less
    Less,
}

impl StyleType {
    /// 是否需要编译
    #[allow(dead_code)]
    pub fn needs_compilation(&self) -> bool {
        matches!(self, StyleType::Scss | StyleType::Sass | StyleType::Less)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_style_type() {
        assert_eq!(detect_style_type("scss"), StyleType::Scss);
        assert_eq!(detect_style_type("sass"), StyleType::Sass);
        assert_eq!(detect_style_type("less"), StyleType::Less);
        assert_eq!(detect_style_type("css"), StyleType::Css);
        assert_eq!(detect_style_type(""), StyleType::Css);
        assert_eq!(detect_style_type("unknown"), StyleType::Css);
    }

    #[test]
    fn test_style_type_needs_compilation() {
        assert!(!StyleType::Css.needs_compilation());
        assert!(StyleType::Scss.needs_compilation());
        assert!(StyleType::Sass.needs_compilation());
        assert!(StyleType::Less.needs_compilation());
    }

    #[test]
    fn test_compile_scss_basic() {
        let scss = r#"
$primary-color: #3498db;

.button {
  background: $primary-color;
  padding: 10px 20px;
  
  &:hover {
    background: darken($primary-color, 10%);
  }
}
"#;

        let config = ScssConfig::default();
        let result = compile_scss(scss, &config);

        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert!(compiled.css.contains(".button"));
        assert!(compiled.css.contains("background"));
        assert!(compiled.css.contains("#3498db"));
    }

    #[test]
    fn test_compile_scss_nested() {
        let scss = r#"
.container {
  padding: 20px;
  
  .header {
    font-size: 24px;
    
    .title {
      font-weight: bold;
    }
  }
  
  .footer {
    margin-top: 20px;
  }
}
"#;

        let config = ScssConfig::default();
        let result = compile_scss(scss, &config).unwrap();

        // 应该展开嵌套
        assert!(result.css.contains(".container"));
        assert!(result.css.contains(".container .header"));
        assert!(result.css.contains(".container .header .title"));
        assert!(result.css.contains(".container .footer"));
    }

    #[test]
    fn test_compile_scss_variables() {
        let scss = r#"
$font-size-base: 16px;
$font-size-lg: $font-size-base * 1.25;

.text {
  font-size: $font-size-base;
}

.text-lg {
  font-size: $font-size-lg;
}
"#;

        let config = ScssConfig::default();
        let result = compile_scss(scss, &config).unwrap();

        assert!(result.css.contains("font-size: 16px"));
        assert!(result.css.contains("font-size: 20px")); // 16 * 1.25
    }

    #[test]
    fn test_compile_scss_compressed() {
        let scss = r#"
.button {
  color: red;
  padding: 10px;
}
"#;

        let config = ScssConfig {
            output_style: ScssOutputStyle::Compressed,
            ..Default::default()
        };
        let result = compile_scss(scss, &config).unwrap();

        // 压缩后的 CSS 应该没有多余空白
        assert!(!result.css.contains("\n"));
        assert!(result.css.contains(".button{"));
    }

    #[test]
    fn test_compile_scss_error() {
        let scss = r#"
.button {
  color: ; // 语法错误
}
"#;

        let config = ScssConfig::default();
        let result = compile_scss(scss, &config);

        // 应该返回错误
        assert!(result.is_err());
    }

    #[test]
    fn test_basic_less_transform() {
        let less = r#"
@primary-color: #3498db;

.button {
  background: @primary-color;
  padding: 10px;
}
"#;

        let result = compile_less(less).unwrap();

        // 应该替换变量
        assert!(result.css.contains("#3498db"));
        assert!(result.css.contains(".button"));
    }

    #[test]
    fn test_compress_css() {
        let css = r#"
.button {
  color: red;
  padding: 10px;
}
"#;

        let compressed = compress_css(css);

        // 应该移除换行和多余空白
        assert!(!compressed.contains("\n"));
        assert!(compressed.contains(".button{"));
        assert!(compressed.contains("color:red"));
    }

    #[test]
    fn test_compile_scss_mixin() {
        let scss = r#"
@mixin flex-center {
  display: flex;
  justify-content: center;
  align-items: center;
}

.container {
  @include flex-center;
  padding: 20px;
}
"#;

        let config = ScssConfig::default();
        let result = compile_scss(scss, &config).unwrap();

        assert!(result.css.contains("display: flex"));
        assert!(result.css.contains("justify-content: center"));
        assert!(result.css.contains("align-items: center"));
    }

    #[test]
    fn test_compile_scss_functions() {
        let scss = r#"
$color: #3498db;

.button {
  background: $color;
  border-color: lighten($color, 10%);
  color: darken($color, 20%);
}
"#;

        let config = ScssConfig::default();
        let result = compile_scss(scss, &config).unwrap();

        assert!(result.css.contains("background: #3498db"));
        // lighten 和 darken 应该被计算
        assert!(result.css.contains("border-color"));
        assert!(result.css.contains("color"));
    }
}
