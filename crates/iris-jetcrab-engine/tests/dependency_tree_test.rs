//! 依赖树管理测试

use iris_jetcrab_engine::dependency_tree::{DependencyTree, DependencyInfo};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_dependency_tree_creation() {
        // 这个测试需要一个真实的 Vue 项目
        // 这里只是验证结构体创建
        let dep_tree = DependencyTree {
            project_root: std::path::PathBuf::from("/tmp/test"),
            dependencies: HashMap::new(),
            runtime_dependencies: HashMap::new(),
            dependency_hash: String::from("test"),
        };

        assert_eq!(dep_tree.dependencies.len(), 0);
    }

    #[test]
    fn test_is_build_tool() {
        // 测试编译工具检测
        assert!(DependencyTree::is_build_tool("vite"));
        assert!(DependencyTree::is_build_tool("webpack"));
        assert!(DependencyTree::is_build_tool("@babel/core"));
        assert!(DependencyTree::is_build_tool("typescript"));
        
        // 测试运行时依赖
        assert!(!DependencyTree::is_build_tool("vue"));
        assert!(!DependencyTree::is_build_tool("axios"));
        assert!(!DependencyTree::is_build_tool("lodash"));
    }

    #[test]
    fn test_dependency_hash() {
        let mut deps1 = HashMap::new();
        let mut deps2 = HashMap::new();

        // 相同的依赖应该产生相同的哈希
        let dep1 = DependencyInfo {
            name: "vue".to_string(),
            version_req: "^3.0.0".to_string(),
            installed_version: Some("3.4.0".to_string()),
            is_dev_dependency: false,
            is_build_tool: false,
            package_path: None,
            dependencies: vec![],
        };

        deps1.insert("vue".to_string(), dep1.clone());
        deps2.insert("vue".to_string(), dep1);

        let hash1 = DependencyTree::calculate_hash(&deps1);
        let hash2 = DependencyTree::calculate_hash(&deps2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_dependency_change_detection() {
        let mut old_deps = HashMap::new();
        let mut new_deps = HashMap::new();

        let old_vue = DependencyInfo {
            name: "vue".to_string(),
            version_req: "^3.0.0".to_string(),
            installed_version: Some("3.3.0".to_string()),
            is_dev_dependency: false,
            is_build_tool: false,
            package_path: None,
            dependencies: vec![],
        };

        let new_vue = DependencyInfo {
            name: "vue".to_string(),
            version_req: "^3.0.0".to_string(),
            installed_version: Some("3.4.0".to_string()),
            is_dev_dependency: false,
            is_build_tool: false,
            package_path: None,
            dependencies: vec![],
        };

        old_deps.insert("vue".to_string(), old_vue);
        new_deps.insert("vue".to_string(), new_vue);

        let old_tree = DependencyTree {
            project_root: std::path::PathBuf::from("/tmp/test"),
            dependencies: old_deps,
            runtime_dependencies: HashMap::new(),
            dependency_hash: String::from("old"),
        };

        let new_tree = DependencyTree {
            project_root: std::path::PathBuf::from("/tmp/test"),
            dependencies: new_deps,
            runtime_dependencies: HashMap::new(),
            dependency_hash: String::from("new"),
        };

        // 检测变化
        assert!(old_tree.has_changed(&new_tree));
        
        let changes = old_tree.get_changed_dependencies(&new_tree);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].name, "vue");
    }
}
