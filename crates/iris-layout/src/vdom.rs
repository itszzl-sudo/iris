//! 虚拟 DOM 实现
//!
//! 提供虚拟 DOM 树、Diff 算法和 Patch 算法，用于高效的 DOM 更新。

use crate::dom::{DOMNode, NodeType};
use std::collections::HashMap;

/// 虚拟 DOM 节点
#[derive(Debug, Clone, PartialEq)]
pub enum VNode {
    /// 元素节点
    Element(VElement),
    /// 文本节点
    Text(String),
    /// 注释节点
    Comment(String),
}

/// 虚拟元素节点
#[derive(Debug, Clone, PartialEq)]
pub struct VElement {
    /// 标签名
    pub tag: String,
    /// 属性
    pub attrs: HashMap<String, String>,
    /// 子节点
    pub children: Vec<VNode>,
    /// 节点键值（用于优化列表渲染）
    pub key: Option<String>,
}

impl VElement {
    /// 创建新的虚拟元素
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_lowercase(),
            attrs: HashMap::new(),
            children: Vec::new(),
            key: None,
        }
    }

    /// 设置属性
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.attrs.insert(key.to_string(), value.to_string());
        self
    }

    /// 设置键值
    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(key.to_string());
        self
    }

    /// 添加子节点
    pub fn child(mut self, child: impl Into<VNode>) -> Self {
        self.children.push(child.into());
        self
    }

    /// 批量添加子节点
    pub fn children(mut self, children: Vec<VNode>) -> Self {
        self.children.extend(children);
        self
    }
}

impl VNode {
    /// 创建元素节点
    pub fn element(tag: &str) -> VElement {
        VElement::new(tag)
    }

    /// 创建文本节点
    pub fn text(content: &str) -> Self {
        VNode::Text(content.to_string())
    }

    /// 创建注释节点
    pub fn comment(content: &str) -> Self {
        VNode::Comment(content.to_string())
    }

    /// 获取标签名（仅对元素节点有效）
    pub fn tag_name(&self) -> Option<&str> {
        match self {
            VNode::Element(elem) => Some(&elem.tag),
            _ => None,
        }
    }

    /// 获取键值
    pub fn key(&self) -> Option<&str> {
        match self {
            VNode::Element(elem) => elem.key.as_deref(),
            _ => None,
        }
    }
}

impl From<VElement> for VNode {
    fn from(elem: VElement) -> Self {
        VNode::Element(elem)
    }
}

/// Diff 操作
#[derive(Debug, Clone)]
pub enum Patch {
    /// 插入节点
    Insert {
        /// 父节点路径
        parent_path: Vec<usize>,
        /// 插入位置
        index: usize,
        /// 新节点
        node: VNode,
    },
    /// 删除节点
    Remove {
        /// 父节点路径
        parent_path: Vec<usize>,
        /// 删除位置
        index: usize,
    },
    /// 替换节点
    Replace {
        /// 节点路径
        path: Vec<usize>,
        /// 新节点
        new_node: VNode,
    },
    /// 更新属性
    UpdateAttrs {
        /// 节点路径
        path: Vec<usize>,
        /// 新增或更新的属性
        added: HashMap<String, String>,
        /// 删除的属性
        removed: Vec<String>,
    },
    /// 更新文本
    UpdateText {
        /// 节点路径
        path: Vec<usize>,
        /// 新文本内容
        text: String,
    },
}

/// 虚拟 DOM 树
#[derive(Debug, Clone)]
pub struct VTree {
    /// 根节点
    pub root: VNode,
}

impl VTree {
    /// 创建新的虚拟 DOM 树
    pub fn new(root: impl Into<VNode>) -> Self {
        Self {
            root: root.into(),
        }
    }

    /// 从 DOMNode 创建虚拟 DOM 树
    pub fn from_dom_node(node: &DOMNode) -> Self {
        Self {
            root: Self::convert_to_vnode(node),
        }
    }

    /// 将 DOMNode 转换为 VNode
    fn convert_to_vnode(node: &DOMNode) -> VNode {
        match &node.node_type {
            NodeType::Element(tag) => {
                let mut velement = VElement::new(tag);
                
                // 复制属性
                for (key, value) in &node.attributes {
                    velement.attrs.insert(key.clone(), value.clone());
                }
                
                // 递归转换子节点
                for child in &node.children {
                    velement.children.push(Self::convert_to_vnode(child));
                }
                
                VNode::Element(velement)
            }
            NodeType::Text(text) => VNode::Text(text.clone()),
            NodeType::Comment(comment) => VNode::Comment(comment.clone()),
        }
    }

