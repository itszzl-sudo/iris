//! 虚拟 DOM (VNode) 数据结构
//!
//! 提供轻量级的虚拟 DOM 表示，支持差异比较和高效更新。

use iris_layout::dom::DOMNode;
use iris_layout::layout::LayoutBox;
use iris_layout::style::ComputedStyles;
use std::collections::HashMap;

/// 虚拟 DOM 节点
///
/// 表示 UI 的一个声明式描述，与实际 DOM 分离。
#[derive(Debug, Clone)]
pub enum VNode {
    /// 元素节点
    Element {
        /// 标签名
        tag: String,
        /// 属性
        attrs: HashMap<String, String>,
        /// 子节点
        children: Vec<VNode>,
        /// 计算后的样式
        styles: ComputedStyles,
        /// 布局信息
        layout: Option<LayoutBox>,
    },
    /// 文本节点
    Text {
        /// 文本内容
        content: String,
    },
    /// 注释节点
    Comment {
        /// 注释内容
        content: String,
    },
    /// Fragment (不渲染的包装节点)
    Fragment {
        /// 子节点
        children: Vec<VNode>,
    },
}

impl VNode {
    /// 创建元素节点
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_dom::vnode::VNode;
    ///
    /// let div = VNode::element("div");
    /// ```
    pub fn element(tag: &str) -> Self {
        VNode::Element {
            tag: tag.to_string(),
            attrs: HashMap::new(),
            children: Vec::new(),
            styles: ComputedStyles::new(),
            layout: None,
        }
    }

    /// 创建文本节点
    pub fn text(content: &str) -> Self {
        VNode::Text {
            content: content.to_string(),
        }
    }

    /// 创建注释节点
    pub fn comment(content: &str) -> Self {
        VNode::Comment {
            content: content.to_string(),
        }
    }

    /// 创建 Fragment
    pub fn fragment(children: Vec<VNode>) -> Self {
        VNode::Fragment { children }
    }

    /// 设置属性
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_dom::vnode::VNode;
    ///
    /// let mut div = VNode::element("div");
    /// div.set_attr("class", "container");
    /// div.set_attr("id", "main");
    /// ```
    pub fn set_attr(&mut self, key: &str, value: &str) {
        if let VNode::Element { attrs, .. } = self {
            attrs.insert(key.to_string(), value.to_string());
        }
    }

    /// 获取属性
    pub fn get_attr(&self, key: &str) -> Option<&String> {
        match self {
            VNode::Element { attrs, .. } => attrs.get(key),
            _ => None,
        }
    }

    /// 添加子节点
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_dom::vnode::VNode;
    ///
    /// let mut div = VNode::element("div");
    /// div.append_child(VNode::text("Hello"));
    /// div.append_child(VNode::element("p"));
    /// ```
    pub fn append_child(&mut self, child: VNode) {
        match self {
            VNode::Element { children, .. } | VNode::Fragment { children } => {
                children.push(child);
            }
            _ => {}
        }
    }

    /// 获取标签名
    pub fn tag_name(&self) -> Option<&str> {
        match self {
            VNode::Element { tag, .. } => Some(tag),
            _ => None,
        }
    }

    /// 获取文本内容
    pub fn text_content(&self) -> Option<&str> {
        match self {
            VNode::Text { content } => Some(content),
            _ => None,
        }
    }

    /// 获取子节点引用（只读）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_dom::vnode::VNode;
    ///
    /// let mut div = VNode::element("div");
    /// div.append_child(VNode::text("Hello"));
    /// assert_eq!(div.children().len(), 1);
    /// ```
    pub fn children(&self) -> &[VNode] {
        match self {
            VNode::Element { children, .. } | VNode::Fragment { children } => children,
            _ => &[],
        }
    }

    /// 获取子节点可变引用（可修改）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_dom::vnode::VNode;
    ///
    /// let mut div = VNode::element("div");
    /// div.append_child(VNode::text("Hello"));
    /// div.children_mut()[0] = VNode::text("World");
    /// ```
    pub fn children_mut(&mut self) -> &mut Vec<VNode> {
        match self {
            VNode::Element { children, .. } | VNode::Fragment { children } => children,
            _ => panic!("Cannot get mutable children reference for non-element/fragment node"),
        }
    }

