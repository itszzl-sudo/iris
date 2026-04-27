//! CSS 解析器
//!
//! 基于 cssparser 实现 CSS 样式表解析。

use cssparser::{Parser, ParserInput};

/// CSS 选择器类型
#[derive(Debug, Clone, PartialEq)]
pub enum SelectorType {
    /// 标签选择器: div, p, span
    Tag(String),
    /// ID 选择器: #id
    Id(String),
    /// Class 选择器: .class
    Class(String),
    /// 属性选择器: [attr=value]
    Attribute { name: String, value: Option<String> },
    /// 通配符: *
    Universal,
    /// 复合选择器: div.class#id
    Compound(Vec<SelectorType>),
    /// 后代选择器: div p
    Descendant(Box<SelectorType>, Box<SelectorType>),
    /// 子元素选择器: div > p
    Child(Box<SelectorType>, Box<SelectorType>),
}

/// CSS 选择器
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    /// 选择器文本 (如 ".class", "#id", "div")
    pub text: String,
    /// 选择器类型（增强版）
    pub selector_type: SelectorType,
}

impl Selector {
    /// 创建新的选择器（自动解析类型）
    pub fn new(text: &str) -> Self {
        let selector_type = parse_selector_type(text);
        Self {
            text: text.to_string(),
            selector_type,
        }
    }

    /// 创建指定类型的选择器
    pub fn with_type(text: &str, selector_type: SelectorType) -> Self {
        Self {
            text: text.to_string(),
            selector_type,
        }
    }

    /// 判断是否为 ID 选择器
    pub fn is_id(&self) -> bool {
        matches!(self.selector_type, SelectorType::Id(_))
    }

    /// 判断是否为 Class 选择器
    pub fn is_class(&self) -> bool {
        matches!(self.selector_type, SelectorType::Class(_))
    }

    /// 判断是否为标签选择器
    pub fn is_tag(&self) -> bool {
        matches!(self.selector_type, SelectorType::Tag(_))
    }

    /// 判断是否为复合选择器
    pub fn is_compound(&self) -> bool {
        matches!(self.selector_type, SelectorType::Compound(_))
    }
}

/// 解析选择器类型（增强版）
fn parse_selector_type(text: &str) -> SelectorType {
    let text = text.trim();
    
    // 通配符
    if text == "*" {
        return SelectorType::Universal;
    }
    
    // 属性选择器 [attr] 或 [attr=value]
    if text.starts_with('[') && text.ends_with(']') {
        let content = &text[1..text.len() - 1];
        if let Some(eq_pos) = content.find('=') {
            let name = content[..eq_pos].trim().to_string();
            let value = Some(content[eq_pos + 1..].trim().trim_matches(|c| c == '"' || c == '\'').to_string());
            return SelectorType::Attribute { name, value };
        } else {
            return SelectorType::Attribute {
                name: content.trim().to_string(),
                value: None,
            };
        }
    }
    
    // 复合选择器（包含 . 或 # 的组合）
    if (text.contains('.') || text.contains('#')) && !text.starts_with('.') && !text.starts_with('#') {
        // 例如: div.class, div#id, div.class1.class2
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '.' || ch == '#' {
                if !current.is_empty() {
                    // 保存之前的部分
                    if parts.is_empty() && !current.starts_with('.') && !current.starts_with('#') {
                        parts.push(SelectorType::Tag(current.clone()));
                    } else if current.starts_with('.') {
                        parts.push(SelectorType::Class(current[1..].to_string()));
                    } else if current.starts_with('#') {
                        parts.push(SelectorType::Id(current[1..].to_string()));
                    }
                    current.clear();
                }
                current.push(ch);
            } else {
                current.push(ch);
            }
        }
        
        // 处理最后一部分
        if !current.is_empty() {
            if current.starts_with('.') {
                parts.push(SelectorType::Class(current[1..].to_string()));
            } else if current.starts_with('#') {
                parts.push(SelectorType::Id(current[1..].to_string()));
            } else {
                parts.push(SelectorType::Class(current)); // 默认当作 class
            }
        }
        
        if parts.len() > 1 {
            return SelectorType::Compound(parts);
        }
    }
    
    // 简单选择器
    if text.starts_with('#') {
        SelectorType::Id(text[1..].to_string())
    } else if text.starts_with('.') {
        SelectorType::Class(text[1..].to_string())
    } else {
        SelectorType::Tag(text.to_string())
    }
}

/// CSS 声明 (属性: 值)
#[derive(Debug, Clone)]
pub struct Declaration {
    /// 属性名 (如 "color", "font-size")
    pub property: String,
    /// 属性值 (如 "red", "16px")
    pub value: String,
}

/// CSS 规则 (选择器 { 声明块 })
#[derive(Debug, Clone)]
pub struct CSSRule {
    /// 选择器
    pub selector: Selector,
    /// 声明列表
    pub declarations: Vec<Declaration>,
}

impl CSSRule {
    /// 创建新的 CSS 规则
    pub fn new(selector: Selector, declarations: Vec<Declaration>) -> Self {
        Self {
            selector,
            declarations,
        }
    }
}

/// CSS 样式表
#[derive(Debug, Clone)]
pub struct Stylesheet {
    /// CSS 规则列表
    pub rules: Vec<CSSRule>,
}

impl Stylesheet {
    /// 创建空的样式表
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: CSSRule) {
        self.rules.push(rule);
    }
}