    /// 将虚拟 DOM 树转换为 DOMNode
    pub fn to_dom_node(&self) -> DOMNode {
        Self::convert_to_dom_node(&self.root, 0)
    }

    /// 将 VNode 转换为 DOMNode
    fn convert_to_dom_node(vnode: &VNode, parent_id: u64) -> DOMNode {
        match vnode {
            VNode::Element(velement) => {
                let mut dom_node = DOMNode::new_element(&velement.tag);
                dom_node.parent_id = parent_id;
                
                // 复制属性
                for (key, value) in &velement.attrs {
                    dom_node.set_attribute(key, value);
                }
                
                // 递归转换子节点
                for child in &velement.children {
                    dom_node.children.push(Self::convert_to_dom_node(child, dom_node.id));
                }
                
                dom_node
            }
            VNode::Text(text) => {
                let mut dom_node = DOMNode::new_text(text);
                dom_node.parent_id = parent_id;
                dom_node
            }
            VNode::Comment(comment) => {
                let mut dom_node = DOMNode::new_comment(comment);
                dom_node.parent_id = parent_id;
                dom_node
            }
        }
    }

    /// 计算两个虚拟 DOM 树的差异
    pub fn diff(&self, old_tree: &VTree) -> Vec<Patch> {
        let mut patches = Vec::new();
        Self::diff_nodes(&old_tree.root, &self.root, &mut patches, vec![]);
        patches
    }

    /// 递归比较两个节点
    fn diff_nodes(old_node: &VNode, new_node: &VNode, patches: &mut Vec<Patch>, path: Vec<usize>) {
        match (old_node, new_node) {
            // 两个都是元素节点
            (VNode::Element(old_elem), VNode::Element(new_elem)) => {
                // 如果标签名不同，直接替换
                if old_elem.tag != new_elem.tag {
                    patches.push(Patch::Replace {
                        path,
                        new_node: new_node.clone(),
                    });
                    return;
                }

                // 比较属性
                let mut added = HashMap::new();
                let mut removed = Vec::new();

                // 查找新增或更新的属性
                for (key, new_value) in &new_elem.attrs {
                    match old_elem.attrs.get(key) {
                        Some(old_value) if old_value != new_value => {
                            added.insert(key.clone(), new_value.clone());
                        }
                        None => {
                            added.insert(key.clone(), new_value.clone());
                        }
                        _ => {} // 属性未变化
                    }
                }

                // 查找删除的属性
                for key in old_elem.attrs.keys() {
                    if !new_elem.attrs.contains_key(key) {
                        removed.push(key.clone());
                    }
                }

                // 如果有属性变化，添加更新操作
                if !added.is_empty() || !removed.is_empty() {
                    patches.push(Patch::UpdateAttrs {
                        path: path.clone(),
                        added,
                        removed,
                    });
                }

                // 递归比较子节点
                Self::diff_children(&old_elem.children, &new_elem.children, patches, path);
            }

            // 两个都是文本节点
            (VNode::Text(old_text), VNode::Text(new_text)) => {
                if old_text != new_text {
                    patches.push(Patch::UpdateText {
                        path,
                        text: new_text.clone(),
                    });
                }
            }

            // 两个都是注释节点
            (VNode::Comment(old_comment), VNode::Comment(new_comment)) => {
                if old_comment != new_comment {
                    patches.push(Patch::UpdateText {
                        path,
                        text: new_comment.clone(),
                    });
                }
            }

            // 节点类型不同，直接替换
            _ => {
                patches.push(Patch::Replace {
                    path,
                    new_node: new_node.clone(),
                });
            }
        }
    }

    /// 比较子节点列表
    fn diff_children(
        old_children: &[VNode],
        new_children: &[VNode],
        patches: &mut Vec<Patch>,
        parent_path: Vec<usize>,
    ) {
        let max_len = old_children.len().max(new_children.len());

        for i in 0..max_len {
            let mut child_path = parent_path.clone();
            child_path.push(i);

            match (old_children.get(i), new_children.get(i)) {
                // 两个节点都存在，递归比较
                (Some(old_child), Some(new_child)) => {
                    Self::diff_nodes(old_child, new_child, patches, child_path);
                }
                // 新节点存在，旧节点不存在 -> 插入
                (None, Some(new_child)) => {
                    patches.push(Patch::Insert {
                        parent_path: parent_path.clone(),
                        index: i,
                        node: new_child.clone(),
                    });
                }
                // 旧节点存在，新节点不存在 -> 删除
                (Some(_), None) => {
                    patches.push(Patch::Remove {
                        parent_path: parent_path.clone(),
                        index: i,
                    });
                }
                // 都不存在，不可能发生
                (None, None) => {}
            }
        }
    }

