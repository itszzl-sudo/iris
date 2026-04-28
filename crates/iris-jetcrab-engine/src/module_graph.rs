//! 模块依赖图
//!
//! 管理 Vue 项目中的模块依赖关系，支持循环依赖检测

use std::collections::{HashMap, HashSet};
use tracing::debug;

/// 模块依赖图
pub struct ModuleGraph {
    /// 模块列表 key: 模块路径, value: 依赖列表
    modules: HashMap<String, Vec<String>>,
}

impl ModuleGraph {
    /// 创建新的模块图
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// 添加模块
    pub fn add_module(&mut self, module_path: String, dependencies: Vec<String>) {
        debug!(
            "Adding module: {} with {} dependencies",
            module_path,
            dependencies.len()
        );
        
        self.modules.insert(module_path, dependencies);
    }

    /// 获取模块数量
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// 检测循环依赖
    ///
    /// 返回所有检测到的循环依赖路径
    pub fn detect_cycles(&self) -> Option<Vec<Vec<String>>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut path = Vec::new();

        for module in self.modules.keys() {
            if !visited.contains(module) {
                self.dfs_detect_cycles(
                    module,
                    &mut visited,
                    &mut recursion_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        if cycles.is_empty() {
            None
        } else {
            Some(cycles)
        }
    }

    /// DFS 检测循环依赖
    fn dfs_detect_cycles(
        &self,
        module: &str,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(module.to_string());
        recursion_stack.insert(module.to_string());
        path.push(module.to_string());

        if let Some(dependencies) = self.modules.get(module) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    self.dfs_detect_cycles(dep, visited, recursion_stack, path, cycles);
                } else if recursion_stack.contains(dep) {
                    // 找到循环依赖
                    let cycle_start = path.iter().position(|p| p == dep).unwrap();
                    let cycle = path[cycle_start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }

        path.pop();
        recursion_stack.remove(module);
    }

    /// 获取模块的依赖
    pub fn get_dependencies(&self, module_path: &str) -> Option<&Vec<String>> {
        self.modules.get(module_path)
    }

    /// 获取所有模块
    pub fn get_all_modules(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    /// 获取模块的拓扑排序
    ///
    /// 如果存在循环依赖，返回错误
    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut result = Vec::new();

        for module in self.modules.keys() {
            if !visited.contains(module) {
                self.dfs_topological_sort(module, &mut visited, &mut stack, &mut result)?;
            }
        }

        result.reverse();
        Ok(result)
    }

    /// DFS 拓扑排序
    fn dfs_topological_sort(
        &self,
        module: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), String> {
        visited.insert(module.to_string());
        stack.insert(module.to_string());

        if let Some(dependencies) = self.modules.get(module) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    self.dfs_topological_sort(dep, visited, stack, result)?;
                } else if stack.contains(dep) {
                    return Err(format!("Circular dependency detected: {} -> {}", module, dep));
                }
            }
        }

        stack.remove(module);
        result.push(module.to_string());

        Ok(())
    }
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_module() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A.vue".to_string(), vec!["B.vue".to_string()]);
        
        assert_eq!(graph.len(), 1);
        assert_eq!(graph.get_dependencies("A.vue").unwrap().len(), 1);
    }

    #[test]
    fn test_detect_no_cycles() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A.vue".to_string(), vec!["B.vue".to_string()]);
        graph.add_module("B.vue".to_string(), vec!["C.vue".to_string()]);
        graph.add_module("C.vue".to_string(), vec![]);

        assert!(graph.detect_cycles().is_none());
    }

    #[test]
    fn test_detect_cycles() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A.vue".to_string(), vec!["B.vue".to_string()]);
        graph.add_module("B.vue".to_string(), vec!["C.vue".to_string()]);
        graph.add_module("C.vue".to_string(), vec!["A.vue".to_string()]);

        let cycles = graph.detect_cycles();
        assert!(cycles.is_some());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A.vue".to_string(), vec!["B.vue".to_string()]);
        graph.add_module("B.vue".to_string(), vec!["C.vue".to_string()]);
        graph.add_module("C.vue".to_string(), vec![]);

        let sorted = graph.topological_sort().unwrap();
        println!("Sorted order: {:?}", sorted);
        
        // 拓扑排序保证依赖先出现，所以顺序应该是 A, B, C
        // A 依赖 B，所以 A 在 B 前
        // B 依赖 C，所以 B 在 C 前
        let a_pos = sorted.iter().position(|m| m == "A.vue").unwrap();
        let b_pos = sorted.iter().position(|m| m == "B.vue").unwrap();
        let c_pos = sorted.iter().position(|m| m == "C.vue").unwrap();
        
        assert!(a_pos < b_pos, "A should come before B");
        assert!(b_pos < c_pos, "B should come before C");
    }

    #[test]
    fn test_topological_sort_with_cycle() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A.vue".to_string(), vec!["B.vue".to_string()]);
        graph.add_module("B.vue".to_string(), vec!["A.vue".to_string()]);

        let result = graph.topological_sort();
        assert!(result.is_err());
    }
}
