//! DOM 树管理
//!
//! 提供完整的 DOM 树操作，包括需要父节点上下文的方法。

use crate::dom::DOMNode;
use std::collections::HashMap;

/// DOM 路径（用于定位节点）
#[derive(Debug, Clone, PartialEq)]
pub struct DOMPath {
    /// 从根节点到目标节点的索引路径
    pub indices: Vec<usize>,
}

impl DOMPath {
    /// 创建根路径
    pub fn root() -> Self {
        Self { indices: vec![] }
    }

    /// 创建子路径
    pub fn child(&self, index: usize) -> Self {
        let mut new_path = self.clone();
        new_path.indices.push(index);
        new_path
    }
}

/// DOM 树
///
/// 管理完整的 DOM 树结构，提供需要父节点上下文的操作。
#[derive(Debug, Clone)]
pub struct DOMTree {
    /// 根节点
    pub root: DOMNode,
    /// 节点索引（ID → Path）
    node_index: HashMap<u64, DOMPath>,
}

impl DOMTree {
    /// 创建新的 DOM 树
    pub fn new(root: DOMNode) -> Self {
        let mut tree = Self {
            root,
            node_index: HashMap::new(),
        };
        tree.build_index();
        tree
    }

    /// 构建节点索引
    fn build_index(&mut self) {
        let mut new_index = HashMap::new();
        Self::build_index_recursive(&self.root, DOMPath::root(), &mut new_index);
        self.node_index = new_index;
    }

    /// 递归构建索引
    fn build_index_recursive(node: &DOMNode, path: DOMPath, index: &mut HashMap<u64, DOMPath>) {
        index.insert(node.id, path.clone());
        
        for (i, child) in node.children.iter().enumerate() {
            Self::build_index_recursive(child, path.child(i), index);
        }
    }

    /// 根据 ID 查找节点
    pub fn get_node(&self, node_id: u64) -> Option<&DOMNode> {
        self.node_index.get(&node_id).and_then(|path| {
            self.get_node_by_path(&path)
        })
    }

    /// 根据 ID 查找节点的可变引用
    pub fn get_node_mut(&mut self, node_id: u64) -> Option<&mut DOMNode> {
        let path = self.node_index.get(&node_id)?.clone();
        self.get_node_by_path_mut(&path)
    }

    /// 根据路径查找节点
    fn get_node_by_path(&self, path: &DOMPath) -> Option<&DOMNode> {
        let mut current = &self.root;
        for &index in &path.indices {
            if index < current.children.len() {
                current = &current.children[index];
            } else {
                return None;
            }
        }
        Some(current)
    }

    /// 根据路径查找节点的可变引用
    fn get_node_by_path_mut(&mut self, path: &DOMPath) -> Option<&mut DOMNode> {
        let mut current = &mut self.root;
        for &index in &path.indices {
            if index < current.children.len() {
                current = &mut current.children[index];
            } else {
                return None;
            }
        }
        Some(current)
    }

    /// 在节点之后插入兄弟节点
    ///
    /// # 参数
    ///
    /// * `reference_id` - 参考节点的 ID
    /// * `new_node` - 要插入的新节点
    ///
    /// # 返回
    ///
    /// 如果插入成功返回 true，否则返回 false
    pub fn insert_after(&mut self, reference_id: u64, new_node: DOMNode) -> bool {
        if let Some(path) = self.node_index.get(&reference_id).cloned() {
            if path.indices.is_empty() {
                // 根节点没有兄弟
                return false;
            }

            // 获取父节点路径
            let parent_indices = path.indices[..path.indices.len() - 1].to_vec();
            let index = path.indices[path.indices.len() - 1];

            // 获取父节点
            let parent_path = DOMPath { indices: parent_indices };
            if let Some(parent) = self.get_node_by_path_mut(&parent_path) {
                if index + 1 <= parent.children.len() {
                    let mut new_node = new_node;
                    new_node.parent_id = parent.id;
                    parent.children.insert(index + 1, new_node);
                    self.build_index(); // 重建索引
                    return true;
                }
            }
        }
        false
    }

    /// 在节点之前插入兄弟节点
    ///
    /// # 参数
    ///
    /// * `reference_id` - 参考节点的 ID
    /// * `new_node` - 要插入的新节点
    ///
    /// # 返回
    ///
    /// 如果插入成功返回 true，否则返回 false
    pub fn insert_before_node(&mut self, reference_id: u64, new_node: DOMNode) -> bool {
        if let Some(path) = self.node_index.get(&reference_id).cloned() {
            if path.indices.is_empty() {
                // 根节点没有兄弟
                return false;
            }

            // 获取父节点路径
            let parent_indices = path.indices[..path.indices.len() - 1].to_vec();
            let index = path.indices[path.indices.len() - 1];

            // 获取父节点
            let parent_path = DOMPath { indices: parent_indices };
            if let Some(parent) = self.get_node_by_path_mut(&parent_path) {
                if index <= parent.children.len() {
                    let mut new_node = new_node;
                    new_node.parent_id = parent.id;
                    parent.children.insert(index, new_node);
                    self.build_index(); // 重建索引
                    return true;
                }
            }
        }
        false
    }

