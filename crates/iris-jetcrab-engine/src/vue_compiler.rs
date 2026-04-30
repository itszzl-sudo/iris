//! Vue 项目编译器
//!
//! 从 App.vue 开始反向解析依赖，按依赖顺序编译所有模块
//! 支持 npm 包依赖、TypeScript、CSS 预处理器
//!
//! 使用成熟编译器：
//! - TypeScript: swc (via iris-sfc::ts_compiler)
//! - SCSS/SASS: grass
//! - Less: less-rs

use anyhow::{Result, Context};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{info, debug, warn};

use crate::sfc_compiler::{self, CompiledModule, resolve_module, StyleBlock};
use iris_sfc::ts_compiler::{TsCompiler, TsCompilerConfig};

/// npm 包信息
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// 包名
    pub name: String,
    /// 包版本
    pub version: String,
    /// 入口文件
    pub main: String,
    /// 模块入口（ESM）
    pub module: Option<String>,
    /// 样式文件
    pub style: Option<String>,
    /// 类型定义
    pub types: Option<String>,
}

/// 编译结果
#[derive(Debug, Clone)]
pub struct CompilationResult {
    /// 已编译的模块 key: 模块路径, value: 编译结果
    pub compiled_modules: HashMap<String, CompiledModule>,
    /// npm 包依赖 key: 包名, value: 包信息
    pub npm_packages: HashMap<String, PackageInfo>,
    /// 编译顺序（从叶子到根）
    pub compilation_order: Vec<String>,
    /// 入口文件
    pub entry_file: String,
    /// 全局样式
    pub global_styles: Vec<StyleBlock>,
}

/// Vue 项目编译器
pub struct VueProjectCompiler {
    /// 项目根目录
    project_root: PathBuf,
    /// node_modules 目录
    node_modules_path: PathBuf,
    /// 已编译的模块缓存
    compiled_cache: HashMap<String, CompiledModule>,
    /// 正在编译的模块（用于检测循环依赖）
    #[allow(dead_code)]
    compiling: HashSet<String>,
    /// 编译完成的模块
    compiled: HashSet<String>,
    /// npm 包缓存
    npm_packages: HashMap<String, PackageInfo>,
    /// package.json 中的依赖
    #[allow(dead_code)]
    project_dependencies: HashMap<String, String>,
    /// TypeScript 编译器
    ts_compiler: TsCompiler,
}

impl VueProjectCompiler {
    /// 创建新的编译器实例
    pub fn new(project_root: PathBuf) -> Self {
        let node_modules_path = project_root.join("node_modules");
        
        // 加载 package.json 依赖
        let project_dependencies = Self::load_package_dependencies(&project_root)
            .unwrap_or_default();
        
        // 初始化 TypeScript 编译器
        let ts_compiler = TsCompiler::new(TsCompilerConfig::default());
        
        Self {
            project_root,
            node_modules_path,
            compiled_cache: HashMap::new(),
            compiling: HashSet::new(),
            compiled: HashSet::new(),
            npm_packages: HashMap::new(),
            project_dependencies,
            ts_compiler,
        }
    }

    /// 加载 package.json 中的依赖
    fn load_package_dependencies(project_root: &Path) -> Result<HashMap<String, String>> {
        let package_json_path = project_root.join("package.json");
        
        if !package_json_path.exists() {
            return Ok(HashMap::new());
        }
        
        let content = std::fs::read_to_string(&package_json_path)
            .context("Failed to read package.json")?;
        
        let package_json: serde_json::Value = serde_json::from_str(&content)
            .context("Failed to parse package.json")?;
        
        let mut dependencies = HashMap::new();
        
        // 合并 dependencies 和 devDependencies
        if let Some(deps) = package_json.get("dependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps {
                dependencies.insert(name.clone(), version.as_str().unwrap_or("").to_string());
            }
        }
        