    /// 设置样式
    pub fn set_style(&mut self, property: &str, value: &str) {
        if let VNode::Element { styles, .. } = self {
            styles.set(property, value);
        }
    }

    /// 获取样式
    pub fn get_style(&self, property: &str) -> Option<&String> {
        match self {
            VNode::Element { styles, .. } => styles.get(property),
            _ => None,
        }
    }

    /// 设置布局信息
    pub fn set_layout(&mut self, layout: LayoutBox) {
        if let VNode::Element {
            layout: layout_ref,
            ..
        } = self
        {
            *layout_ref = Some(layout);
        }
    }

    /// 获取布局信息
    pub fn get_layout(&self) -> Option<&LayoutBox> {
        match self {
            VNode::Element { layout, .. } => layout.as_ref(),
            _ => None,
        }
    }

    /// 递归收集所有文本内容
    pub fn collect_text(&self) -> String {
        match self {
            VNode::Text { content } => content.clone(),
            VNode::Element { children, .. } | VNode::Fragment { children } => {
                let mut text = String::new();
                for child in children {
                    text.push_str(&child.collect_text());
                }
                text
            }
            VNode::Comment { .. } => String::new(),
        }
    }

    /// 获取子节点数量
    pub fn child_count(&self) -> usize {
        match self {
            VNode::Element { children, .. } | VNode::Fragment { children } => children.len(),
            _ => 0,
        }
    }

    /// 判断是否为元素节点
    pub fn is_element(&self) -> bool {
        matches!(self, VNode::Element { .. })
    }

    /// 判断是否为文本节点
    pub fn is_text(&self) -> bool {
        matches!(self, VNode::Text { .. })
    }
}

/// 将 DOMNode 转换为 VNode
///
/// # 示例
///
/// ```rust
/// use iris_layout::dom::DOMNode;
/// use iris_dom::vnode::dom_to_vnode;
///
/// let dom_node = DOMNode::new_element("div");
/// let vnode = dom_to_vnode(&dom_node);
/// assert!(vnode.is_element());
/// ```
pub fn dom_to_vnode(node: &DOMNode) -> VNode {
    match &node.node_type {
        iris_layout::dom::NodeType::Element(tag) => {
            let mut children = Vec::new();
            for child in &node.children {
                children.push(dom_to_vnode(child));
            }

            VNode::Element {
                tag: tag.clone(),
                attrs: node.attributes.clone(),
                children,
                styles: ComputedStyles::new(),
                layout: None,
            }
        }
        iris_layout::dom::NodeType::Text(text) => VNode::Text {
            content: text.clone(),
        },
        iris_layout::dom::NodeType::Comment(comment) => VNode::Comment {
            content: comment.clone(),
        },
    }
}

/// VNode 差异比较结果
#[derive(Debug)]
pub struct DiffResult {
    /// 需要更新的节点路径
    pub patches: Vec<Patch>,
}

/// 补丁操作
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Patch {
    /// 插入节点
    Insert { index: usize, node: VNode },
    /// 删除节点
    Remove { index: usize },
    /// 替换节点
    Replace { index: usize, node: VNode },
    /// 更新属性
    UpdateAttr { key: String, value: String },
    /// 删除属性
    RemoveAttr { key: String },
    /// 更新文本
    UpdateText { content: String },
}

/// 比较两个 VNode，生成差异补丁
///
/// # 示例
///
/// ```rust
/// use iris_dom::vnode::{VNode, diff_vnodes};
///
/// let old = VNode::element("div");
/// let new = VNode::element("div");
/// let diff = diff_vnodes(&old, &new);
/// ```
pub fn diff_vnodes(old: &VNode, new: &VNode) -> DiffResult {
    let mut patches = Vec::new();
    diff_recursive(old, new, &mut patches);
    DiffResult { patches }
}

