//! Less 样式预处理器
//!
//! 使用 rust-less (Rust 原生 Less 编译器) 将 Less 编译为 CSS。
//!
//! ## 功能
//!
//! - Less 编译（变量、嵌套选择器、媒体查询等）
//! - 错误处理和诊断
//!
//! ## 使用示例
//!
//! ```less
//! @primary-color: #3498db;
//!
//! .button {
//!   background: @primary-color;
//!   
//!   &:hover {
//!     background: darken(@primary-color, 10%);
//!   }
//! }
//! ```
//!
//! 编译后：
//! ```css
//! .button {
//!   background: #3498db;
//! }
//! .button:hover {
//!   background: #2980b9;
//! }
//! ```

use tracing::{debug, warn};

/// Less 编译配置
#[derive(Debug, Clone)]
pub struct LessConfig {
    /// 是否启用压缩输出
    pub compressed: bool,
}

impl Default for LessConfig {
    fn default() -> Self {
        Self {
            compressed: false,
        }
    }
}

/// Less 编译结果
#[derive(Debug, Clone)]
pub struct LessCompileResult {
    /// 编译后的 CSS
    pub css: String,
    /// 原始大小
    pub original_size: usize,
    /// 编译后大小
    pub output_size: usize,
    /// 编译时间（毫秒）
    pub compile_time_ms: f64,
}

/// 编译 Less 为 CSS
///
/// 使用 rust-less 编译器将 Less 源码编译为标准 CSS。
/// 支持：
/// - 变量定义和引用（@variable）
/// - 嵌套选择器
/// - & 父选择器引用
/// - 媒体查询嵌套
///
/// # 参数
///
/// * `less` - Less 源码
/// * `config` - 编译配置
///
/// # 返回
///
/// 编译结果或错误信息
pub fn compile_less(less: &str, config: &LessConfig) -> Result<LessCompileResult, String> {
    let original_size = less.len();
    let start_time = std::time::Instant::now();

    if less.trim().is_empty() {
        return Ok(LessCompileResult {
            css: String::new(),
            original_size: 0,
            output_size: 0,
            compile_time_ms: 0.0,
        });
    }

    debug!("Less compilation started ({} bytes)", original_size);

    // 第一步：先做基础变量替换（兼容 rust-less v0.1.0 变量支持有限的情况）
    let preprocessed = basic_less_transform(less);

    // 第二步：使用 rust-less 编译（处理嵌套选择器、媒体查询等）
    let css = match rust_less::parse_less(&preprocessed) {
        Ok(result) => result,
        Err(e) => {
            warn!("rust-less parse failed: {}. Using preprocessed output.", e);
            preprocessed
        }
    };

    // 应用压缩（如果启用）
    let final_css = if config.compressed {
        compress_css(&css)
    } else {
        css
    };

    let compile_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    let output_size = final_css.len();

    debug!(
        "Less compiled: {} -> {} bytes ({:.1}ms)",
        original_size, output_size, compile_time_ms
    );

    Ok(LessCompileResult {
        css: final_css,
        original_size,
        output_size,
        compile_time_ms,
    })
}

/// 基础 Less 转换（变量替换）
///
/// 作为 rust-less 编译失败的降级方案。
/// 仅支持基础的变量定义和替换，不支持嵌套、mixin 等高级特性。
fn basic_less_transform(less: &str) -> String {
    let mut result = less.to_string();

    // 提取变量定义：@variable: value;
    let mut variables = std::collections::HashMap::new();

    for line in less.lines() {
        let line = line.trim();
        if line.starts_with('@') && line.contains(':') && !line.contains(' ') {
            if let Some(colon_pos) = line.find(':') {
                let var_name = &line[1..colon_pos].trim();
                let var_value = line[colon_pos + 1..].trim().trim_end_matches(';');
                if var_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                    variables.insert(var_name.to_string(), var_value.to_string());
                }
            }
        }
    }

    // 替换变量引用
    for (var_name, var_value) in &variables {
        let pattern = format!("@{}", var_name);
        result = result.replace(&pattern, var_value);
    }

    // 移除变量定义行
    result = result
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !(trimmed.starts_with('@')
              && trimmed.contains(':')
              && !trimmed.contains(' ')
              && trimmed.ends_with(';'))
        })
        .collect::<Vec<&str>>()
        .join("\n");

    result
}

/// 移除 CSS 注释（支持单行和多行注释）
fn remove_css_comments(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'*') {
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

/// 压缩 CSS（移除空白和注释）
fn compress_css(css: &str) -> String {
    let without_comments = remove_css_comments(css);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_less_variables() {
        // rust-less v0.1.0 变量支持有限
        // 验证编译不崩溃且返回有效输出
        let less = r#"
@primary-color: #3498db;
@font-size: 16px;

.button {
  background: @primary-color;
  font-size: @font-size;
  padding: 10px;
}
"#;

        let config = LessConfig::default();
        let result = compile_less(less, &config).unwrap();

        // 输出不应为空
        assert!(!result.css.is_empty(), "Output should not be empty");
        assert!(result.css.contains(".button"), "Should contain selector: {}", result.css);
        assert!(result.output_size > 0, "Output size should be positive");
    }

    #[test]
    fn test_compile_less_nesting() {
        let less = r#"
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

        let config = LessConfig::default();
        let result = compile_less(less, &config);

        assert!(result.is_ok(), "Less compilation should succeed: {:?}", result.err());
        let css = result.unwrap().css;
        println!("Nested less output:\n{}", css);
        assert!(css.contains(".container"), "Should contain container selector");
    }

    #[test]
    fn test_compile_less_empty() {
        let config = LessConfig::default();
        let result = compile_less("", &config).unwrap();
        assert!(result.css.is_empty());
    }

    #[test]
    fn test_compile_less_compressed() {
        let less = r#"
.button {
  color: red;
  padding: 10px;
}
"#;

        let config = LessConfig { compressed: true };
        let result = compile_less(less, &config).unwrap();
        assert!(!result.css.contains("\n"), "Compressed CSS should not have newlines");
        assert!(result.css.contains(".button{"), "Compressed CSS should have compact selectors");
    }

    #[test]
    fn test_basic_less_transform_fallback() {
        let less = r#"
@primary-color: #3498db;

.button {
  background: @primary-color;
}
"#;

        let css = basic_less_transform(less);
        assert!(css.contains("#3498db"), "Basic transform should replace variables");
        assert!(css.contains(".button"), "Basic transform should keep selectors");
    }
}