    /// 移除节点自身
    ///
    /// # 参数
    ///
    /// * `node_id` - 要移除的节点 ID
    ///
    /// # 返回
    ///
    /// 如果移除成功返回 Some(被移除的节点)，否则返回 None
    pub fn remove_node(&mut self, node_id: u64) -> Option<DOMNode> {
        if let Some(path) = self.node_index.get(&node_id).cloned() {
            if path.indices.is_empty() {
                // 不能移除根节点
                return None;
            }

            // 获取父节点路径
            let parent_indices = path.indices[..path.indices.len() - 1].to_vec();
            let index = path.indices[path.indices.len() - 1];

            // 获取父节点并移除子节点
            let parent_path = DOMPath { indices: parent_indices };
            if let Some(parent) = self.get_node_by_path_mut(&parent_path) {
                if index < parent.children.len() {
                    let removed = parent.children.remove(index);
                    self.build_index(); // 重建索引
                    return Some(removed);
                }
            }
        }
        None
    }

    /// 比较两个节点的文档位置关系
    ///
    /// # 返回
    ///
    /// 返回位掩码表示位置关系：
    /// - 0: 同一节点
    /// - 1: node1 在 node2 之前
    /// - 2: node1 在 node2 之后
    /// - 4: node1 包含 node2
    /// - 8: node1 被 node2 包含
    /// - 16: node1 和 node2 无关联
    pub fn compare_document_position(&self, node1_id: u64, node2_id: u64) -> u32 {
        if node1_id == node2_id {
            return 0; // 同一节点
        }

        let path1 = match self.node_index.get(&node1_id) {
            Some(p) => p,
            None => return 16, // 节点不存在
        };
        let path2 = match self.node_index.get(&node2_id) {
            Some(p) => p,
            None => return 16,
        };

        // 检查包含关系
        if path2.indices.starts_with(&path1.indices) && path2.indices.len() > path1.indices.len() {
            return 4; // node1 包含 node2
        }

        if path1.indices.starts_with(&path2.indices) && path1.indices.len() > path2.indices.len() {
            return 8; // node1 被 node2 包含
        }

        // 比较字典序
        for (i1, i2) in path1.indices.iter().zip(path2.indices.iter()) {
            if i1 < i2 {
                return 1; // node1 在 node2 之前
            } else if i1 > i2 {
                return 2; // node1 在 node2 之后
            }
        }

        // 长度不同
        if path1.indices.len() < path2.indices.len() {
            return 1;
        } else {
            return 2;
        }
    }

    /// 获取节点的深度（根节点深度为 0）
    pub fn get_depth(&self, node_id: u64) -> Option<usize> {
        self.node_index.get(&node_id).map(|path| path.indices.len())
    }

    /// 获取节点的所有祖先节点
    pub fn get_ancestors(&self, node_id: u64) -> Vec<u64> {
        let mut ancestors = Vec::new();
        
        if let Some(path) = self.node_index.get(&node_id) {
            for i in 0..path.indices.len() {
                let ancestor_path = DOMPath { 
                    indices: path.indices[..i].to_vec() 
                };
                if let Some(ancestor) = self.get_node_by_path(&ancestor_path) {
                    ancestors.push(ancestor.id);
                }
            }
        }

        ancestors
    }

    /// 获取节点的兄弟节点数量
    pub fn get_sibling_count(&self, node_id: u64) -> usize {
        if let Some(path) = self.node_index.get(&node_id) {
            if path.indices.is_empty() {
                return 1; // 根节点
            }

            let parent_indices = &path.indices[..path.indices.len() - 1];
            let parent_path = DOMPath { indices: parent_indices.to_vec() };
            
            if let Some(parent) = self.get_node_by_path(&parent_path) {
                return parent.children.len();
            }
        }
        0
    }

    /// 获取节点的兄弟索引
    pub fn get_sibling_index(&self, node_id: u64) -> Option<usize> {
        self.node_index.get(&node_id).map(|path| {
            if path.indices.is_empty() {
                0
            } else {
                path.indices[path.indices.len() - 1]
            }
        })
    }

