//! Scoped CSS 处理器
//!
//! 实现 Vue 的 `<style scoped>` 功能，为每个组件生成唯一属性并添加到选择器。
//!
//! ## 功能
//!
//! - 为选择器添加唯一数据属性：`.button` -> `.button[data-v-xxxxx]`
//! - 处理组合选择器：`.button.active` -> `.button[data-v-xxxxx].active[data-v-xxxxx]`
//! - 处理伪类和伪元素：`:hover` -> `:hover`（保持不变）
//! - 处理深层选择器：`::v-deep` 或 `/deep/` 语法
//!
//! ## 使用示例
//!
//! ```vue
//! <style scoped>
//! .button {
//!   color: red;
//! }
//! </style>
//! ```
//!
//! 编译后：
//! ```css
//! .button[data-v-a1b2c3d4] {
//!   color: red;
//! }
//! ```

use regex::Regex;
use std::sync::LazyLock;

/// CSS 选择器块匹配（匹配 { 之前的部分）
static SELECTOR_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([^{}]+)\s*\{").unwrap()
});

/// 简单类名、ID、元素选择器
/// 匹配：.class, #id, element, .class:hover 等
static SIMPLE_SELECTOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([.#]?[a-zA-Z_-][a-zA-Z0-9_-]*)").unwrap()
});

/// ::v-deep 或 /deep/ 深层选择器
static DEEP_SELECTOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(::v-deep|/deep/|:deep\()").unwrap()
});

/// 生成唯一的作用域 ID
///
/// # 参数
///
/// * `component_name` - 组件名称
/// * `content_hash` - 内容哈希
///
/// # 返回
///
/// 唯一的作用域 ID，例如：`data-v-a1b2c3d4`
pub fn generate_scope_id(component_name: &str, content_hash: &str) -> String {
    use xxhash_rust::xxh3::xxh3_64;
    let hash = xxh3_64(format!("{}{}", component_name, content_hash).as_bytes());
    format!("data-v-{:08x}", hash & 0xFFFFFFFF)
}

/// 转换 CSS 为作用域版本
///
/// # 参数
///
/// * `css` - 原始 CSS 内容
/// * `scope_id` - 作用域 ID（例如：`data-v-a1b2c3d4`）
///
/// # 返回
///
/// 作用域化后的 CSS
pub fn transform_css_scoped(css: &str, scope_id: &str) -> String {
    // 处理 ::v-deep 或 /deep/ - 这些选择器不应该被作用域化
    // 暂时标记它们，处理完其他选择器后再恢复
    let mut deep_placeholders = Vec::new();
    let mut placeholder_counter = 0;
    let mut result = css.to_string();

    while let Some(mat) = DEEP_SELECTOR_RE.find(&result) {
        let placeholder = format!("__DEEP_PLACEHOLDER_{:x}__", placeholder_counter);
        placeholder_counter += 1;

        // 找到对应的 } 来确定整个选择器块
        if let Some(end_brace) = result[mat.start()..].find('}') {
            let full_block = &result[mat.start()..mat.start() + end_brace + 1];
            deep_placeholders.push((placeholder.clone(), full_block.to_string()));
            result = format!(
                "{}{}{}",
                &result[..mat.start()],
                placeholder,
                &result[mat.start() + end_brace + 1..]
            );
        } else {
            break;
        }
    }

    // 使用累积方式处理选择器块，避免无限循环
    let mut final_result = String::new();
    let mut remaining = result;

    while let Some(mat) = SELECTOR_BLOCK_RE.find(&remaining) {
        // 添加匹配前的部分（包括换行和空白）
        final_result.push_str(&remaining[..mat.start()]);
        
        let selector_text = mat.as_str().trim_end_matches('{').trim();
        let scoped_selector = scope_selector(selector_text, scope_id);
        
        // 添加作用域化的选择器
        final_result.push_str(&format!("{} {{", scoped_selector));
        
        // 找到对应的 } 并添加内容
        if let Some(end_brace) = remaining[mat.end()..].find('}') {
            // 添加选择器块的内容（包括 }）
            final_result.push_str(&remaining[mat.end()..mat.end() + end_brace + 1]);
            // 更新 remaining，跳过已处理的部分
            remaining = remaining[mat.end() + end_brace + 1..].to_string();
        } else {
            // 如果没有找到 }，将剩余部分全部添加
            final_result.push_str(&remaining[mat.end()..]);
            remaining = String::new();
            break;
        }
    }

    // 添加剩余未处理的部分
    final_result.push_str(&remaining);

    // 恢复 ::v-deep 选择器
    for (placeholder, original) in deep_placeholders {
        final_result = final_result.replace(&placeholder, &original);
    }

    final_result
}