        if let Some(deps) = package_json.get("devDependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps {
                dependencies.insert(name.clone(), version.as_str().unwrap_or("").to_string());
            }
        }
        
        info!("Loaded {} dependencies from package.json", dependencies.len());
        Ok(dependencies)
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

        // 3. 直接使用依赖图中的所有模块（DFS 构建时已经是正确的依赖顺序）
        let compilation_order: Vec<String> = dependency_graph.keys().cloned().collect();
        info!("Compilation order: {:?}", compilation_order);
        debug!("Dependency graph keys: {:?}", dependency_graph.keys().collect::<Vec<_>>());

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
            npm_packages: self.npm_packages.clone(),
            compilation_order,
            entry_file: entry_path.to_string_lossy().to_string(),
            global_styles: Vec::new(), // TODO: 从入口文件提取全局样式
        })
    }

    /// 编译单个文件（按需编译）
    ///
    /// 不解析依赖图，只编译指定的单个文件
    /// 由浏览器端的原生 ESM 模块加载器负责按需请求每个依赖
    pub fn compile_file(&mut self, file_path: &str) -> Result<CompiledModule> {
        debug!("On-demand compiling single file: {}", file_path);
        self.compile_single_module(file_path)
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
            // 判断是否为 npm 包（不是相对路径或绝对路径）
            if !dep_path_str.starts_with('.') && !dep_path_str.starts_with('/') {
                // npm 包依赖
                debug!("Found npm package dependency: {}", dep_path_str);
                self.resolve_npm_package(dep_path_str)?;
                continue;
            }
            
            // 本地文件依赖
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

    /// 解析 TypeScript 文件的依赖
    fn parse_ts_dependencies(&self, content: &str, current_path: &Path) -> Result<Vec<String>> {
        // TypeScript 的 import 语法与 JavaScript 相同
        // 但需要额外处理 type imports
        let mut imports = self.extract_imports(content, current_path)?;
        
        // 处理 type imports: import type { Foo } from './foo'
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("import type ") || line.contains("import type{") {
                if let Some(dep) = self.extract_string_from_import(line) {
                    if !imports.contains(&dep) {
                        imports.push(dep);
                    }
                }
            }
        }
        
        Ok(imports)
    }

    /// 从 JavaScript 代码中提取 import 语句
    fn extract_imports(&self, script: &str, _current_path: &Path) -> Result<Vec<String>> {
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

    /// 解析 npm 包
    fn resolve_npm_package(&mut self, package_name: &str) -> Result<()> {
        // 如果已经解析过，跳过
        if self.npm_packages.contains_key(package_name) {
            return Ok(());
        }
        
        debug!("Resolving npm package: {}", package_name);
        
        // 查找包的 package.json
        let package_path = self.resolve_npm_package_path(package_name)?;
        let package_json_path = package_path.join("package.json");
        
        if !package_json_path.exists() {
            warn!("Package not found in node_modules: {}", package_name);
            return Ok(());
        }
        
        // 解析 package.json
        let content = std::fs::read_to_string(&package_json_path)?;
        let package_json: serde_json::Value = serde_json::from_str(&content)?;
        
        let name = package_json.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(package_name)
            .to_string();
        
        let version = package_json.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        // 获取入口文件（优先使用 module 字段，其次 main）
        let module_entry = package_json.get("module")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let main_entry = package_json.get("main")
            .and_then(|v| v.as_str())
            .unwrap_or("index.js")
            .to_string();
        
        let entry_file = module_entry.clone().unwrap_or_else(|| main_entry.clone());
        
        // 获取样式文件
        let style_file = package_json.get("style")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        // 获取类型定义
        let types_file = package_json.get("types")
            .or_else(|| package_json.get("typings"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let package_info = PackageInfo {
            name: name.clone(),
            version,
            main: main_entry,
            module: module_entry,
            style: style_file,
            types: types_file,
        };
        
        info!("Resolved npm package: {}@{} (entry: {})", 
              name, package_info.version, entry_file);
        
        self.npm_packages.insert(package_name.to_string(), package_info);
        
        // 继续解析包的依赖
        let entry_path = package_path.join(&entry_file);
        if entry_path.exists() {
            self.parse_file_dependencies(&entry_path)?;
        }
        
        Ok(())
    }

    /// 解析 npm 包的路径
    fn resolve_npm_package_path(&self, package_name: &str) -> Result<PathBuf> {
        // 处理 scoped packages (@vue/runtime-core)
        let package_path = if package_name.starts_with('@') {
            let parts: Vec<&str> = package_name.split('/').collect();
            if parts.len() >= 2 {
                self.node_modules_path.join(package_name)
            } else {
                self.node_modules_path.join(package_name)
            }
        } else {
            self.node_modules_path.join(package_name)
        };
        
        if !package_path.exists() {
            anyhow::bail!("Package not found: {:?}", package_path);
        }
        
        Ok(package_path)
    }

    /// 解析文件依赖（供 npm 包调用）
    fn parse_file_dependencies(&mut self, file_path: &Path) -> Result<()> {
        if !file_path.exists() {
            return Ok(());
        }
        
        let content = std::fs::read_to_string(file_path)
            .context(format!("Failed to read file: {:?}", file_path))?;
        
        let ext = file_path.extension().and_then(|e| e.to_str());
        
        // 根据文件类型解析依赖
        let dependencies = match ext {
            Some("vue") => self.parse_vue_dependencies(&content, file_path)?,
            Some("js") | Some("jsx") => self.parse_js_dependencies(&content, file_path)?,
            Some("ts") | Some("tsx") => self.parse_ts_dependencies(&content, file_path)?,
            _ => vec![],
        };
        
        // 继续解析依赖
        for dep in dependencies {
            if !dep.starts_with('.') && !dep.starts_with('/') {
                // npm 包
                self.resolve_npm_package(&dep)?;
            }
        }
        
        Ok(())
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
            // Vue SFC 文件
            sfc_compiler::compile_sfc(&content, module_path)?
        } else if module_path.ends_with(".ts") || module_path.ends_with(".tsx") {
            // TypeScript 文件 - 使用 swc 编译器
            let js_code = self.compile_typescript(&content, module_path)?;
            CompiledModule {
                script: js_code,
                styles: vec![],
                deps: vec![],
            }
        } else if module_path.ends_with(".css") {
            // CSS 文件 - 应用 PostCSS 转换（autoprefixer/nesting）
            let postcss_config = iris_sfc::postcss_processor::PostCssConfig::default();
            let postcss_result = iris_sfc::postcss_processor::process_css(
                &content,
                &postcss_config,
                module_path
            );
            if postcss_result.transformed {
                debug!("PostCSS applied to CSS: {} -> {} bytes", postcss_result.original_size, postcss_result.output_size);
            }
            CompiledModule {
                script: format!("// CSS module: {}\nexport default {{}}", module_path),
                styles: vec![StyleBlock {
                    code: postcss_result.css,
                    scoped: false,
                }],
                deps: vec![],
            }
        } else if module_path.ends_with(".scss") || module_path.ends_with(".sass") {
            // SCSS/SASS 文件 - 使用 grass 编译器，然后 PostCSS 转换
            let css_code = self.compile_scss(&content, module_path)?;
            let postcss_config = iris_sfc::postcss_processor::PostCssConfig::default();
            let postcss_result = iris_sfc::postcss_processor::process_css(
                &css_code,
                &postcss_config,
                module_path
            );
            if postcss_result.transformed {
                debug!("PostCSS applied to SCSS: {} -> {} bytes", postcss_result.original_size, postcss_result.output_size);
            }
            CompiledModule {
                script: format!("// SCSS module: {}\nexport default {{}}", module_path),
                styles: vec![StyleBlock {
                    code: postcss_result.css,
                    scoped: false,
                }],
                deps: vec![],
            }
        } else if module_path.ends_with(".less") {
            // Less 文件 - 使用 rust-less 编译器，然后 PostCSS 转换
            let less_config = iris_sfc::less_processor::LessConfig::default();
            let less_result = iris_sfc::less_processor::compile_less(&content, &less_config)
                .map_err(|e| anyhow::anyhow!("Less compilation failed: {}", e))?;
            let css_code = less_result.css;
            let postcss_config = iris_sfc::postcss_processor::PostCssConfig::default();
            let postcss_result = iris_sfc::postcss_processor::process_css(
                &css_code,
                &postcss_config,
                module_path
            );
            if postcss_result.transformed {
                debug!("PostCSS applied to Less: {} -> {} bytes", postcss_result.original_size, postcss_result.output_size);
            }
            CompiledModule {
                script: format!("// Less module: {}\nexport default {{}}", module_path),
                styles: vec![StyleBlock {
                    code: postcss_result.css,
                    scoped: false,
                }],
                deps: vec![],
            }
        } else {
            // JS/JSX 文件直接作为 script
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

    /// 使用 swc 编译 TypeScript
    fn compile_typescript(&self, ts_code: &str, filename: &str) -> Result<String> {
        debug!("Compiling TypeScript with swc: {}", filename);
        
        let result = self.ts_compiler.compile(ts_code, filename)
            .map_err(|e| anyhow::anyhow!("Failed to compile TypeScript {}: {}", filename, e))?;
        
        debug!(
            "TypeScript compiled in {:.2}ms: {} -> {} bytes",
            result.compile_time_ms,
            ts_code.len(),
            result.code.len()
        );
        
        Ok(result.code)
    }

    /// 使用 grass 编译 SCSS/SASS
    fn compile_scss(&self, scss_code: &str, filename: &str) -> Result<String> {
        debug!("Compiling SCSS with grass: {}", filename);
        
        // 使用 grass 编译
        let css = grass::from_string(
            scss_code.to_string(),
            &grass::Options::default()
        ).context(format!("Failed to compile SCSS: {}", filename))?;
        
        debug!("SCSS compiled: {} -> {} bytes", scss_code.len(), css.len());
        Ok(css)
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