    /// 应用差异补丁到 DOM 节点
    pub fn apply_patches(&self, dom_root: &mut DOMNode, patches: &[Patch]) {
        for patch in patches {
            self.apply_patch(dom_root, patch);
        }
    }

    /// 应用单个补丁
    fn apply_patch(&self, dom_root: &mut DOMNode, patch: &Patch) {
        match patch {
            Patch::Insert {
                parent_path,
                index,
                node,
            } => {
                if let Some(parent) = self.get_node_mut(dom_root, parent_path) {
                    let dom_node = Self::convert_to_dom_node(node, parent.id);
                    parent.children.insert(*index, dom_node);
                }
            }
            Patch::Remove {
                parent_path,
                index,
            } => {
                if let Some(parent) = self.get_node_mut(dom_root, parent_path) {
                    if *index < parent.children.len() {
                        parent.children.remove(*index);
                    }
                }
            }
            Patch::Replace { path, new_node } => {
                // 替换需要找到父节点并替换子节点
                if path.is_empty() {
                    // 替换根节点
                    *dom_root = Self::convert_to_dom_node(new_node, 0);
                } else if path.len() > 0 {
                    // 获取父节点路径
                    let parent_path = &path[..path.len()-1];
                    let index = path[path.len()-1];
                    
                    if let Some(parent) = self.get_node_mut(dom_root, parent_path) {
                        if index < parent.children.len() {
                            let dom_node = Self::convert_to_dom_node(new_node, parent.id);
                            parent.children[index] = dom_node;
                        }
                    }
                }
            }
            Patch::UpdateAttrs {
                path,
                added,
                removed,
            } => {
                if let Some(node) = self.get_node_mut(dom_root, path) {
                    // 删除属性
                    for key in removed {
                        node.attributes.remove(key);
                    }
                    // 添加或更新属性
                    for (key, value) in added {
                        node.set_attribute(key, value);
                    }
                }
            }
            Patch::UpdateText { path, text } => {
                if let Some(node) = self.get_node_mut(dom_root, path) {
                    if let NodeType::Text(_) = &node.node_type {
                        node.node_type = NodeType::Text(text.clone());
                    }
                }
            }
        }
    }