/// 作用域化单个选择器
///
/// # 参数
///
/// * `selector` - 原始选择器（例如：`.button`, `.button.active`, `div > p`）
/// * `scope_id` - 作用域 ID
///
/// # 返回
///
/// 作用域化后的选择器
fn scope_selector(selector: &str, scope_id: &str) -> String {
    // 处理逗号分隔的选择器组
    let parts: Vec<&str> = selector.split(',').collect();
    let scoped_parts: Vec<String> = parts
        .iter()
        .map(|part| scope_single_selector(part.trim(), scope_id))
        .collect();

    scoped_parts.join(", ")
}

/// 作用域化单个选择器（不含逗号）
fn scope_single_selector(selector: &str, scope_id: &str) -> String {
    // 特殊处理：根选择器（html, body 等）不作用域化
    if selector.trim().starts_with("html") || selector.trim().starts_with("body") {
        return selector.to_string();
    }

    // 使用状态机方式处理，正确处理伪类和伪元素
    let mut result = String::new();
    let mut current_simple = String::new();
    let mut chars = selector.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            ':' => {
                // 如果当前有简单选择器，先添加它和 scope_id
                if !current_simple.is_empty() {
                    result.push_str(&current_simple);
                    result.push_str(&format!("[{}]", scope_id));
                    current_simple.clear();
                }
                
                // 添加伪类/伪元素标记
                result.push(':');
                
                // 检查是否是伪元素 (::)
                if chars.peek() == Some(&':') {
                    result.push(chars.next().unwrap());
                }
                
                // 添加伪类/伪元素名称和参数
                loop {
                    match chars.next() {
                        Some(c) => {
                            result.push(c);
                            if c == '(' {
                                // 处理括号内的内容（如 :not(.class)）
                                let mut depth = 1;
                                while depth > 0 {
                                    match chars.next() {
                                        Some('(') => {
                                            result.push('(');
                                            depth += 1;
                                        }
                                        Some(')') => {
                                            result.push(')');
                                            depth -= 1;
                                        }
                                        Some(c) => result.push(c),
                                        None => break,
                                    }
                                }
                            }
                            // 如果遇到组合器或结束，退出伪类处理循环
                            let last = result.chars().last().unwrap_or(' ');
                            if !['(', ')', '-', '_'].contains(&last) && !last.is_alphanumeric() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
            '.' | '#' => {
                // 新的类名或 ID 开始，先处理之前的简单选择器
                if !current_simple.is_empty() {
                    result.push_str(&current_simple);
                    result.push_str(&format!("[{}]", scope_id));
                    current_simple.clear();
                }
                current_simple.push(ch);
            }
            ' ' | '>' | '+' | '~' | ',' => {
                // 组合器或分隔符
                if !current_simple.is_empty() {
                    result.push_str(&current_simple);
                    result.push_str(&format!("[{}]", scope_id));
                    current_simple.clear();
                }
                result.push(ch);
            }
            _ => {
                current_simple.push(ch);
            }
        }
    }
    
    // 处理最后一个简单选择器
    if !current_simple.is_empty() {
        result.push_str(&current_simple);
        result.push_str(&format!("[{}]", scope_id));
    }
    
    if result.is_empty() {
        selector.to_string()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_scope_id() {
        let scope_id = generate_scope_id("MyComponent", "hash123");
        assert!(scope_id.starts_with("data-v-"));
        assert_eq!(scope_id.len(), 15); // data-v- + 8 位十六进制

        // 相同输入生成相同 ID
        let scope_id2 = generate_scope_id("MyComponent", "hash123");
        assert_eq!(scope_id, scope_id2);

        // 不同输入生成不同 ID
        let scope_id3 = generate_scope_id("OtherComponent", "hash123");
        assert_ne!(scope_id, scope_id3);
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

        let scope_id = "data-v-test123";
        let result = transform_css_scoped(css, scope_id);

        assert!(result.contains(".button[data-v-test123]"));
        assert!(result.contains(".container[data-v-test123]"));
    }

    #[test]
    fn test_transform_css_combined_selectors() {
        let css = r#"
            .button.active {
                color: blue;
            }
            div > p {
                margin: 0;
            }
        "#;

        let scope_id = "data-v-test123";
        let result = transform_css_scoped(css, scope_id);

        // 组合选择器的每个部分都应该被作用域化
        assert!(result.contains(".button[data-v-test123].active[data-v-test123]"));
        assert!(result.contains("div[data-v-test123] > p[data-v-test123]"));
    }

    #[test]
    fn test_transform_css_with_pseudo_classes() {
        let css = r#"
            .button:hover {
                background: blue;
            }
            .link::before {
                content: "";
            }
        "#;

        let scope_id = "data-v-test123";
        let result = transform_css_scoped(css, scope_id);

        // 伪类应该保持不变，但基础选择器应该被作用域化
        assert!(result.contains(".button[data-v-test123]:hover"));
        assert!(result.contains(".link[data-v-test123]::before"));
    }

    #[test]
    fn test_transform_css_grouped_selectors() {
        let css = r#"
            .button, .link, .tag {
                color: red;
            }
        "#;

        let scope_id = "data-v-test123";
        let result = transform_css_scoped(css, scope_id);

        // 逗号分隔的选择器组都应该被作用域化
        assert!(result.contains(".button[data-v-test123], .link[data-v-test123], .tag[data-v-test123]"));
    }

    #[test]
    fn test_transform_css_with_deep_selector() {
        let css = r#"
            ::v-deep .child {
                color: red;
            }
            /deep/ .other {
                margin: 0;
            }
        "#;

        let scope_id = "data-v-test123";
        let result = transform_css_scoped(css, scope_id);

        // ::v-deep 和 /deep/ 选择器不应该被作用域化
        assert!(result.contains("::v-deep .child"));
        assert!(result.contains("/deep/ .other"));
    }

    #[test]
    fn test_transform_css_root_selectors() {
        let css = r#"
            html {
                font-size: 16px;
            }
            body {
                margin: 0;
            }
        "#;

        let scope_id = "data-v-test123";
        let result = transform_css_scoped(css, scope_id);

        // html 和 body 不应该被作用域化
        assert!(result.contains("html {"));
        assert!(result.contains("body {"));
        assert!(!result.contains("html[data-v-"));
        assert!(!result.contains("body[data-v-"));
    }

    #[test]
    fn test_scope_selector_basic() {
        let result = scope_selector(".button", "data-v-abc");
        assert_eq!(result, ".button[data-v-abc]");
    }

    #[test]
    fn test_scope_selector_combined() {
        let result = scope_selector(".button.active", "data-v-abc");
        assert_eq!(result, ".button[data-v-abc].active[data-v-abc]");
    }

    #[test]
    fn test_scope_selector_with_pseudo() {
        let result = scope_selector(".button:hover", "data-v-abc");
        assert_eq!(result, ".button[data-v-abc]:hover");
    }

    #[test]
    fn test_scope_selector_grouped() {
        let result = scope_selector(".button, .link", "data-v-abc");
        assert_eq!(result, ".button[data-v-abc], .link[data-v-abc]");
    }
}
