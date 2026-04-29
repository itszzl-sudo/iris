//! 依赖树管理模块
//!
//! 负责：
//! 1. 解析 package.json 构建依赖树
//! 2. 排除编译工具类依赖（vite、webpack、babel 等）
//! 3. 检测依赖版本变化
//! 4. 按需重新编译受影响的模块

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, debug};

/// 编译工具类依赖（需要排除）
const BUILD_TOOLS: &[&str] = &[
    // 构建工具
    "vite", "webpack", "webpack-cli", "webpack-dev-server",
    "rollup", "parcel", "esbuild", "swc",
    // Babel 相关
    "babel-loader", "@babel/core", "@babel/preset-env", "@babel/preset-typescript",
    // TypeScript 编译
    "typescript", "ts-loader", "ts-node",
    // 开发工具
    "eslint", "prettier", "stylelint",
    "eslint-loader", "css-loader", "sass-loader", "less-loader",
    // 测试工具
    "jest", "vitest", "mocha", "chai",
    // 其他开发依赖
    "nodemon", "concurrently", "cross-env",
];

/// 依赖信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// 包名
    pub name: String,
    /// 版本要求（如 "^3.0.0"）
    pub version_req: String,
    /// 实际安装的版本
    pub installed_version: Option<String>,
    /// 是否为开发依赖
    pub is_dev_dependency: bool,
    /// 是否为编译工具（需要排除）
    pub is_build_tool: bool,
    /// 包路径
    pub package_path: Option<PathBuf>,
    /// 依赖的其他包
    pub dependencies: Vec<String>,
}

/// 依赖树
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTree {
    /// 项目根目录
    pub project_root: PathBuf,
    /// 所有依赖
    pub dependencies: HashMap<String, DependencyInfo>,
    /// 运行时依赖（非编译工具）
    pub runtime_dependencies: HashMap<String, DependencyInfo>,
    /// 依赖哈希（用于检测变化）
    pub dependency_hash: String,
}

impl DependencyTree {
    /// 从 package.json 构建依赖树
    pub fn from_package_json(project_root: &Path) -> Result<Self> {
        let package_json_path = project_root.join("package.json");
        
        if !package_json_path.exists() {
            return Err(anyhow::anyhow!("package.json not found"));
        }

        let content = std::fs::read_to_string(&package_json_path)
            .context("Failed to read package.json")?;
        
        let package_json: serde_json::Value = serde_json::from_str(&content)
            .context("Failed to parse package.json")?;

        let mut dependencies = HashMap::new();
        let mut runtime_dependencies = HashMap::new();

        // 解析 dependencies
        if let Some(deps) = package_json.get("dependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps {
                let dep_info = Self::parse_dependency(
                    project_root,
                    name,
                    version.as_str().unwrap_or(""),
                    false,
                )?;
                
                if !dep_info.is_build_tool {
                    runtime_dependencies.insert(name.clone(), dep_info.clone());
                }
                dependencies.insert(name.clone(), dep_info);
            }
        }

        // 解析 devDependencies
        if let Some(deps) = package_json.get("devDependencies").and_then(|v| v.as_object()) {
            for (name, version) in deps {
                let dep_info = Self::parse_dependency(
                    project_root,
                    name,
                    version.as_str().unwrap_or(""),
                    true,
                )?;
                
                if !dep_info.is_build_tool {
                    runtime_dependencies.insert(name.clone(), dep_info.clone());
                }
                dependencies.insert(name.clone(), dep_info);
            }
        }

        // 计算依赖哈希
        let dependency_hash = Self::calculate_hash(&dependencies);

        info!(
            "Loaded {} dependencies ({} runtime)",
            dependencies.len(),
            runtime_dependencies.len()
        );