    /// 根据路径获取节点的可变引用
    fn get_node_mut<'a>(&self, root: &'a mut DOMNode, path: &[usize]) -> Option<&'a mut DOMNode> {
        let mut current = root;
        for &index in path {
            if index < current.children.len() {
                current = &mut current.children[index];
            } else {
                return None;
            }
        }
        Some(current)
    }

    /// 获取父节点和索引的可变引用
    fn get_parent_and_index_mut<'a>(
        &self,
        root: &'a mut DOMNode,
        path: &[usize],
    ) -> Option<(&'a mut DOMNode, usize)> {
        if path.is_empty() {
            return None;
        }

        let (parent_path, index) = path.split_at(path.len() - 1);
        let index = index[0];

        if let Some(parent) = self.get_node_mut(root, parent_path) {
            if index < parent.children.len() {
                // 这里需要使用 unsafe 或者重新设计数据结构
                // 简化处理：返回父节点，调用者自行处理
                return Some((parent, index));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vnode_creation() {
        // 测试虚拟节点创建
        let vtree = VTree::new(VNode::element("div")
            .attr("class", "container")
            .child(VNode::text("Hello"))
            .child(VNode::element("span").child(VNode::text("World"))));

        assert_eq!(vtree.root.tag_name(), Some("div"));
    }

    #[test]
    fn test_vtree_from_dom_node() {
        // 测试从 DOMNode 创建虚拟 DOM
        let mut dom_node = DOMNode::new_element("div");
        dom_node.set_attribute("class", "container");
        
        let child = DOMNode::new_text("Hello");
        dom_node.children.push(child);

        let vtree = VTree::from_dom_node(&dom_node);

        assert_eq!(vtree.root.tag_name(), Some("div"));
    }

    #[test]
    fn test_vtree_to_dom_node() {
        // 测试虚拟 DOM 转换为 DOMNode
        let vtree = VTree::new(VNode::element("div")
            .attr("id", "app")
            .child(VNode::text("Test")));

        let dom_node = vtree.to_dom_node();

        assert!(matches!(dom_node.node_type, NodeType::Element(ref tag) if tag == "div"));
        assert_eq!(dom_node.get_attribute("id"), Some(&"app".to_string()));
        assert_eq!(dom_node.children.len(), 1);
    }

    #[test]
    fn test_diff_text_change() {
        // 测试文本变化的 diff
        let old_tree = VTree::new(VNode::text("old"));
        let new_tree = VTree::new(VNode::text("new"));

        let patches = new_tree.diff(&old_tree);

        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::UpdateText { text, .. } if text == "new"));
    }

    #[test]
    fn test_diff_element_replace() {
        // 测试元素替换的 diff
        let old_tree = VTree::new(VNode::element("div"));
        let new_tree = VTree::new(VNode::element("span"));

        let patches = new_tree.diff(&old_tree);

        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::Replace { .. }));
    }

    #[test]
    fn test_diff_attr_update() {
        // 测试属性更新的 diff
        let old_tree = VTree::new(VNode::element("div").attr("class", "old"));
        let new_tree = VTree::new(VNode::element("div").attr("class", "new").attr("id", "app"));

        let patches = new_tree.diff(&old_tree);

        assert_eq!(patches.len(), 1);
        if let Patch::UpdateAttrs { added, removed, .. } = &patches[0] {
            assert_eq!(added.get("class"), Some(&"new".to_string()));
            assert_eq!(added.get("id"), Some(&"app".to_string()));
            // class 只是更新，不在 removed 中
            assert!(removed.is_empty());
        } else {
            panic!("Expected UpdateAttrs patch");
        }
    }

    #[test]
    fn test_diff_children_insert() {
        // 测试子节点插入的 diff
        let old_tree = VTree::new(VNode::element("div")
            .child(VNode::text("first")));
        let new_tree = VTree::new(VNode::element("div")
            .child(VNode::text("first"))
            .child(VNode::text("second")));

        let patches = new_tree.diff(&old_tree);

        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::Insert { index: 1, .. }));
    }

    #[test]
    fn test_diff_children_remove() {
        // 测试子节点删除的 diff
        let old_tree = VTree::new(VNode::element("div")
            .child(VNode::text("first"))
            .child(VNode::text("second")));
        let new_tree = VTree::new(VNode::element("div")
            .child(VNode::text("first")));

        let patches = new_tree.diff(&old_tree);

        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::Remove { index: 1, .. }));
    }

    #[test]
    fn test_apply_patches_insert() {
        // 测试应用插入补丁
        let vtree = VTree::new(VNode::element("div"));
        let mut dom_node = DOMNode::new_element("div");

        let patches = vec![Patch::Insert {
            parent_path: vec![],
            index: 0,
            node: VNode::text("Hello"),
        }];

        vtree.apply_patches(&mut dom_node, &patches);

        assert_eq!(dom_node.children.len(), 1);
        assert!(matches!(&dom_node.children[0].node_type, NodeType::Text(text) if text == "Hello"));
    }

    #[test]
    fn test_apply_patches_update_attrs() {
        // 测试应用属性更新补丁
        let vtree = VTree::new(VNode::element("div"));
        let mut dom_node = DOMNode::new_element("div");
        dom_node.set_attribute("class", "old");

        let mut added = HashMap::new();
        added.insert("class".to_string(), "new".to_string());
        added.insert("id".to_string(), "app".to_string());

        let patches = vec![Patch::UpdateAttrs {
            path: vec![],
            added,
            removed: vec!["class".to_string()],
        }];

        vtree.apply_patches(&mut dom_node, &patches);

        assert_eq!(dom_node.get_attribute("class"), Some(&"new".to_string()));
        assert_eq!(dom_node.get_attribute("id"), Some(&"app".to_string()));
    }

    #[test]
    fn test_vtree_roundtrip() {
        // 测试 DOM -> VTree -> DOM 的往返转换
        let mut original = DOMNode::new_element("div");
        original.set_attribute("class", "container");
        
        let child = DOMNode::new_text("Hello World");
        original.children.push(child);

        let vtree = VTree::from_dom_node(&original);
        let restored = vtree.to_dom_node();

        assert_eq!(restored.tag_name(), original.tag_name());
        assert_eq!(restored.get_attribute("class"), original.get_attribute("class"));
        assert_eq!(restored.children.len(), original.children.len());
    }
}