/// 解析 CSS 字符串，生成样式表
///
/// # 示例
///
/// ```rust
/// use iris_layout::css::parse_stylesheet;
///
/// let css = r#"
///     .container {
///         padding: 20px;
///         background-color: white;
///     }
///     
///     #title {
///         font-size: 24px;
///         color: blue;
///     }
/// "#;
///
/// let stylesheet = parse_stylesheet(css);
/// assert!(!stylesheet.rules.is_empty());
/// ```
pub fn parse_stylesheet(css: &str) -> Stylesheet {
    let mut input = ParserInput::new(css);
    let _parser = Parser::new(&mut input);
    
    let mut stylesheet = Stylesheet::new();
    
    // 简化实现：手动解析 CSS
    // 实际应该使用 cssparser 的完整解析能力
    parse_css_manual(css, &mut stylesheet);
    
    stylesheet
}

/// 手动解析 CSS (简化实现)
fn parse_css_manual(css: &str, stylesheet: &mut Stylesheet) {
    // 移除注释
    let css = remove_comments(css);
    
    // 分割规则
    let rules = split_rules(&css);
    
    for rule_text in rules {
        if let Some(rule) = parse_single_rule(rule_text.trim()) {
            stylesheet.add_rule(rule);
        }
    }
}

/// 移除 CSS 注释
fn remove_comments(css: &str) -> String {
    let mut result = String::new();
    let mut chars = css.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '/' {
            if let Some(&'*') = chars.peek() {
                chars.next(); // consume '*'
                // Skip until */
                while let Some(ch) = chars.next() {
                    if ch == '*' {
                        if let Some(&'/') = chars.peek() {
                            chars.next(); // consume '/'
                            break;
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// 分割 CSS 规则
fn split_rules(css: &str) -> Vec<&str> {
    let mut rules = Vec::new();
    let mut start = 0;
    let mut brace_count = 0;
    
    for (i, ch) in css.char_indices() {
        match ch {
            '{' => brace_count += 1,
            '}' => {
                brace_count -= 1;
                if brace_count == 0 {
                    rules.push(&css[start..=i]);
                    start = i + 1;
                }
            }
            _ => {}
        }
    }
    
    rules
}

/// 解析单个 CSS 规则
fn parse_single_rule(rule_text: &str) -> Option<CSSRule> {
    // 查找 { 的位置
    let brace_pos = rule_text.find('{')?;
    
    let selector_text = rule_text[..brace_pos].trim();
    let declarations_text = &rule_text[brace_pos + 1..rule_text.len() - 1]; // remove { }
    
    if selector_text.is_empty() {
        return None;
    }
    
    let selector = Selector::new(selector_text);
    let declarations = parse_declarations(declarations_text);
    
    Some(CSSRule::new(selector, declarations))
}

/// 解析声明块
fn parse_declarations(text: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    
    for decl_text in text.split(';') {
        let decl_text = decl_text.trim();
        if decl_text.is_empty() {
            continue;
        }
        
        if let Some(colon_pos) = decl_text.find(':') {
            let property = decl_text[..colon_pos].trim().to_string();
            let value = decl_text[colon_pos + 1..].trim().to_string();
            
            declarations.push(Declaration { property, value });
        }
    }
    
    declarations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_css() {
        let css = r#"
            .container {
                padding: 20px;
                background-color: white;
            }
        "#;
        
        let stylesheet = parse_stylesheet(css);
        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].selector.text, ".container");
        assert_eq!(stylesheet.rules[0].declarations.len(), 2);
    }

    #[test]
    fn test_parse_multiple_rules() {
        let css = r#"
            .class1 { color: red; }
            #id1 { font-size: 16px; }
            div { margin: 0; }
        "#;
        
        let stylesheet = parse_stylesheet(css);
        assert_eq!(stylesheet.rules.len(), 3);
    }

    #[test]
    fn test_selector_types() {
        let class_sel = Selector::new(".container");
        let id_sel = Selector::new("#main");
        let tag_sel = Selector::new("div");
        
        assert!(class_sel.is_class());
        assert!(id_sel.is_id());
        assert!(tag_sel.is_tag());
    }

    #[test]
    fn test_parse_with_comments() {
        let css = r#"
            /* This is a comment */
            .container {
                padding: 20px; /* inline comment */
            }
        "#;
        
        let stylesheet = parse_stylesheet(css);
        assert_eq!(stylesheet.rules.len(), 1);
    }

    #[test]
    fn test_attribute_selector() {
        let sel1 = Selector::new("[data-type]");
        let sel2 = Selector::new("[data-type=button]");
        let sel3 = Selector::new("[href=\"https://example.com\"]");
        
        assert!(sel1.is_compound() || matches!(sel1.selector_type, crate::css::SelectorType::Attribute { .. }));
        assert!(sel2.is_compound() || matches!(sel2.selector_type, crate::css::SelectorType::Attribute { .. }));
        assert!(sel3.is_compound() || matches!(sel3.selector_type, crate::css::SelectorType::Attribute { .. }));
    }

    #[test]
    fn test_compound_selector() {
        let sel = Selector::new("div.container");
        assert!(sel.is_compound());
        
        let sel2 = Selector::new("div#main.container");
        assert!(sel2.is_compound());
    }

    #[test]
    fn test_universal_selector() {
        let sel = Selector::new("*");
        assert!(matches!(sel.selector_type, crate::css::SelectorType::Universal));
    }

    #[test]
    fn test_selector_type_parsing() {
        use crate::css::SelectorType;
        
        let id_sel = Selector::new("#main");
        assert!(matches!(id_sel.selector_type, SelectorType::Id(s) if s == "main"));
        
        let class_sel = Selector::new(".container");
        assert!(matches!(class_sel.selector_type, SelectorType::Class(s) if s == "container"));
        
        let tag_sel = Selector::new("div");
        assert!(matches!(tag_sel.selector_type, SelectorType::Tag(s) if s == "div"));
    }
}