        Ok(Self {
            project_root: project_root.to_path_buf(),
            dependencies,
            runtime_dependencies,
            dependency_hash,
        })
    }

    /// 解析单个依赖
    fn parse_dependency(
        project_root: &Path,
        name: &str,
        version_req: &str,
        is_dev: bool,
    ) -> Result<DependencyInfo> {
        let is_build_tool = Self::is_build_tool(name);
        
        // 查找包的实际路径和版本
        let package_path = Self::find_package_path(project_root, name);
        let installed_version = package_path
            .as_ref()
            .and_then(|p| Self::read_package_version(p));

        // 解析包的依赖
        let dependencies = package_path
            .as_ref()
            .and_then(|p| Self::read_package_dependencies(p).ok())
            .unwrap_or_default();

        Ok(DependencyInfo {
            name: name.to_string(),
            version_req: version_req.to_string(),
            installed_version,
            is_dev_dependency: is_dev,
            is_build_tool,
            package_path,
            dependencies,
        })
    }

    /// 检查是否为编译工具
    pub fn is_build_tool(name: &str) -> bool {
        BUILD_TOOLS.iter().any(|&tool| name == tool || name.starts_with(&format!("{}@", tool)))
    }

    /// 查找包路径
    fn find_package_path(project_root: &Path, name: &str) -> Option<PathBuf> {
        let node_modules = project_root.join("node_modules");
        
        // 处理 scoped packages (@vue/cli)
        if name.starts_with('@') {
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() == 2 {
                let path = node_modules.join(parts[0]).join(parts[1]);
                if path.exists() {
                    return Some(path);
                }
            }
        } else {
            let path = node_modules.join(name);
            if path.exists() {
                return Some(path);
            }
        }
        
        None
    }

    /// 读取包的版本
    fn read_package_version(package_path: &Path) -> Option<String> {
        let package_json = package_path.join("package.json");
        if !package_json.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&package_json).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        
        json.get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// 读取包的依赖
    fn read_package_dependencies(package_path: &Path) -> Result<Vec<String>> {
        let package_json = package_path.join("package.json");
        if !package_json.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&package_json)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let mut deps = Vec::new();

        if let Some(dependencies) = json.get("dependencies").and_then(|v| v.as_object()) {
            for name in dependencies.keys() {
                deps.push(name.clone());
            }
        }

        Ok(deps)
    }

    /// 计算依赖哈希
    pub fn calculate_hash(dependencies: &HashMap<String, DependencyInfo>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        
        // 按名称排序确保一致性
        let mut names: Vec<&String> = dependencies.keys().collect();
        names.sort();
        
        for name in names {
            if let Some(dep) = dependencies.get(name) {
                name.hash(&mut hasher);
                dep.version_req.hash(&mut hasher);
                if let Some(version) = &dep.installed_version {
                    version.hash(&mut hasher);
                }
            }
        }
        
        format!("{:x}", hasher.finish())
    }

    /// 检查依赖是否发生变化
    pub fn has_changed(&self, other: &DependencyTree) -> bool {
        self.dependency_hash != other.dependency_hash
    }

    /// 获取变化的依赖列表
    pub fn get_changed_dependencies(&self, other: &DependencyTree) -> Vec<ChangedDependency> {
        let mut changes = Vec::new();

        // 检查新增和更新的依赖
        for (name, new_dep) in &other.dependencies {
            if let Some(old_dep) = self.dependencies.get(name) {
                if old_dep.installed_version != new_dep.installed_version {
                    changes.push(ChangedDependency {
                        name: name.clone(),
                        old_version: old_dep.installed_version.clone(),
                        new_version: new_dep.installed_version.clone(),
                        change_type: if old_dep.installed_version.is_none() {
                            ChangeType::Added
                        } else {
                            ChangeType::Updated
                        },
                    });
                }
            } else {
                changes.push(ChangedDependency {
                    name: name.clone(),
                    old_version: None,
                    new_version: new_dep.installed_version.clone(),
                    change_type: ChangeType::Added,
                });
            }
        }

        // 检查删除的依赖
        for (name, old_dep) in &self.dependencies {
            if !other.dependencies.contains_key(name) {
                changes.push(ChangedDependency {
                    name: name.clone(),
                    old_version: old_dep.installed_version.clone(),
                    new_version: None,
                    change_type: ChangeType::Removed,
                });
            }
        }

        changes
    }

    /// 获取需要重新编译的模块列表
    pub fn get_modules_to_rebuild(
        &self,
        changes: &[ChangedDependency],
        module_dependencies: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let mut modules_to_rebuild = Vec::new();

        for change in changes {
            // 查找依赖此包的所有模块
            for (module, deps) in module_dependencies {
                if deps.contains(&change.name) {
                    if !modules_to_rebuild.contains(module) {
                        modules_to_rebuild.push(module.clone());
                    }
                }
            }
        }

        info!(
            "Found {} modules to rebuild due to dependency changes",
            modules_to_rebuild.len()
        );

        modules_to_rebuild
    }

    /// 保存依赖树到缓存文件
    pub fn save_to_cache(&self) -> Result<()> {
        let cache_dir = self.project_root.join(".iris-cache");
        std::fs::create_dir_all(&cache_dir)?;

        let cache_file = cache_dir.join("dependency-tree.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&cache_file, content)?;

        debug!("Saved dependency tree to cache");
        Ok(())
    }

    /// 从缓存加载依赖树
    pub fn load_from_cache(project_root: &Path) -> Result<Self> {
        let cache_file = project_root.join(".iris-cache").join("dependency-tree.json");
        
        if !cache_file.exists() {
            return Err(anyhow::anyhow!("Cache file not found"));
        }

        let content = std::fs::read_to_string(&cache_file)?;
        let tree: DependencyTree = serde_json::from_str(&content)?;

        debug!("Loaded dependency tree from cache");
        Ok(tree)
    }
}

/// 依赖变化类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Updated,
    Removed,
}

/// 变化的依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedDependency {
    pub name: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub change_type: ChangeType,
}
