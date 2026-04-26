//! DOM 节点树数据结构
//!
//! 提供轻量级的 DOM 节点表示，支持树形结构和属性管理。

use std::collections::HashMap;

/// DOM 节点类型
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    /// 元素节点 (如 `<div>`, `<p>`)
    Element(String),
    /// 文本节点
    Text(String),
    /// 注释节点
    Comment(String),
}

/// DOM 节点
///
/// 表示 HTML DOM 树中的一个节点，支持元素、文本和注释。
#[derive(Debug, Clone)]
pub struct DOMNode {
    /// 节点唯一标识
    pub id: u64,
    /// 节点类型
    pub node_type: NodeType,
    /// 属性集合
    pub attributes: HashMap<String, String>,
    /// 子节点
    pub children: Vec<DOMNode>,
    /// 父节点 ID (0 表示根节点)
    pub parent_id: u64,
}

impl DOMNode {
    /// 创建新的元素节点
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_layout::dom::{DOMNode, NodeType};
    ///
    /// let div = DOMNode::new_element("div");
    /// assert!(matches!(div.node_type, NodeType::Element(tag) if tag == "div"));
    /// ```
    pub fn new_element(tag: &str) -> Self {
        Self {
            id: Self::generate_id(),
            node_type: NodeType::Element(tag.to_lowercase()),
            attributes: HashMap::new(),
            children: Vec::new(),
            parent_id: 0,
        }
    }

    /// 创建文本节点
    pub fn new_text(text: &str) -> Self {
        Self {
            id: Self::generate_id(),
            node_type: NodeType::Text(text.to_string()),
            attributes: HashMap::new(),
            children: Vec::new(),
            parent_id: 0,
        }
    }

    /// 创建注释节点
    pub fn new_comment(comment: &str) -> Self {
        Self {
            id: Self::generate_id(),
            node_type: NodeType::Comment(comment.to_string()),
            attributes: HashMap::new(),
            children: Vec::new(),
            parent_id: 0,
        }
    }

    /// 设置属性
    pub fn set_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }

    /// 获取属性
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// 获取 class 属性
    pub fn class(&self) -> Option<&String> {
        self.get_attribute("class")
    }

    /// 获取 id 属性
    pub fn id_attr(&self) -> Option<&String> {
        self.get_attribute("id")
    }

    /// 添加子节点
    pub fn append_child(&mut self, mut child: DOMNode) {
        child.parent_id = self.id;
        self.children.push(child);
    }

    /// 获取标签名 (仅对元素节点有效)
    pub fn tag_name(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Element(tag) => Some(tag),
            _ => None,
        }
    }

    /// 获取文本内容 (仅对文本节点有效)
    pub fn text_content(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Text(text) => Some(text),
            _ => None,
        }
    }

    /// 判断是否为元素节点
    pub fn is_element(&self) -> bool {
        matches!(self.node_type, NodeType::Element(_))
    }

    /// 判断是否为文本节点
    pub fn is_text(&self) -> bool {
        matches!(self.node_type, NodeType::Text(_))
    }

    /// 递归收集所有文本节点内容
    pub fn collect_text(&self) -> String {
        match &self.node_type {
            NodeType::Text(text) => text.clone(),
            NodeType::Element(_) => {
                let mut text = String::new();
                for child in &self.children {
                    text.push_str(&child.collect_text());
                }
                text
            }
            NodeType::Comment(_) => String::new(),
        }
    }

    /// 生成唯一 ID (简化实现，实际应使用原子计数器)
    fn generate_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }
}

/// DOM 树
///
/// 管理整个 DOM 节点树，提供查询和操作接口。
pub struct DOMTree {
    /// 根节点
    pub root: DOMNode,
}

impl DOMTree {
    /// 创建新的 DOM 树
    pub fn new(root: DOMNode) -> Self {
        Self { root }
    }

