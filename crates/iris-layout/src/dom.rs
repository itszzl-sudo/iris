//! DOM 节点树数据结构
//!
//! 提供轻量级的 DOM 节点表示，支持树形结构和属性管理。

use std::collections::HashMap;
use crate::style::ComputedStyles;

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

    /// 移除指定的子节点
    ///
    /// # 返回值
    ///
    /// 如果找到并成功移除子节点，返回 `Some(child)`；否则返回 `None`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_layout::dom::DOMNode;
    ///
    /// let mut parent = DOMNode::new_element("div");
    /// let child = DOMNode::new_element("span");
    /// parent.append_child(child);
    ///
    /// let removed = parent.remove_child(0);
    /// assert!(removed.is_some());
    /// assert_eq!(parent.children.len(), 0);
    /// ```
    pub fn remove_child(&mut self, index: usize) -> Option<DOMNode> {
        if index < self.children.len() {
            let mut child = self.children.remove(index);
            child.parent_id = 0; // 清除父节点引用
            Some(child)
        } else {
            None
        }
    }

    /// 在指定子节点之前插入新节点
    ///
    /// # 参数
    ///
    /// - `new_node`: 要插入的新节点
    /// - `reference_index`: 参考子节点的索引，新节点将插入到此节点之前
    ///
    /// # 返回值
    ///
    /// 如果插入成功返回 `true`，如果索引无效返回 `false`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_layout::dom::DOMNode;
    ///
    /// let mut parent = DOMNode::new_element("div");
    /// let child1 = DOMNode::new_element("span");
    /// parent.append_child(child1);
    ///
    /// let child2 = DOMNode::new_element("p");
    /// parent.insert_before(child2, 0); // 在第一个子节点前插入
    ///
    /// assert_eq!(parent.children.len(), 2);
    /// ```
    pub fn insert_before(&mut self, mut new_node: DOMNode, reference_index: usize) -> bool {
        if reference_index <= self.children.len() {
            new_node.parent_id = self.id;
            self.children.insert(reference_index, new_node);
            true
        } else {
            false
        }
    }

    /// 替换指定的子节点
    ///
    /// # 参数
    ///
    /// - `new_node`: 新节点
    /// - `index`: 要替换的子节点索引
    ///
    /// # 返回值
    ///
    /// 如果替换成功返回 `Some(old_node)`，如果索引无效返回 `None`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_layout::dom::DOMNode;
    ///
    /// let mut parent = DOMNode::new_element("div");
    /// let old_child = DOMNode::new_element("span");
    /// parent.append_child(old_child);
    ///
    /// let new_child = DOMNode::new_element("p");
    /// let replaced = parent.replace_child(new_child, 0);
    ///
    /// assert!(replaced.is_some());
    /// ```
    pub fn replace_child(&mut self, mut new_node: DOMNode, index: usize) -> Option<DOMNode> {
        if index < self.children.len() {
            let old_node = self.children.remove(index);
            new_node.parent_id = self.id;
            self.children.insert(index, new_node);
            Some(old_node)
        } else {
            None
        }
    }

    /// 克隆节点
    ///
    /// # 参数
    ///
    /// - `deep`: 如果为 `true`，则递归克隆所有子孙节点；如果为 `false`，仅克隆当前节点
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_layout::dom::DOMNode;
    ///
    /// let parent = DOMNode::new_element("div");
    /// let clone_shallow = parent.clone_node(false);
    /// let clone_deep = parent.clone_node(true);
    /// ```
    pub fn clone_node(&self, deep: bool) -> Self {
        let mut cloned = Self {
            id: Self::generate_id(), // 新 ID
            node_type: self.node_type.clone(),
            attributes: self.attributes.clone(),
            children: Vec::new(),
            parent_id: 0, // 克隆后的节点没有父节点
        };

        if deep {
            for child in &self.children {
                let mut cloned_child = child.clone_node(true);
                cloned_child.parent_id = cloned.id; // 修正父节点引用
                cloned.children.push(cloned_child);
            }
        }

        cloned
    }

    /// 在末尾添加多个子节点（现代 API）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_layout::dom::DOMNode;
    ///
    /// let mut parent = DOMNode::new_element("div");
    /// let child1 = DOMNode::new_element("span");
    /// let child2 = DOMNode::new_element("p");
    ///
    /// parent.append(vec![child1, child2]);
    /// assert_eq!(parent.children.len(), 2);
    /// ```
    pub fn append(&mut self, children: Vec<DOMNode>) {
        for mut child in children {
            child.parent_id = self.id;
            self.children.push(child);
        }
    }

    /// 在开头添加多个子节点（现代 API）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_layout::dom::{DOMNode, NodeType};
    ///
    /// let mut parent = DOMNode::new_element("div");
    /// let existing = DOMNode::new_element("span");
    /// parent.append_child(existing);
    ///
    /// let child1 = DOMNode::new_element("p");
    /// let child2 = DOMNode::new_element("a");
    ///
    /// parent.prepend(vec![child1, child2]);
    /// assert_eq!(parent.children.len(), 3);
    /// assert!(matches!(&parent.children[0].node_type, NodeType::Element(e) if e == "p"));
    /// ```
    pub fn prepend(&mut self, children: Vec<DOMNode>) {
        let mut new_children = Vec::new();
        for mut child in children {
            child.parent_id = self.id;
            new_children.push(child);
        }
        
        // 将现有子节点追加到新子节点后面
        new_children.extend(self.children.drain(..));
        self.children = new_children;
    }

    /// 在当前节点之后插入兄弟节点（需要在父节点中操作）
    ///
    /// # 注意
    ///
    /// 此方法仅当节点有父节点时有效，会在父节点中操作
    /// 
    /// @deprecated 尚未实现，请使用父节点的 insert_before 方法
    #[allow(unused_variables)]
    pub fn insert_after_sibling(&mut self, sibling: DOMNode) -> bool {
        // 这个方法需要在父节点上下文中实现，暂时返回 false
        false
    }

    /// 在当前节点之前插入兄弟节点（需要在父节点中操作）
    ///
    /// # 注意
    ///
    /// 此方法仅当节点有父节点时有效，会在父节点中操作
    /// 
    /// @deprecated 尚未实现，请使用父节点的 insert_before 方法
    #[allow(unused_variables)]
    pub fn insert_before_sibling(&mut self, sibling: DOMNode) -> bool {
        // 这个方法需要在父节点上下文中实现，暂时返回 false
        false
    }

    /// 从父节点中移除自身（现代 API）
    ///
    /// # 注意
    ///
    /// 此方法仅当节点有父节点时有效，需要在父节点中操作
    /// 
    /// @deprecated 尚未实现，请使用父节点的 remove_child 方法
    pub fn remove_self(&mut self) -> bool {
        // 这个方法需要在父节点上下文中实现，暂时返回 false
        false
    }

    /// 检查是否包含指定的后代节点
    ///
    /// # 参数
    ///
    /// - `node_id`: 要检查的节点 ID
    ///
    /// # 返回值
    ///
    /// 如果当前节点包含指定 ID 的后代节点，返回 `true`
    pub fn contains(&self, node_id: u64) -> bool {
        if self.id == node_id {
            return true;
        }
        
        for child in &self.children {
            if child.contains(node_id) {
                return true;
            }
        }
        
        false
    }

    /// 获取子节点数量
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// 检查是否有子节点
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// 获取第一个子节点的可变引用
    pub fn first_child_mut(&mut self) -> Option<&mut DOMNode> {
        self.children.first_mut()
    }

    /// 获取最后一个子节点的可变引用
    pub fn last_child_mut(&mut self) -> Option<&mut DOMNode> {
        self.children.last_mut()
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
    
    /// 获取计算后的样式
    ///
    /// 从 style 属性解析 CSS 并返回 ComputedStyles
    /// 如果节点没有 style 属性，返回空的 ComputedStyles
    pub fn computed_styles(&self) -> Option<ComputedStyles> {
        // 只对元素节点返回样式
        if !self.is_element() {
            return None;
        }
        
        // 获取 style 属性
        let style_attr = self.get_attribute("style")?;
        
        // 解析 CSS 样式字符串
        let mut styles = ComputedStyles::new();
        self.parse_style_attribute(style_attr, &mut styles);
        
        Some(styles)
    }
    
    /// 解析 style 属性字符串
    fn parse_style_attribute(&self, style_str: &str, styles: &mut ComputedStyles) {
        // 按分号分割属性
        for declaration in style_str.split(';') {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }
            
            // 按冒号分割属性名和值
            if let Some(colon_pos) = declaration.find(':') {
                let property = declaration[..colon_pos].trim().to_lowercase();
                let value = declaration[colon_pos + 1..].trim();
                
                if !property.is_empty() && !value.is_empty() {
                    styles.set(&property, value);
                }
            }
        }
    }

    /// 生成唯一 ID (简化实现，实际应使用原子计数器)
    fn generate_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// 在当前节点后插入兄弟节点（需要 DOMTree 上下文）
    ///
    /// # 注意
    ///
    /// 此方法返回操作指令，实际的修改需要通过 DOMTree::insert_after() 来完成。
    ///
    /// # 返回
    ///
    /// 返回 (reference_id, new_node)，其中 reference_id 是当前节点的 ID
    /// 如果当前节点没有父节点（根节点），返回 None
    pub fn after(&self, new_node: DOMNode) -> Option<(u64, DOMNode)> {
        if self.parent_id == 0 {
            None
        } else {
            Some((self.id, new_node))
        }
    }

    /// 在当前节点前插入兄弟节点（需要 DOMTree 上下文）
    ///
    /// # 注意
    ///
    /// 此方法返回操作指令，实际的修改需要通过 DOMTree::insert_before_node() 来完成。
    ///
    /// # 返回
    ///
    /// 返回 (reference_id, new_node)，其中 reference_id 是当前节点的 ID
    /// 如果当前节点没有父节点（根节点），返回 None
    pub fn before(&self, new_node: DOMNode) -> Option<(u64, DOMNode)> {
        if self.parent_id == 0 {
            None
        } else {
            Some((self.id, new_node))
        }
    }

    /// 从父节点中移除当前节点（需要 DOMTree 上下文）
    ///
    /// # 注意
    ///
    /// 此方法返回操作指令，实际的修改需要通过 DOMTree::remove_node() 来完成。
    ///
    /// # 返回
    ///
    /// 返回当前节点的 ID，如果当前节点没有父节点（根节点），返回 None
    pub fn remove(&self) -> Option<u64> {
        if self.parent_id == 0 {
            None
        } else {
            Some(self.id)
        }
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

    #[test]
    fn test_computed_styles_with_style_attribute() {
        // 测试有 style 属性的元素
        let mut div = DOMNode::new_element("div");
        div.set_attribute("style", "width: 100px; height: 200px; color: red;");
        
        let styles = div.computed_styles();
        assert!(styles.is_some());
        
        let styles = styles.unwrap();
        assert_eq!(styles.get("width"), Some(&"100px".to_string()));
        assert_eq!(styles.get("height"), Some(&"200px".to_string()));
        assert_eq!(styles.get("color"), Some(&"red".to_string()));
    }

    #[test]
    fn test_computed_styles_without_style_attribute() {
        // 测试没有 style 属性的元素
        let div = DOMNode::new_element("div");
        
        let styles = div.computed_styles();
        assert!(styles.is_none());
    }

    #[test]
    fn test_computed_styles_text_node() {
        // 测试文本节点
        let text = DOMNode::new_text("Hello");
        
        let styles = text.computed_styles();
        assert!(styles.is_none());
    }

    #[test]
    fn test_computed_styles_complex_css() {
        // 测试复杂的 CSS 样式
        let mut div = DOMNode::new_element("div");
        div.set_attribute("style", "display: flex; flex-wrap: wrap; gap: 10px; min-height: 50px; max-width: 500px;");
        
        let styles = div.computed_styles().unwrap();
        assert_eq!(styles.get("display"), Some(&"flex".to_string()));
        assert_eq!(styles.get("flex-wrap"), Some(&"wrap".to_string()));
        assert_eq!(styles.get("gap"), Some(&"10px".to_string()));
        assert_eq!(styles.get("min-height"), Some(&"50px".to_string()));
        assert_eq!(styles.get("max-width"), Some(&"500px".to_string()));
    }

    #[test]
    fn test_remove_child() {
        // 测试移除子节点
        let mut parent = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        let child2 = DOMNode::new_element("p");
        parent.append_child(child1);
        parent.append_child(child2);
        
        assert_eq!(parent.children.len(), 2);
        
        // 移除第一个子节点
        let removed = parent.remove_child(0);
        assert!(removed.is_some());
        assert_eq!(parent.children.len(), 1);
        
        // 移除不存在的索引
        let removed = parent.remove_child(5);
        assert!(removed.is_none());
    }

    #[test]
    fn test_insert_before() {
        // 测试在指定位置前插入节点
        let mut parent = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        parent.append_child(child1);
        
        let child2 = DOMNode::new_element("p");
        let success = parent.insert_before(child2, 0);
        
        assert!(success);
        assert_eq!(parent.children.len(), 2);
        
        // 验证插入位置正确（p 应该在 span 前面）
        assert!(matches!(&parent.children[0].node_type, NodeType::Element(e) if e == "p"));
        assert!(matches!(&parent.children[1].node_type, NodeType::Element(e) if e == "span"));
    }

    #[test]
    fn test_replace_child() {
        // 测试替换子节点
        let mut parent = DOMNode::new_element("div");
        let old_child = DOMNode::new_element("span");
        parent.append_child(old_child);
        
        let new_child = DOMNode::new_element("p");
        let replaced = parent.replace_child(new_child, 0);
        
        assert!(replaced.is_some());
        assert_eq!(parent.children.len(), 1);
        assert!(matches!(&parent.children[0].node_type, NodeType::Element(e) if e == "p"));
        
        // 测试无效索引
        let new_child2 = DOMNode::new_element("a");
        let replaced = parent.replace_child(new_child2, 5);
        assert!(replaced.is_none());
    }

    #[test]
    fn test_clone_node_shallow() {
        // 测试浅克隆
        let mut parent = DOMNode::new_element("div");
        parent.set_attribute("class", "container");
        let child = DOMNode::new_element("span");
        parent.append_child(child);
        
        let cloned = parent.clone_node(false);
        
        // 验证克隆后的节点属性相同，但 ID 不同
        assert_eq!(cloned.tag_name(), parent.tag_name());
        assert_eq!(cloned.get_attribute("class"), parent.get_attribute("class"));
        assert_ne!(cloned.id, parent.id); // ID 应该不同
        
        // 浅克隆不应该包含子节点
        assert_eq!(cloned.children.len(), 0);
    }

    #[test]
    fn test_clone_node_deep() {
        // 测试深克隆
        let mut parent = DOMNode::new_element("div");
        parent.set_attribute("class", "container");
        let child = DOMNode::new_element("span");
        parent.append_child(child);
        
        let cloned = parent.clone_node(true);
        
        // 深克隆应该包含子节点
        assert_eq!(cloned.children.len(), 1);
        assert_eq!(cloned.children[0].tag_name(), Some("span"));
        
        // 克隆的子节点 ID 也应该不同
        assert_ne!(cloned.children[0].id, parent.children[0].id);
        
        // 验证克隆后的 parent_id 关系正确
        assert_eq!(cloned.children[0].parent_id, cloned.id);
    }

    #[test]
    fn test_insert_before_at_end() {
        // 测试 insert_before 在末尾插入（边界情况）
        let mut parent = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        let child2 = DOMNode::new_element("p");
        parent.append_child(child1);
        parent.append_child(child2);
        
        // 在末尾插入（索引 == children.len()）
        let child3 = DOMNode::new_element("a");
        let success = parent.insert_before(child3, 2);
        
        assert!(success);
        assert_eq!(parent.children.len(), 3);
        
        // 验证插入位置（应该在最后）
        assert!(matches!(&parent.children[2].node_type, NodeType::Element(e) if e == "a"));
    }

    #[test]
    fn test_append_multiple() {
        // 测试 append 方法（现代 API）
        let mut parent = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        let child2 = DOMNode::new_element("p");
        let child3 = DOMNode::new_element("a");
        
        parent.append(vec![child1, child2, child3]);
        
        assert_eq!(parent.children.len(), 3);
    }

    #[test]
    fn test_prepend_multiple() {
        // 测试 prepend 方法（现代 API）
        let mut parent = DOMNode::new_element("div");
        let existing = DOMNode::new_element("span");
        parent.append_child(existing);
        
        let child1 = DOMNode::new_element("p");
        let child2 = DOMNode::new_element("a");
        
        parent.prepend(vec![child1, child2]);
        
        assert_eq!(parent.children.len(), 3);
        // 验证顺序：p, a, span
        assert!(matches!(&parent.children[0].node_type, NodeType::Element(e) if e == "p"));
        assert!(matches!(&parent.children[1].node_type, NodeType::Element(e) if e == "a"));
        assert!(matches!(&parent.children[2].node_type, NodeType::Element(e) if e == "span"));
    }

    #[test]
    fn test_contains() {
        // 测试 contains 方法
        let mut parent = DOMNode::new_element("div");
        let mut child = DOMNode::new_element("span");
        let grandchild = DOMNode::new_element("p");
        child.append_child(grandchild);
        parent.append_child(child);
        
        // 测试包含自己
        assert!(parent.contains(parent.id));
        
        // 测试包含子节点
        assert!(parent.contains(parent.children[0].id));
        
        // 测试包含孙节点
        assert!(parent.contains(parent.children[0].children[0].id));
        
        // 测试不包含的节点
        let unrelated = DOMNode::new_element("div");
        assert!(!parent.contains(unrelated.id));
    }

    #[test]
    fn test_child_count_and_has_children() {
        // 测试 child_count 和 has_children 方法
        let mut parent = DOMNode::new_element("div");
        
        assert_eq!(parent.child_count(), 0);
        assert!(!parent.has_children());
        
        let child = DOMNode::new_element("span");
        parent.append_child(child);
        
        assert_eq!(parent.child_count(), 1);
        assert!(parent.has_children());
    }

    #[test]
    fn test_first_and_last_child_mut() {
        // 测试 first_child_mut 和 last_child_mut
        let mut parent = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        let child2 = DOMNode::new_element("p");
        parent.append_child(child1);
        parent.append_child(child2);
        
        // 测试 first_child_mut
        if let Some(first) = parent.first_child_mut() {
            assert!(matches!(&first.node_type, NodeType::Element(e) if e == "span"));
        }
        
        // 测试 last_child_mut
        if let Some(last) = parent.last_child_mut() {
            assert!(matches!(&last.node_type, NodeType::Element(e) if e == "p"));
        }
        
        // 空节点测试
        let mut empty = DOMNode::new_element("div");
        assert!(empty.first_child_mut().is_none());
        assert!(empty.last_child_mut().is_none());
    }
}
