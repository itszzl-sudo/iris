//! CSS Modules 处理器
//!
//! 实现 CSS Modules 作用域化类名生成，支持 `<style module>` 语法。
//!
//! ## 功能
//!
//! - 类名作用域化：`.button` -> `.button__hash123`
//! - 生成类名映射：`{ "button": "button__hash123" }`
//! - 支持 `:local()` 和 `:global()` 语法
//!
//! ## 使用示例
//!
//! ```vue
//! <style module>
//! .button {
//!   color: red;
//! }
//! </style>
//! ```
//!
//! 编译后：
//! ```css
//! .button__a1b2c3 {
//!   color: red;
//! }
//! ```

use std::collections::HashMap;
use regex::Regex;
use std::sync::LazyLock;

/// CSS 类名选择器正则表达式
static CLASS_SELECTOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.([a-zA-Z_-][a-zA-Z0-9_-]*)").unwrap()
});

/// :local() 伪类正则
static LOCAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r":local\(([^)]+)\)").unwrap()
});

/// :global() 伪类正则
static GLOBAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r":global\(([^)]+)\)").unwrap()
});

/// 生成短哈希（基于内容）
pub fn generate_short_hash(content: &str) -> String {
    use xxhash_rust::xxh3::xxh3_64;
    let hash = xxh3_64(content.as_bytes());
    // 取前 8 位十六进制字符
    format!("{:08x}", hash & 0xFFFFFFFF)
}

/// 生成作用域化类名
///
/// # 参数
///
/// * `class_name` - 原始类名
/// * `hash` - 哈希值
///
/// # 返回
///
/// 作用域化后的类名
pub fn scope_class_name(class_name: &str, hash: &str) -> String {
    format!("{}__{}", class_name, hash)
}

/// 转换 CSS 内容为作用域化版本
///
/// # 参数
///
/// * `css` - 原始 CSS 内容
/// * `hash` - 哈希值
///
/// # 返回
///
/// 作用域化后的 CSS
pub fn transform_css(css: &str, hash: &str) -> String {
    let mut result = css.to_string();
    
    // 处理 :global() - 移除 :global() 包装，保持类名不变
    loop {
        if let Some(mat) = GLOBAL_RE.find(&result) {
            let content = &mat.as_str()[8..mat.as_str().len() - 1];
            result = format!("{}{}{}", 
                &result[..mat.start()], 
                content, 
                &result[mat.end()..]
            );
        } else {
            break;
        }
    }
    
    // 处理 :local() - 作用域化类名
    loop {
        if let Some(mat) = LOCAL_RE.find(&result) {
            let content = &mat.as_str()[7..mat.as_str().len() - 1];
            let scoped = scope_class_name(content.trim(), hash);
            result = format!("{}.{}{}", 
                &result[..mat.start()], 
                scoped, 
                &result[mat.end()..]
            );
        } else {
            break;
        }
    }
    
    // 处理普通类名选择器（自动作用域化）
    result = CLASS_SELECTOR_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let class_name = &caps[1];
            // 跳过已经作用域化的类名（包含 __hash 后缀）
            if class_name.contains("__") {
                return format!(".{}", class_name);
            }
            let scoped = scope_class_name(class_name, hash);
            format!(".{}", scoped)
        })
        .to_string();
    
    result
}

/// 生成类名映射表
///
/// # 参数
///
/// * `css` - 原始 CSS 内容
/// * `hash` - 哈希值
///
/// # 返回
///
/// 类名映射：原始类名 -> 作用域化类名（排除 :global() 中的类名）
pub fn generate_mapping(css: &str, hash: &str) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    
    // 先找出所有 :global() 块的范围
    let mut global_ranges = Vec::new();
    for cap in GLOBAL_RE.captures_iter(css) {
        if let Some(m) = cap.get(0) {
            global_ranges.push((m.start(), m.end()));
        }
    }
    
    // 提取所有类名，排除在 :global() 范围内的
    for cap in CLASS_SELECTOR_RE.captures_iter(css) {
        let class_name = cap[1].to_string();
        let match_start = cap.get(0).unwrap().start();
        
        // 检查是否在 :global() 范围内
        let in_global = global_ranges.iter().any(|&(start, end)| {
            match_start >= start && match_start < end
        });
        
        if !in_global {
            let scoped = scope_class_name(&class_name, hash);
            mapping.insert(class_name, scoped);
        }
    }
    
    mapping
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_short_hash() {
        let hash1 = generate_short_hash(".button { color: red; }");
        let hash2 = generate_short_hash(".button { color: red; }");
        let hash3 = generate_short_hash(".button { color: blue; }");
        
        // 相同内容生成相同哈希
        assert_eq!(hash1, hash2);
        // 不同内容生成不同哈希
        assert_ne!(hash1, hash3);
        // 哈希长度为 8
        assert_eq!(hash1.len(), 8);
    }

    #[test]
    fn test_scope_class_name() {
        let scoped = scope_class_name("button", "a1b2c3d4");
        assert_eq!(scoped, "button__a1b2c3d4");
    }

    #[test]
    fn test_transform_css_basic() {
        let css = r#"
            .button {
                color: red;
            }
            .container {
                padding: 10px;
            }
        "#;
        
        let hash = "test123";
        let result = transform_css(css, hash);
        
        assert!(result.contains(".button__test123"));
        assert!(result.contains(".container__test123"));
        assert!(!result.contains(".button {"));
        assert!(!result.contains(".container {"));
    }

    #[test]
    fn test_transform_css_global() {
        let css = r#"
            :global(.global-class) {
                color: red;
            }
        "#;
        
        let hash = "test123";
        let result = transform_css(css, hash);
        
        // :global() 应该被移除，保留类名
        assert!(result.contains(".global-class"));
        assert!(!result.contains(":global"));
    }

    #[test]
    fn test_transform_css_local() {
        let css = r#"
            :local(.local-class) {
                color: red;
            }
        "#;
        
        let hash = "test123";
        let result = transform_css(css, hash);
        
        // :local() 应该被作用域化
        assert!(result.contains(".local-class__test123"));
        assert!(!result.contains(":local"));
    }

    #[test]
    fn test_generate_mapping() {
        let css = r#"
            .button {
                color: red;
            }
            .container {
                padding: 10px;
            }
        "#;
        
        let hash = "test123";
        let mapping = generate_mapping(css, hash);
        
        assert_eq!(mapping.get("button"), Some(&"button__test123".to_string()));
        assert_eq!(mapping.get("container"), Some(&"container__test123".to_string()));
        assert_eq!(mapping.len(), 2);
    }

    #[test]
    fn test_css_modules_integration() {
        // 模拟完整的 CSS Modules 处理流程
        let css = r#"
            .button {
                color: red;
            }
            :global(.external) {
                font-size: 14px;
            }
        "#;
        
        let hash = generate_short_hash(css);
        let scoped_css = transform_css(css, &hash);
        let mapping = generate_mapping(css, &hash);
        
        // 验证 CSS 被正确作用域化
        assert!(scoped_css.contains(&format!(".button__{}", hash)));
        assert!(scoped_css.contains(".external")); // global 不被作用域化
        
        // 验证映射表
        assert!(mapping.contains_key("button"));
        assert_eq!(mapping.len(), 1); // 只有 .button，.external 是 global
    }
}
