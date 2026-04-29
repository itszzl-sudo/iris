//! 依赖扫描器
//!
//! 在 dev server 启动时扫描项目源码，识别以下问题：
//! 1. npm 包在源码中使用但未在 package.json 中声明
//! 2. 本地 SFC/JS/TS 文件引用但不存在
//! 3. CSS/SCSS 文件引用但不存在
//! 4. 图片等静态资源文件引用但不存在

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use anyhow::Result;
use tracing::{info, debug};
use serde::{Serialize, Deserialize};

/// 依赖问题类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    /// npm 包未在 package.json 中声明
    MissingNpmPackage,
    /// 本地 SFC/JS/TS 文件不存在
    MissingLocalFile,
    /// CSS/SCSS/LESS 文件不存在
    MissingCssFile,
    /// 图片等静态资源不存在
    MissingAsset,
}

/// 依赖问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyIssue {
    /// 问题类型
    pub issue_type: IssueType,
    /// import 语句中引用的路径/包名
    pub import_path: String,
    /// 在哪个源文件中引用
    pub source_file: String,
    /// 源文件中的行号（近似）
    pub source_line: Option<usize>,
    /// 问题描述
    pub description: String,
    /// 建议的解决方案
    pub solution: String,
    /// 严重级别: error, warning, info
    pub severity: String,
    /// 是否可以自动修复
    pub can_auto_fix: bool,
}

/// 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// 发现的问题列表
    pub issues: Vec<DependencyIssue>,
    /// 项目中的 npm 包声明
    pub declared_packages: HashMap<String, String>,
    /// 项目中已安装的 npm 包（在 node_modules 中）
    pub installed_packages: Vec<String>,
    /// node_modules 是否存在
    pub has_node_modules: bool,
    /// 项目中的源文件数量
    pub source_file_count: usize,
    /// irisResolved 中记录的解析版本
    pub iris_resolved: HashMap<String, String>,
}

/// 依赖扫描器
pub struct DependencyScanner {
    /// 项目根目录
    project_root: PathBuf,
    /// package.json 中声明的依赖（含 dependencies + devDependencies）
    declared_deps: HashMap<String, String>,
    /// irisResolved 字段中记录的已解析版本（由 iris 自动下载时写入）
    iris_resolved: HashMap<String, String>,
    /// node_modules 中已安装的包
    installed_packages: HashSet<String>,
}

impl DependencyScanner {
    /// 创建新的扫描器
    pub fn new(project_root: PathBuf) -> Self {
        let declared_deps = Self::load_declared_deps(&project_root);
        let iris_resolved = Self::load_iris_resolved(&project_root);
        let installed_packages = Self::scan_installed_packages(&project_root);
        
        Self {
            project_root,
            declared_deps,
            iris_resolved,
            installed_packages,
        }
    }

    /// 扫描整个项目，返回所有依赖问题
    pub fn scan(&self) -> ScanResult {
        info!("Scanning project dependencies...");
        
        let src_dir = self.project_root.join("src");
        let mut issues = Vec::new();
        let mut source_file_count = 0;
        
        if src_dir.exists() {
            self.scan_directory(&src_dir, &src_dir, &mut issues, &mut source_file_count);
        }
        
        info!(
            "Scan complete: {} source files, {} issues found",
            source_file_count,
            issues.len()
        );
        
        let has_node_modules = self.project_root.join("node_modules").exists();
        
        ScanResult {
            issues,
            declared_packages: self.declared_deps.clone(),
            installed_packages: self.installed_packages.iter().cloned().collect(),
            has_node_modules,
            source_file_count,
            iris_resolved: self.iris_resolved.clone(),
        }
    }