fn diff_recursive(old: &VNode, new: &VNode, patches: &mut Vec<Patch>) {
    // 类型不同，直接替换
    if std::mem::discriminant(old) != std::mem::discriminant(new) {
        patches.push(Patch::Replace {
            index: 0,
            node: new.clone(),
        });
        return;
    }

    // 根据类型比较
    match (old, new) {
        (VNode::Text { content: old_text }, VNode::Text { content: new_text }) => {
            if old_text != new_text {
                patches.push(Patch::UpdateText {
                    content: new_text.clone(),
                });
            }
        }
        (
            VNode::Element {
                attrs: old_attrs,
                children: old_children,
                ..
            },
            VNode::Element {
                attrs: new_attrs,
                children: new_children,
                ..
            },
        ) => {
            // 比较属性
            for (key, value) in new_attrs {
                if old_attrs.get(key) != Some(value) {
                    patches.push(Patch::UpdateAttr {
                        key: key.clone(),
                        value: value.clone(),
                    });
                }
            }
            for key in old_attrs.keys() {
                if !new_attrs.contains_key(key) {
                    patches.push(Patch::RemoveAttr {
                        key: key.clone(),
                    });
                }
            }

            // 递归比较子节点
            let max_len = old_children.len().max(new_children.len());
            for i in 0..max_len {
                if i >= old_children.len() {
                    // 新节点更多，插入
                    patches.push(Patch::Insert {
                        index: i,
                        node: new_children[i].clone(),
                    });
                } else if i >= new_children.len() {
                    // 旧节点更多，删除
                    patches.push(Patch::Remove { index: i });
                } else {
                    // 都有，递归比较
                    diff_recursive(&old_children[i], &new_children[i], patches);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_element() {
        let div = VNode::element("div");
        assert!(div.is_element());
        assert_eq!(div.tag_name(), Some("div"));
    }

    #[test]
    fn test_create_text() {
        let text = VNode::text("Hello");
        assert!(text.is_text());
        assert_eq!(text.text_content(), Some("Hello"));
    }

    #[test]
    fn test_set_attr() {
        let mut div = VNode::element("div");
        div.set_attr("class", "container");
        div.set_attr("id", "main");

        assert_eq!(div.get_attr("class"), Some(&"container".to_string()));
        assert_eq!(div.get_attr("id"), Some(&"main".to_string()));
    }

    #[test]
    fn test_append_child() {
        let mut div = VNode::element("div");
        div.append_child(VNode::text("Hello"));
        div.append_child(VNode::element("p"));

        assert_eq!(div.child_count(), 2);
    }

    #[test]
    fn test_collect_text() {
        let mut div = VNode::element("div");
        div.append_child(VNode::text("Hello "));

        let mut p = VNode::element("p");
        p.append_child(VNode::text("World"));
        div.append_child(p);

        assert_eq!(div.collect_text(), "Hello World");
    }

    #[test]
    fn test_dom_to_vnode() {
        let dom_node = DOMNode::new_element("div");
        let vnode = dom_to_vnode(&dom_node);

        assert!(vnode.is_element());
        assert_eq!(vnode.tag_name(), Some("div"));
    }

    #[test]
    fn test_diff_text_change() {
        let old = VNode::text("Hello");
        let new = VNode::text("World");

        let diff = diff_vnodes(&old, &new);
        assert_eq!(diff.patches.len(), 1);
    }

    #[test]
    fn test_diff_attr_change() {
        let mut old = VNode::element("div");
        old.set_attr("class", "old");

        let mut new = VNode::element("div");
        new.set_attr("class", "new");
        new.set_attr("id", "main");

        let diff = diff_vnodes(&old, &new);
        assert!(!diff.patches.is_empty());
    }

    #[test]
    fn test_diff_children_change() {
        let mut old = VNode::element("div");
        old.append_child(VNode::text("Child 1"));

        let mut new = VNode::element("div");
        new.append_child(VNode::text("Child 1"));
        new.append_child(VNode::text("Child 2"));

        let diff = diff_vnodes(&old, &new);
        assert!(!diff.patches.is_empty());
    }
}