    /// 获取根节点
    pub fn root(&self) -> &DOMNode {
        &self.root
    }

    /// 获取根节点的可变引用
    pub fn root_mut(&mut self) -> &mut DOMNode {
        &mut self.root
    }

    /// 按选择器查找节点 (简化实现，仅支持 #id 和 .class)
    pub fn query_selector(&self, selector: &str) -> Option<&DOMNode> {
        if selector.starts_with('#') {
            // ID 选择器
            let id = &selector[1..];
            self.find_by_id(&self.root, id)
        } else if selector.starts_with('.') {
            // Class 选择器
            let class = &selector[1..];
            self.find_by_class(&self.root, class)
        } else {
            // 标签选择器
            self.find_by_tag(&self.root, selector)
        }
    }

    fn find_by_id<'a>(&'a self, node: &'a DOMNode, id: &str) -> Option<&'a DOMNode> {
        if let Some(node_id) = node.id_attr() {
            if node_id == id {
                return Some(node);
            }
        }
        for child in &node.children {
            if let Some(found) = self.find_by_id(child, id) {
                return Some(found);
            }
        }
        None
    }

    fn find_by_class<'a>(&'a self, node: &'a DOMNode, class: &str) -> Option<&'a DOMNode> {
        if let Some(node_class) = node.class() {
            if node_class.split_whitespace().any(|c| c == class) {
                return Some(node);
            }
        }
        for child in &node.children {
            if let Some(found) = self.find_by_class(child, class) {
                return Some(found);
            }
        }
        None
    }

    fn find_by_tag<'a>(&'a self, node: &'a DOMNode, tag: &str) -> Option<&'a DOMNode> {
        if let Some(node_tag) = node.tag_name() {
            if node_tag == tag {
                return Some(node);
            }
        }
        for child in &node.children {
            if let Some(found) = self.find_by_tag(child, tag) {
                return Some(found);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_element() {
        let div = DOMNode::new_element("div");
        assert!(div.is_element());
        assert_eq!(div.tag_name(), Some("div"));
    }

    #[test]
    fn test_create_text() {
        let text = DOMNode::new_text("Hello");
        assert!(text.is_text());
        assert_eq!(text.text_content(), Some("Hello"));
    }

    #[test]
    fn test_set_attribute() {
        let mut div = DOMNode::new_element("div");
        div.set_attribute("class", "container");
        div.set_attribute("id", "main");
        
        assert_eq!(div.class(), Some(&"container".to_string()));
        assert_eq!(div.id_attr(), Some(&"main".to_string()));
    }

    #[test]
    fn test_append_child() {
        let mut parent = DOMNode::new_element("div");
        let child = DOMNode::new_element("p");
        let parent_id = parent.id;
        
        parent.append_child(child);
        
        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0].parent_id, parent_id);
    }

    #[test]
    fn test_collect_text() {
        let mut div = DOMNode::new_element("div");
        div.append_child(DOMNode::new_text("Hello "));
        
        let mut p = DOMNode::new_element("p");
        p.append_child(DOMNode::new_text("World"));
        div.append_child(p);
        
        assert_eq!(div.collect_text(), "Hello World");
    }

    #[test]
    fn test_query_selector_by_id() {
        let mut root = DOMNode::new_element("div");
        let mut child = DOMNode::new_element("p");
        child.set_attribute("id", "test");
        root.append_child(child);
        
        let tree = DOMTree::new(root);
        let found = tree.query_selector("#test");
        
        assert!(found.is_some());
        assert_eq!(found.unwrap().tag_name(), Some("p"));
    }

    #[test]
    fn test_query_selector_by_class() {
        let mut root = DOMNode::new_element("div");
        let mut child = DOMNode::new_element("p");
        child.set_attribute("class", "highlight");
        root.append_child(child);
        
        let tree = DOMTree::new(root);
        let found = tree.query_selector(".highlight");
        
        assert!(found.is_some());
    }
}
