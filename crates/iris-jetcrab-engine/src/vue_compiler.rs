//! Vue 项目编译器
//!
//! 从 App.vue 开始反向解析依赖，按依赖顺序编译所有模块

use anyhow::{Result, Context};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use tracing::{info, debug, warn, error};

use crate::sfc_compiler::{self, CompiledModule, resolve_module};

/// 编译结果
#[derive(Debug, Clone)]
pub struct CompilationResult {
    /// 已编译的模块 key: 模块路径, value: 编译结果
    pub compiled_modules: HashMap<String, CompiledModule>,
    /// 编译顺序（从叶子到根）
    pub compilation_order: Vec<String>,
    /// 入口文件
    pub entry_file: String,
}

/// Vue 项目编译器
pub struct VueProjectCompiler {
    /// 项目根目录
    project_root: PathBuf,
    /// 已编译的模块缓存
    compiled_cache: HashMap<String, CompiledModule>,
    /// 正在编译的模块（用于检测循环依赖）
    compiling: HashSet<String>,
    /// 编译完成的模块
    compiled: HashSet<String>,
}

impl VueProjectCompiler {
    /// 创建新的编译器实例
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            compiled_cache: HashMap::new(),
            compiling: HashSet::new(),
            compiled: HashSet::new(),
        }
    }

    /// 编译整个 Vue 项目
    ///
    /// 从入口文件（App.vue）开始，反向解析依赖并按顺序编译
    pub async fn compile_project(&mut self, entry_file: &str) -> Result<CompilationResult> {
        info!("Compiling Vue project from entry: {}", entry_file);

        // 1. 解析入口文件的完整路径
        let entry_path = self.resolve_path(entry_file)?;
        debug!("Entry file path: {:?}", entry_path);

        // 2. 从入口文件开始，递归构建依赖图
        let dependency_graph = self.build_dependency_graph(&entry_path)?;
        info!("Dependency graph built with {} modules", dependency_graph.len());

        // 3. 拓扑排序（确保依赖先编译）
        let compilation_order = self.topological_sort(&dependency_graph)?;
        debug!("Compilation order: {:?}", compilation_order);

        // 4. 按顺序编译所有模块
        let mut compiled_modules = HashMap::new();
        for module_path in &compilation_order {
            let compiled = self.compile_single_module(module_path)?;
            compiled_modules.insert(module_path.clone(), compiled);
        }

        info!(
            "Project compilation complete: {} modules compiled",
            compiled_modules.len()
        );

        Ok(CompilationResult {
            compiled_modules,
            compilation_order,
            entry_file: entry_path.to_string_lossy().to_string(),
        })
    }

    /// 从入口文件开始，递归构建依赖图
    fn build_dependency_graph(&mut self, entry_path: &Path) -> Result<HashMap<String, Vec<String>>> {
        let mut graph = HashMap::new();
        let mut visited = HashSet::new();

        self.dfs_build_graph(entry_path, &mut graph, &mut visited)?;

        Ok(graph)
    }

    /// DFS 构建依赖图
    fn dfs_build_graph(
        &mut self,
        module_path: &Path,
        graph: &mut HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        let module_key = module_path.to_string_lossy().to_string();

        // 如果已经访问过，跳过
        if visited.contains(&module_key) {
            debug!("Module already visited: {}", module_key);
            return Ok(());
        }

        visited.insert(module_key.clone());
        debug!("Building dependencies for: {}", module_key);

        // 读取文件内容
        let content = std::fs::read_to_string(module_path)
            .context(format!("Failed to read file: {:?}", module_path))?;

        // 解析依赖
        let dependencies = if module_path.extension().and_then(|e| e.to_str()) == Some("vue") {
            // Vue SFC 文件
            self.parse_vue_dependencies(&content, module_path)?
        } else {
            // JavaScript/TypeScript 文件
            self.parse_js_dependencies(&content, module_path)?
        };

        // 添加到图中
        graph.insert(module_key.clone(), dependencies.clone());

        // 递归处理依赖
        for dep_path_str in &dependencies {
            // 解析依赖的完整路径
            if let Ok(dep_path) = self.resolve_dependency(dep_path_str, module_path) {
                // 检查文件是否存在
                if dep_path.exists() {
                    self.dfs_build_graph(&dep_path, graph, visited)?;
                } else {
                    warn!("Dependency not found: {} (imported from {})", dep_path_str, module_key);
                }
            }
        }

        Ok(())
    }

    /// 解析 Vue 文件的依赖
    fn parse_vue_dependencies(&self, content: &str, current_path: &Path) -> Result<Vec<String>> {
        // 使用 iris-sfc 编译获取 script 部分
        let compiled = iris_sfc::compile_from_string(
            &current_path.to_string_lossy(),
            content
        )?;

        // 从 script 中解析 import 语句
        self.extract_imports(&compiled.script, current_path)
    }

    /// 解析 JS/TS 文件的依赖
    fn parse_js_dependencies(&self, content: &str, current_path: &Path) -> Result<Vec<String>> {
        self.extract_imports(content, current_path)
    }

    /// 从 JavaScript 代码中提取 import 语句
    fn extract_imports(&self, script: &str, current_path: &Path) -> Result<Vec<String>> {
        let mut imports = Vec::new();

        for line in script.lines() {
            let line = line.trim();

            // 静态导入: import ... from '...'
            if line.starts_with("import ") && line.contains(" from ") {
                if let Some(dep) = self.extract_string_from_import(line) {
                    imports.push(dep);
                }
            }

            // 动态导入: import('...')
            if line.contains("import(") {
                if let Some(start) = line.find("import('") {
                    if let Some(end) = line[start + 8..].find('\'') {
                        let dep = &line[start + 8..start + 8 + end];
                        imports.push(dep.to_string());
                    }
                }
            }

            // CommonJS: require('...')
            if line.contains("require(") {
                if let Some(start) = line.find("require('") {
                    if let Some(end) = line[start + 9..].find('\'') {
                        let dep = &line[start + 9..start + 9 + end];
                        imports.push(dep.to_string());
                    }
                }
            }
        }

        Ok(imports)
    }

    /// 从 import 语句中提取路径
    fn extract_string_from_import(&self, line: &str) -> Option<String> {
        // 尝试单引号
        if let (Some(start), Some(end)) = (line.find("from '"), line.rfind('\'')) {
            if start < end {
                return Some(line[start + 6..end].to_string());
            }
        }

        // 尝试双引号
        if let (Some(start), Some(end)) = (line.find("from \""), line.rfind('"')) {
            if start < end {
                return Some(line[start + 6..end].to_string());
            }
        }

        None
    }

    /// 解析依赖路径
    fn resolve_dependency(&self, dep_path: &str, importer: &Path) -> Result<PathBuf> {
        // 使用 sfc_compiler 的 resolve_module
        let resolved_str = resolve_module(dep_path, &importer.to_string_lossy())?;
        
        // 转换为绝对路径
        self.resolve_path(&resolved_str)
    }

    /// 解析文件路径（相对于项目根目录）
    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        let path = path.trim_start_matches("./");
        
        // 如果是绝对路径
        if path.starts_with('/') {
            Ok(self.project_root.join(&path[1..]))
        } else {
            Ok(self.project_root.join(path))
        }
    }

    /// 拓扑排序（确保依赖先于使用者出现）
    fn topological_sort(&self, graph: &HashMap<String, Vec<String>>) -> Result<Vec<String>> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut result = Vec::new();

        // 找到入口文件（没有依赖其他文件的，或者被其他文件依赖最多的）
        let entry = graph.keys().next()
            .ok_or_else(|| anyhow::anyhow!("Empty dependency graph"))?
            .clone();

        self.dfs_topo_sort(&entry, graph, &mut visited, &mut stack, &mut result)?;

        // 反转结果，使依赖先出现
        result.reverse();
        Ok(result)
    }

    /// DFS 拓扑排序
    fn dfs_topo_sort(
        &self,
        module: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<()> {
        if visited.contains(module) {
            return Ok(());
        }

        if stack.contains(module) {
            warn!("Circular dependency detected: {}", module);
            return Ok(()); // 跳过循环依赖
        }

        stack.insert(module.to_string());

        // 先处理依赖
        if let Some(dependencies) = graph.get(module) {
            for dep in dependencies {
                // 只处理在图中的模块（忽略外部依赖如 'vue'）
                if graph.contains_key(dep) {
                    self.dfs_topo_sort(dep, graph, visited, stack, result)?;
                }
            }
        }

        stack.remove(module);
        visited.insert(module.to_string());
        result.push(module.to_string());

        Ok(())
    }

    /// 编译单个模块
    fn compile_single_module(&mut self, module_path: &str) -> Result<CompiledModule> {
        // 检查缓存
        if let Some(cached) = self.compiled_cache.get(module_path) {
            debug!("Using cached module: {}", module_path);
            return Ok(cached.clone());
        }

        debug!("Compiling module: {}", module_path);

        // 读取文件内容
        let content = std::fs::read_to_string(module_path)
            .context(format!("Failed to read file: {}", module_path))?;

        // 编译
        let compiled = if module_path.ends_with(".vue") {
            sfc_compiler::compile_sfc(&content, module_path)?
        } else {
            // JS/TS 文件直接作为 script
            CompiledModule {
                script: content,
                styles: vec![],
                deps: vec![],
            }
        };

        // 缓存
        self.compiled_cache.insert(module_path.to_string(), compiled.clone());
        self.compiled.insert(module_path.to_string());

        Ok(compiled)
    }

    /// 获取编译缓存统计
    pub fn get_cache_stats(&self) -> (usize, usize) {
        (self.compiled_cache.len(), self.compiled.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_imports() {
        let script = r#"
            import { ref } from 'vue';
            import Foo from './components/Foo.vue';
            import Bar from "../Bar.vue";
            const module = await import('./lazy.vue');
            const req = require('./common.js');
        "#;

        let compiler = VueProjectCompiler::new(PathBuf::from("."));
        let imports = compiler.extract_imports(script, Path::new("App.vue")).unwrap();

        assert!(imports.contains(&"vue".to_string()));
        assert!(imports.contains(&"./components/Foo.vue".to_string()));
        assert!(imports.contains(&"../Bar.vue".to_string()));
        assert!(imports.contains(&"./lazy.vue".to_string()));
        assert!(imports.contains(&"./common.js".to_string()));
    }

    #[test]
    fn test_resolve_path() {
        let compiler = VueProjectCompiler::new(PathBuf::from("/project"));

        // 相对路径
        let resolved = compiler.resolve_path("src/App.vue").unwrap();
        assert_eq!(resolved, PathBuf::from("/project/src/App.vue"));

        // 带 ./ 的路径
        let resolved = compiler.resolve_path("./src/App.vue").unwrap();
        assert_eq!(resolved, PathBuf::from("/project/src/App.vue"));

        // 绝对路径（相对于项目根）
        let resolved = compiler.resolve_path("/src/App.vue").unwrap();
        assert_eq!(resolved, PathBuf::from("/project/src/App.vue"));
    }
}