    /// 递归扫描目录
    fn scan_directory(
        &self,
        dir: &Path,
        src_root: &Path,
        issues: &mut Vec<DependencyIssue>,
        file_count: &mut usize,
    ) {
        if !dir.exists() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    // 跳过 node_modules 和隐藏目录
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str == "node_modules" || name_str.starts_with('.') {
                            continue;
                        }
                    }
                    self.scan_directory(&path, src_root, issues, file_count);
                } else if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy();
                        match ext_str.as_ref() {
                            "vue" | "js" | "jsx" | "ts" | "tsx" => {
                                *file_count += 1;
                                self.scan_imports_in_file(&path, src_root, issues);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// 扫描文件中的 import 语句
    fn scan_imports_in_file(
        &self,
        file_path: &Path,
        src_root: &Path,
        issues: &mut Vec<DependencyIssue>,
    ) {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let rel_path = file_path
            .strip_prefix(&self.project_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // 跳过注释行
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') {
                continue;
            }

            // 收集 import 语句
            if let Some(import_path) = self.extract_import_path(line) {
                self.check_import(import_path, &rel_path, line_num + 1, file_path, src_root, issues);
            }
        }
    }

    /// 从一行中提取 import 路径
    fn extract_import_path(&self, line: &str) -> Option<String> {
        // import ... from '...'
        if line.starts_with("import ") {
            // 单引号
            if let (Some(start), Some(end)) = (line.find("from '"), line.rfind('\'')) {
                if start < end {
                    return Some(line[start + 6..end].to_string());
                }
            }
            // 双引号
            if let (Some(start), Some(end)) = (line.find("from \""), line.rfind('"')) {
                if start < end {
                    return Some(line[start + 6..end].to_string());
                }
            }
            // import '...'（无 from）
            if let Some(start) = line.find("import '") {
                let start = start + 8;
                if let Some(end) = line[start..].find('\'') {
                    return Some(line[start..start + end].to_string());
                }
            }
        }
        // require('...')
        if line.contains("require(") {
            if let Some(start) = line.find("require('") {
                let start = start + 9;
                if let Some(end) = line[start..].find('\'') {
                    return Some(line[start..start + end].to_string());
                }
            }
        }
        // import('...')
        if line.contains("import(") && line.contains(" from ") == false {
            if let Some(start) = line.find("import('") {
                let start = start + 8;
                if let Some(end) = line[start..].find('\'') {
                    return Some(line[start..start + end].to_string());
                }
            }
        }
        None
    }

    /// 检查 import 路径是否存在问题
    fn check_import(
        &self,
        import_path: String,
        source_file: &str,
        line_num: usize,
        file_path: &Path,
        src_root: &Path,
        issues: &mut Vec<DependencyIssue>,
    ) {
        // 判断是否为 npm 包（不是相对路径或绝对路径）
        if !import_path.starts_with('.') && !import_path.starts_with('/') {
            self.check_npm_package(&import_path, source_file, line_num, issues);
        } else {
            self.check_local_file(&import_path, source_file, line_num, file_path, src_root, issues);
        }
    }

    /// 检查 npm 包
    fn check_npm_package(
        &self,
        package_name: &str,
        source_file: &str,
        line_num: usize,
        issues: &mut Vec<DependencyIssue>,
    ) {
        // 检查是否在 package.json 的 dependencies/devDependencies 中声明
        let is_in_deps = self.declared_deps.contains_key(package_name);
        // 检查是否在 irisResolved 中（由 iris 自动下载时记录）
        let is_in_iris_resolved = self.iris_resolved.contains_key(package_name);
        let is_declared = is_in_deps || is_in_iris_resolved;
        
        // 检查是否已安装（在 node_modules 中）
        let is_installed = self.installed_packages.contains(package_name)
            || self.is_npm_package_installed(package_name);
        
        if !is_declared {
            let version_from_iris = self.iris_resolved.get(package_name);
            let version_str = version_from_iris.map(|s| s.as_str()).unwrap_or("latest");
            
            let solution = if is_installed {
                format!(
                    "✅ 包 '{}' 已在 node_modules 中发现 ✓，无需下载。\
                     建议将其添加到 package.json 的 dependencies 中以便版本管理。",
                    package_name
                )
            } else {
                format!(
                    "📦 自动从 npm registry 下载 '{}@{}' 到 node_modules/，并写入 package.json 的 irisResolved 字段",
                    package_name, version_str
                )
            };
            
            let severity = if is_installed { "warning".to_string() } else { "error".to_string() };
            
            issues.push(DependencyIssue {
                issue_type: IssueType::MissingNpmPackage,
                import_path: package_name.to_string(),
                source_file: source_file.to_string(),
                source_line: Some(line_num),
                description: format!(
                    "npm 包 '{}' 在 '{}' 中引用，但未在 package.json 中声明",
                    package_name, source_file
                ),
                solution,
                severity,
                can_auto_fix: !is_installed,
            });
        }
    }

    /// 检查本地文件
    fn check_local_file(
        &self,
        import_path: &str,
        source_file: &str,
        line_num: usize,
        file_path: &Path,
        src_root: &Path,
        issues: &mut Vec<DependencyIssue>,
    ) {
        // 解析 import 路径为实际文件路径
        let resolved = self.resolve_import_path(import_path, file_path, src_root);
        
        match resolved {
            Some(actual_path) if actual_path.exists() => {
                // 文件存在，没问题
            }
            _ => {
                // 文件不存在
                let ext = import_path.split('.').last().unwrap_or("");
                
                let (issue_type, severity, solution) = match ext {
                    "css" | "scss" | "sass" | "less" => (
                        IssueType::MissingCssFile,
                        "warning".to_string(),
                        format!(
                            "创建空桩模块替代 '{}'（仅控制台警告，页面不崩溃）",
                            import_path
                        ),
                    ),
                    "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => (
                        IssueType::MissingAsset,
                        "warning".to_string(),
                        format!(
                            "返回透明占位图片替代 '{}'（仅控制台警告，不影响页面渲染）",
                            import_path
                        ),
                    ),
                    "vue" | "js" | "jsx" | "ts" | "tsx" | "" => (
                        IssueType::MissingLocalFile,
                        "warning".to_string(),
                        format!(
                            "生成桩模块替代 '{}'（仅显示 [Missing Component]，页面不崩溃）",
                            import_path
                        ),
                    ),
                    _ => (
                        IssueType::MissingLocalFile,
                        "info".to_string(),
                        format!("跳过缺失的文件引用 '{}'", import_path),
                    ),
                };

                issues.push(DependencyIssue {
                    issue_type,
                    import_path: import_path.to_string(),
                    source_file: source_file.to_string(),
                    source_line: Some(line_num),
                    description: format!(
                        "文件 '{}' 在 '{}' 中引用，但未找到",
                        import_path, source_file
                    ),
                    solution,
                    severity,
                    can_auto_fix: true,
                });
            }
        }
    }

    /// 解析 import 路径为实际文件路径
    fn resolve_import_path(&self, import_path: &str, importer: &Path, _src_root: &Path) -> Option<PathBuf> {
        if import_path.starts_with('/') {
            // 绝对路径（相对于项目根目录）
            Some(self.project_root.join(&import_path[1..]))
        } else if import_path.starts_with("./") || import_path.starts_with("../") {
            // 相对路径
            let importer_dir = importer.parent()?;
            let resolved = importer_dir.join(import_path);
            
            // 规范化路径
            let canonical = self.normalize_path(&resolved);
            
            // 检查文件是否存在（可能没有扩展名）
            if canonical.exists() {
                Some(canonical)
            } else {
                // 尝试添加 .vue, .js, .ts, .mjs 等扩展名
                let extensions = ["", ".vue", ".js", ".ts", ".jsx", ".tsx", ".mjs", ".css", ".scss"];
                for ext in &extensions {
                    let with_ext = if ext.is_empty() {
                        canonical.clone()
                    } else {
                        let mut p = canonical.to_string_lossy().to_string();
                        p.push_str(ext);
                        PathBuf::from(p)
                    };
                    if with_ext.exists() {
                        return Some(with_ext);
                    }
                }
                
                // 检查是否是目录（import from './components'）
                if canonical.is_dir() {
                    let index_files = ["index.vue", "index.js", "index.ts", "index.mjs"];
                    for index_file in &index_files {
                        let index_path = canonical.join(index_file);
                        if index_path.exists() {
                            return Some(index_path);
                        }
                    }
                }
                
                None
            }
        } else {
            None
        }
    }

    /// 规范化路径（去除 ../ 和 ./，保留根路径）
    fn normalize_path(&self, path: &Path) -> PathBuf {
        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::Prefix(p) => result.push(p.as_os_str()),
                std::path::Component::RootDir => result.push(component.as_os_str()),
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => { result.pop(); }
                std::path::Component::Normal(c) => result.push(c),
            }
        }
        result
    }

    /// 检查 npm 包是否在 node_modules 中安装
    fn is_npm_package_installed(&self, package_name: &str) -> bool {
        let node_modules = self.project_root.join("node_modules");
        if !node_modules.exists() {
            return false;
        }
        
        let package_path = if package_name.starts_with('@') {
            node_modules.join(package_name)
        } else {
            node_modules.join(package_name)
        };
        
        package_path.exists() && package_path.join("package.json").exists()
    }

    /// 扫描 node_modules 中已安装的包
    fn scan_installed_packages(project_root: &Path) -> HashSet<String> {
        let mut packages = HashSet::new();
        let node_modules = project_root.join("node_modules");
        
        if !node_modules.exists() {
            return packages;
        }

        // 扫描 scoped 包 (@vue/xxx)
        if let Ok(entries) = std::fs::read_dir(&node_modules) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy().to_string();
                    
                    if name_str.starts_with('@') && path.is_dir() {
                        // scoped package: @scope/name
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub in sub_entries.flatten() {
                                if sub.path().join("package.json").exists() {
                                    let sub_name = sub.path().file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    packages.insert(format!("{}/{}", name_str, sub_name));
                                }
                            }
                        }
                    } else if path.is_dir() && path.join("package.json").exists() {
                        packages.insert(name_str);
                    }
                }
            }
        }
        
        packages
    }

    /// 加载 package.json 中的依赖
    fn load_declared_deps(project_root: &Path) -> HashMap<String, String> {
        let mut deps = HashMap::new();
        let package_json_path = project_root.join("package.json");
        
        if !package_json_path.exists() {
            return deps;
        }
        
        let content = match std::fs::read_to_string(&package_json_path) {
            Ok(c) => c,
            Err(_) => return deps,
        };
        
        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(j) => j,
            Err(_) => return deps,
        };
        
        // 读取 dependencies
        if let Some(deps_obj) = json.get("dependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps_obj {
                deps.insert(name.clone(), version.as_str().unwrap_or("").to_string());
            }
        }
        
        // 读取 devDependencies
        if let Some(deps_obj) = json.get("devDependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps_obj {
                deps.insert(name.clone(), version.as_str().unwrap_or("").to_string());
            }
        }
        
        deps
    }

    /// 解析未被声明的 npm 包的版本
    /// 
    /// 版本决议顺序：
    /// 1. 从传递依赖中查找（其他已安装包的 dependencies/peerDependencies）
    /// 2. 从 npm registry 查询 latest
    pub fn resolve_package_version(&self, package_name: &str) -> Option<String> {
        // 先从传递依赖中查找
        if let Some(version) = self.resolve_from_transitive_deps(package_name) {
            debug!("Found transitive dependency version: {}@{}", package_name, version);
            return Some(version);
        }
        
        // 否则返回 None，由调用者决定使用 "latest"
        None
    }

    /// 从传递依赖中查找版本
    fn resolve_from_transitive_deps(&self, package_name: &str) -> Option<String> {
        let node_modules = self.project_root.join("node_modules");
        if !node_modules.exists() {
            return None;
        }
        
        if let Ok(entries) = std::fs::read_dir(&node_modules) {
            for entry in entries.flatten() {
                let pkg_json_path = entry.path().join("package.json");
                if !pkg_json_path.exists() {
                    continue;
                }
                
                let content = std::fs::read_to_string(&pkg_json_path).ok()?;
                let json: serde_json::Value = serde_json::from_str(&content).ok()?;
                
                // 检查 dependencies
                if let Some(deps) = json.get("dependencies").and_then(|v| v.as_object()) {
                    if let Some(version) = deps.get(package_name) {
                        return version.as_str().map(|s| s.to_string());
                    }
                }
                
                // 检查 peerDependencies
                if let Some(deps) = json.get("peerDependencies").and_then(|v| v.as_object()) {
                    if let Some(version) = deps.get(package_name) {
                        return version.as_str().map(|s| s.to_string());
                    }
                }
            }
        }
        
        None
    }

    /// 查找所有未安装的 npm 包（用于自动下载）
    pub fn find_uninstalled_npm_packages(&self) -> Vec<String> {
        let src_dir = self.project_root.join("src");
        let mut packages = HashSet::new();
        
        if !src_dir.exists() {
            return Vec::new();
        }
        
        self.collect_npm_imports(&src_dir, &mut packages);
        
        // 过滤掉已声明、已在 irisResolved 中记录、以及已安装的
        packages.into_iter()
            .filter(|p| {
                !self.declared_deps.contains_key(p)
                    && !self.iris_resolved.contains_key(p)
                    && !self.installed_packages.contains(p)
                    && !self.is_npm_package_installed(p)
            })
            .collect()
    }

    /// 递归收集所有 npm 导入
    fn collect_npm_imports(&self, dir: &Path, packages: &mut HashSet<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str != "node_modules" && !name_str.starts_with('.') {
                            self.collect_npm_imports(&path, packages);
                        }
                    }
                } else if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    match ext_str.as_ref() {
                        "vue" | "js" | "jsx" | "ts" | "tsx" => {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                for line in content.lines() {
                                    if let Some(import) = self.extract_import_path(line.trim()) {
                                        if !import.starts_with('.') && !import.starts_with('/') {
                                            packages.insert(import);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // ============================================================
    // irisResolved 字段管理
    // ============================================================

    /// 从 package.json 加载 irisResolved 字段
    fn load_iris_resolved(project_root: &Path) -> HashMap<String, String> {
        let mut resolved = HashMap::new();
        let package_json_path = project_root.join("package.json");
        
        if !package_json_path.exists() {
            return resolved;
        }
        
        let content = match std::fs::read_to_string(&package_json_path) {
            Ok(c) => c,
            Err(_) => return resolved,
        };
        
        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(j) => j,
            Err(_) => return resolved,
        };
        
        if let Some(iris) = json.get("irisResolved").and_then(|v| v.as_object()) {
            for (name, version) in iris {
                resolved.insert(name.clone(), version.as_str().unwrap_or("").to_string());
            }
        }
        
        resolved
    }

    /// 获取 irisResolved 字段的引用
    pub fn iris_resolved(&self) -> &HashMap<String, String> {
        &self.iris_resolved
    }

    /// 获取已安装包的实际版本号（从 node_modules 中的 package.json 读取）
    pub fn get_actual_installed_version(&self, package_name: &str) -> Option<String> {
        let package_path = self.project_root.join("node_modules").join(package_name).join("package.json");
        if !package_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&package_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        json.get("version").and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// 将自动下载的包及其版本写入 package.json 的 irisResolved 字段
    /// 
    /// 返回更新后的 irisResolved 内容（HashMap）
    pub fn write_iris_resolved(&self, resolved: HashMap<String, String>) -> Result<()> {
        let package_json_path = self.project_root.join("package.json");
        
        let content = std::fs::read_to_string(&package_json_path)
            .map_err(|e| anyhow::anyhow!("Failed to read package.json: {}", e))?;
        
        let mut json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse package.json: {}", e))?;
        
        // 获取或创建 irisResolved 对象
        let iris_obj = json.get_mut("irisResolved")
            .and_then(|v| v.as_object_mut())
            .map(|obj| {
                for (name, version) in &resolved {
                    obj.insert(name.clone(), serde_json::Value::String(version.clone()));
                }
                obj
            });
        
        if iris_obj.is_none() {
            // 创建 irisResolved 字段
            let mut new_obj = serde_json::Map::new();
            for (name, version) in &resolved {
                new_obj.insert(name.clone(), serde_json::Value::String(version.clone()));
            }
            json.as_object_mut()
                .map(|obj| obj.insert("irisResolved".to_string(), serde_json::Value::Object(new_obj)));
        }
        
        // 写回 package.json（保留原始格式）
        // 使用 json! 宏重新序列化为格式化的 JSON
        let pretty = serde_json::to_string_pretty(&json)
            .map_err(|e| anyhow::anyhow!("Failed to serialize package.json: {}", e))?;
        
        std::fs::write(&package_json_path, pretty.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to write package.json: {}", e))?;
        
        info!("Updated package.json irisResolved with {:?}", resolved);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_import_path() {
        let scanner = DependencyScanner::new(PathBuf::from("."));
        
        assert_eq!(
            scanner.extract_import_path("import { ref } from 'vue'"),
            Some("vue".to_string())
        );
        assert_eq!(
            scanner.extract_import_path("import App from './App.vue'"),
            Some("./App.vue".to_string())
        );
        assert_eq!(
            scanner.extract_import_path("import 'vue'"),
            Some("vue".to_string())
        );
        assert_eq!(
            scanner.extract_import_path("const mod = require('./common.js')"),
            Some("./common.js".to_string())
        );
    }

    #[test]
    fn test_scan_issues() {
        let temp_dir = std::env::temp_dir().join("iris_scanner_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir.join("src")).unwrap();
        
        // 创建 package.json
        fs::write(
            temp_dir.join("package.json"),
            r#"{"dependencies": {"vue": "^3.0.0"}}"#
        ).unwrap();
        
        // 创建源码文件
        fs::write(
            temp_dir.join("src/main.ts"),
            r#"import { createApp } from 'vue'
import App from './App.vue'
import { ElButton } from 'element-plus'
import './styles.css'
import logo from './logo.png'"#
        ).unwrap();
        
        // 创建 App.vue
        fs::write(
            temp_dir.join("src/App.vue"),
            "<template><div>Hello</div></template>\n<script>\nexport default {}\n</script>"
        ).unwrap();
        
        let scanner = DependencyScanner::new(temp_dir.clone());
        let result = scanner.scan();
        
        // 应该发现问题：element-plus 未声明，styles.css 不存在，logo.png 不存在
        let npm_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.issue_type == IssueType::MissingNpmPackage)
            .collect();
        assert!(!npm_issues.is_empty(), "Should find missing npm package");
        assert_eq!(npm_issues[0].import_path, "element-plus");
        
        let css_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.issue_type == IssueType::MissingCssFile)
            .collect();
        assert!(!css_issues.is_empty(), "Should find missing CSS file");
        
        let asset_issues: Vec<_> = result.issues.iter()
            .filter(|i| i.issue_type == IssueType::MissingAsset)
            .collect();
        assert!(!asset_issues.is_empty(), "Should find missing asset");
        
        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
