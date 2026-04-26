//! 样式计算
//!
//! 实现 CSS 选择器匹配、样式层叠和继承。

use crate::css::{parse_stylesheet, Declaration, Selector, Stylesheet};
use crate::dom::DOMNode;
use std::collections::HashMap;

/// 计算后的样式值
///
/// 所有 CSS 值都已经被解析和标准化。
#[derive(Debug, Clone)]
pub struct ComputedStyles {
    /// 样式属性映射
    properties: HashMap<String, String>,
}

impl ComputedStyles {
    /// 创建空的计算样式
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    /// 设置样式属性
    pub fn set(&mut self, property: &str, value: &str) {
        self.properties
            .insert(property.to_string(), value.to_string());
    }

    /// 获取样式属性
    pub fn get(&self, property: &str) -> Option<&String> {
        self.properties.get(property)
    }

    /// 获取所有属性
    pub fn properties(&self) -> &HashMap<String, String> {
        &self.properties
    }

    /// 合并另一个计算样式 (用于层叠)
    pub fn merge(&mut self, other: &ComputedStyles) {
        for (key, value) in &other.properties {
            // 只在当前没有该属性时才覆盖 (低优先级)
            if !self.properties.contains_key(key) {
                self.properties.insert(key.clone(), value.clone());
            }
        }
    }
}

/// 匹配 DOM 节点和 CSS 规则，计算样式
///
/// # 示例
///
/// ```rust
/// use iris_layout::dom::DOMNode;
/// use iris_layout::css::parse_stylesheet;
/// use iris_layout::style::compute_styles;
///
/// let mut div = DOMNode::new_element("div");
/// div.set_attribute("class", "container");
///
/// let css = ".container { padding: 20px; }";
/// let stylesheet = parse_stylesheet(css);
///
/// let styles = compute_styles(&div, &stylesheet, None);
/// assert!(styles.get("padding").is_some());
/// ```
pub fn compute_styles(
    node: &DOMNode,
    stylesheet: &Stylesheet,
    parent_styles: Option<&ComputedStyles>,
) -> ComputedStyles {
    let mut computed = ComputedStyles::new();

    // 1. 继承父节点样式
    if let Some(parent) = parent_styles {
        computed.merge(parent);
    }

    // 2. 匹配并应用 CSS 规则
    let mut matching_rules = Vec::new();
    for rule in &stylesheet.rules {
        if matches_selector(node, &rule.selector) {
            matching_rules.push(rule);
        }
    }

    // 3. 按特异性排序 (简化实现：ID > Class > Tag)
    matching_rules.sort_by_key(|rule| selector_specificity(&rule.selector));

    // 4. 应用规则 (从低优先级到高优先级)
    for rule in matching_rules {
        for decl in &rule.declarations {
            computed.set(&decl.property, &decl.value);
        }
    }

    computed
}

/// 判断节点是否匹配选择器
fn matches_selector(node: &DOMNode, selector: &Selector) -> bool {
    if selector.is_id() {
        // ID 选择器: #id
        let id = &selector.text[1..];
        node.id_attr().map(|s| s.as_str()) == Some(id)
    } else if selector.is_class() {
        // Class 选择器: .class
        let class = &selector.text[1..];
        node.class()
            .map(|s| s.split_whitespace().any(|c| c == class))
            .unwrap_or(false)
    } else {
        // 标签选择器: div
        node.tag_name().map(|s| s == selector.text.as_str())
            .unwrap_or(false)
    }
}

/// 计算选择器特异性 (简化实现)
///
/// 返回值: (id_count, class_count, tag_count)
fn selector_specificity(selector: &Selector) -> (u32, u32, u32) {
    if selector.is_id() {
        (1, 0, 0)
    } else if selector.is_class() {
        (0, 1, 0)
    } else {
        (0, 0, 1)
    }
}

/// 为整个 DOM 树计算样式
///
/// 递归处理所有节点，考虑样式继承。
pub fn compute_tree_styles(
    node: &mut DOMNode,
    stylesheet: &Stylesheet,
    parent_styles: Option<&ComputedStyles>,
) -> ComputedStyles {
    // 计算当前节点样式
    let computed = compute_styles(node, stylesheet, parent_styles);

    // 递归处理子节点
    for child in &mut node.children {
        compute_tree_styles(child, stylesheet, Some(&computed));
    }

    computed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_id_selector() {
        let mut node = DOMNode::new_element("div");
        node.set_attribute("id", "main");

        let selector = Selector::new("#main");
        assert!(matches_selector(&node, &selector));

        let selector2 = Selector::new("#other");
        assert!(!matches_selector(&node, &selector2));
    }

    #[test]
    fn test_matches_class_selector() {
        let mut node = DOMNode::new_element("div");
        node.set_attribute("class", "container highlight");

        let selector = Selector::new(".container");
        assert!(matches_selector(&node, &selector));

        let selector2 = Selector::new(".other");
        assert!(!matches_selector(&node, &selector2));
    }

    #[test]
    fn test_matches_tag_selector() {
        let node = DOMNode::new_element("div");

        let selector = Selector::new("div");
        assert!(matches_selector(&node, &selector));

        let selector2 = Selector::new("p");
        assert!(!matches_selector(&node, &selector2));
    }

    #[test]
    fn test_specificity() {
        let id_sel = Selector::new("#main");
        let class_sel = Selector::new(".container");
        let tag_sel = Selector::new("div");

        assert!(selector_specificity(&id_sel) > selector_specificity(&class_sel));
        assert!(selector_specificity(&class_sel) > selector_specificity(&tag_sel));
    }

    #[test]
    fn test_compute_styles() {
        let mut div = DOMNode::new_element("div");
        div.set_attribute("class", "container");

        let css = ".container { padding: 20px; color: red; }";
        let stylesheet = parse_stylesheet(css);

        let computed = compute_styles(&div, &stylesheet, None);

        assert_eq!(computed.get("padding"), Some(&"20px".to_string()));
        assert_eq!(computed.get("color"), Some(&"red".to_string()));
    }

    #[test]
    fn test_style_inheritance() {
        let mut parent = DOMNode::new_element("div");
        let mut child = DOMNode::new_element("p");
        parent.append_child(child);

        // 父节点有可继承的属性
        let css = "div { color: blue; font-size: 16px; }";
        let stylesheet = parse_stylesheet(css);

        let parent_styles = compute_styles(&parent, &stylesheet, None);
        let child_styles = compute_styles(&parent.children[0], &stylesheet, Some(&parent_styles));

        // 子节点应该继承父节点的样式
        assert_eq!(child_styles.get("color"), Some(&"blue".to_string()));
    }
}