    /// 更新索引（局部优化，避免完全重建）
    pub fn rebuild_index(&mut self) {
        self.build_index();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::NodeType;

    #[test]
    fn test_dom_tree_creation() {
        // 测试 DOM 树创建
        let root = DOMNode::new_element("div");
        let tree = DOMTree::new(root);

        assert_eq!(tree.root.tag_name(), Some("div"));
    }

    #[test]
    fn test_insert_after() {
        // 测试在节点后插入
        let mut root = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        root.append_child(child1);

        let mut tree = DOMTree::new(root);
        let child1_id = tree.root.children[0].id;
        let child2 = DOMNode::new_element("p");

        let success = tree.insert_after(child1_id, child2);

        assert!(success);
        assert_eq!(tree.root.children.len(), 2);
        assert!(matches!(&tree.root.children[1].node_type, NodeType::Element(tag) if tag == "p"));
    }

    #[test]
    fn test_insert_before_node() {
        // 测试在节点前插入
        let mut root = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        root.append_child(child1);

        let mut tree = DOMTree::new(root);
        let child1_id = tree.root.children[0].id;
        let child2 = DOMNode::new_element("p");

        let success = tree.insert_before_node(child1_id, child2);

        assert!(success);
        assert_eq!(tree.root.children.len(), 2);
        assert!(matches!(&tree.root.children[0].node_type, NodeType::Element(tag) if tag == "p"));
    }

    #[test]
    fn test_remove_node() {
        // 测试移除节点
        let mut root = DOMNode::new_element("div");
        let child = DOMNode::new_element("span");
        root.append_child(child);

        let mut tree = DOMTree::new(root);
        let child_id = tree.root.children[0].id;

        let removed = tree.remove_node(child_id);

        assert!(removed.is_some());
        assert_eq!(tree.root.children.len(), 0);
    }

    #[test]
    fn test_compare_document_position_same() {
        // 测试同一节点
        let root = DOMNode::new_element("div");
        let tree = DOMTree::new(root);
        let root_id = tree.root.id;

        assert_eq!(tree.compare_document_position(root_id, root_id), 0);
    }

    #[test]
    fn test_compare_document_position_contains() {
        // 测试包含关系
        let mut root = DOMNode::new_element("div");
        let child = DOMNode::new_element("span");
        root.append_child(child);

        let tree = DOMTree::new(root);
        let root_id = tree.root.id;
        let child_id = tree.root.children[0].id;

        assert_eq!(tree.compare_document_position(root_id, child_id), 4);
    }

    #[test]
    fn test_compare_document_position_before() {
        // 测试前后关系
        let mut root = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        let child2 = DOMNode::new_element("p");
        root.append_child(child1);
        root.append_child(child2);

        let tree = DOMTree::new(root);
        let child1_id = tree.root.children[0].id;
        let child2_id = tree.root.children[1].id;

        assert_eq!(tree.compare_document_position(child1_id, child2_id), 1);
        assert_eq!(tree.compare_document_position(child2_id, child1_id), 2);
    }

    #[test]
    fn test_get_depth() {
        // 测试获取深度
        let mut root = DOMNode::new_element("div");
        let child = DOMNode::new_element("span");
        root.append_child(child);

        let tree = DOMTree::new(root);
        let root_id = tree.root.id;
        let child_id = tree.root.children[0].id;

        assert_eq!(tree.get_depth(root_id), Some(0));
        assert_eq!(tree.get_depth(child_id), Some(1));
    }

    #[test]
    fn test_get_ancestors() {
        // 测试获取祖先节点
        let mut root = DOMNode::new_element("div");
        let mut child = DOMNode::new_element("span");
        let grandchild = DOMNode::new_element("p");
        child.append_child(grandchild);
        root.append_child(child);

        let tree = DOMTree::new(root);
        let grandchild_id = tree.root.children[0].children[0].id;

        let ancestors = tree.get_ancestors(grandchild_id);
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&tree.root.id));
    }

    #[test]
    fn test_get_sibling_count_and_index() {
        // 测试兄弟节点信息
        let mut root = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("span");
        let child2 = DOMNode::new_element("p");
        root.append_child(child1);
        root.append_child(child2);

        let tree = DOMTree::new(root);
        let child1_id = tree.root.children[0].id;
        let child2_id = tree.root.children[1].id;

        assert_eq!(tree.get_sibling_count(child1_id), 2);
        assert_eq!(tree.get_sibling_index(child1_id), Some(0));
        assert_eq!(tree.get_sibling_index(child2_id), Some(1));
    }

    #[test]
    fn test_node_index_maintenance() {
        // 测试索引维护
        let mut root = DOMNode::new_element("div");
        let child = DOMNode::new_element("span");
        root.append_child(child);

        let mut tree = DOMTree::new(root);
        let child_id = tree.root.children[0].id;

        // 验证索引存在
        assert!(tree.node_index.contains_key(&child_id));

        // 移除节点后索引应该更新
        tree.remove_node(child_id);
        assert!(!tree.node_index.contains_key(&child_id));
    }
}
