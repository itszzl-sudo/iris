//! CPM (Crab Package Manager) 包管理集成
//!
//! 提供 npm 包解析、下载和管理功能。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// 包信息
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// 包名
    pub name: String,
    /// 版本号
    pub version: String,
    /// 包路径
    pub path: PathBuf,
    /// 依赖列表
    pub dependencies: HashMap<String, String>,
    /// 是否已安装
    pub installed: bool,
}

/// package.json 结构
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,
}

/// CPM 包管理器
pub struct CPMManager {
    /// 项目根目录
    project_root: PathBuf,
    /// 包缓存目录
    cache_dir: PathBuf,
    /// 已安装的包
    installed_packages: HashMap<String, PackageInfo>,
    /// 包注册表 URL
    registry_url: String,
}

impl CPMManager {
    /// 创建新的包管理器
    pub fn new(project_root: &Path) -> Self {
        let cache_dir = project_root.join(".jetcrab-cache");
        
        Self {
            project_root: project_root.to_path_buf(),
            cache_dir,
            installed_packages: HashMap::new(),
            registry_url: "https://registry.npmjs.org".to_string(),
        }
    }

    /// 设置自定义注册表 URL
    pub fn set_registry(&mut self, url: &str) {
        self.registry_url = url.to_string();
        info!("Registry URL set to: {}", url);
    }

    /// 解析 package.json
    pub fn parse_package_json(&self) -> Result<PackageJson, String> {
        let package_json_path = self.project_root.join("package.json");
        
        if !package_json_path.exists() {
            return Err("package.json not found".to_string());
        }

        let content = std::fs::read_to_string(&package_json_path)
            .map_err(|e| format!("Failed to read package.json: {}", e))?;

        let package_json: PackageJson = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse package.json: {}", e))?;

        info!("Parsed package.json: {}@{}", package_json.name, package_json.version);

        Ok(package_json)
    }

    /// 安装包
    pub fn install_package(&mut self, package_name: &str, version: &str) -> Result<PackageInfo, String> {
        info!("Installing package: {}@{}", package_name, version);

        // 检查是否已安装
        let cache_key = format!("{}@{}", package_name, version);
        if let Some(pkg) = self.installed_packages.get(&cache_key) {
            debug!("Package already installed: {}", cache_key);
            return Ok(pkg.clone());
        }

        // 创建缓存目录
        if !self.cache_dir.exists() {
            std::fs::create_dir_all(&self.cache_dir)
                .map_err(|e| format!("Failed to create cache dir: {}", e))?;
        }

        // 下载包（实际应该从注册表下载）
        let package_path = self.download_package(package_name, version)?;

        // 解析包的 package.json
        let pkg_json_path = package_path.join("package.json");
        let pkg_info = if pkg_json_path.exists() {
            let content = std::fs::read_to_string(&pkg_json_path)
                .map_err(|e| format!("Failed to read package.json: {}", e))?;
            let json: PackageJson = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse package.json: {}", e))?;
            
            PackageInfo {
                name: json.name,
                version: json.version,
                path: package_path.clone(),
                dependencies: json.dependencies,
                installed: true,
            }
        } else {
            PackageInfo {
                name: package_name.to_string(),
                version: version.to_string(),
                path: package_path.clone(),
                dependencies: HashMap::new(),
                installed: true,
            }
        };

        // 缓存包信息
        self.installed_packages
            .insert(cache_key.clone(), pkg_info.clone());

        info!("Package installed: {} -> {:?}", cache_key, package_path);

        Ok(pkg_info)
    }

    /// 下载包（模拟）
    fn download_package(&self, package_name: &str, version: &str) -> Result<PathBuf, String> {
        let package_dir = self.cache_dir.join(format!("{}-{}", package_name, version));

        if !package_dir.exists() {
            // 实际应该从 npm 注册表下载
            // 这里仅创建目录模拟
            std::fs::create_dir_all(&package_dir)
                .map_err(|e| format!("Failed to create package dir: {}", e))?;
            
            // 创建一个假的 package.json
            let package_json = PackageJson {
                name: package_name.to_string(),
                version: version.to_string(),
                dependencies: HashMap::new(),
                dev_dependencies: HashMap::new(),
            };

            let json_content = serde_json::to_string_pretty(&package_json)
                .map_err(|e| format!("Failed to serialize package.json: {}", e))?;
            std::fs::write(package_dir.join("package.json"), json_content)
                .map_err(|e| format!("Failed to write package.json: {}", e))?;
        }

        Ok(package_dir)
    }

    /// 安装所有依赖
    pub fn install_all(&mut self) -> Result<Vec<PackageInfo>, String> {
        info!("Installing all dependencies...");

        let package_json = self.parse_package_json()?;
        let mut installed = Vec::new();

        // 安装生产依赖
        for (name, version) in &package_json.dependencies {
            match self.install_package(name, version) {
                Ok(pkg) => installed.push(pkg),
                Err(e) => warn!("Failed to install {}: {}", name, e),
            }
        }

        // 安装开发依赖
        for (name, version) in &package_json.dev_dependencies {
            match self.install_package(name, version) {
                Ok(pkg) => installed.push(pkg),
                Err(e) => warn!("Failed to install dev dep {}: {}", name, e),
            }
        }

        info!("Installed {} packages", installed.len());

        Ok(installed)
    }

    /// 获取已安装的包
    pub fn get_installed_package(&self, package_name: &str, version: &str) -> Option<PackageInfo> {
        let cache_key = format!("{}@{}", package_name, version);
        self.installed_packages.get(&cache_key).cloned()
    }

    /// 列出所有已安装的包
    pub fn list_installed(&self) -> Vec<PackageInfo> {
        self.installed_packages.values().cloned().collect()
    }

    /// 卸载包
    pub fn uninstall_package(&mut self, package_name: &str, version: &str) -> Result<(), String> {
        let cache_key = format!("{}@{}", package_name, version);
        
        if self.installed_packages.remove(&cache_key).is_some() {
            info!("Uninstalled package: {}", cache_key);
            Ok(())
        } else {
            Err(format!("Package not found: {}", cache_key))
        }
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) -> Result<(), String> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| format!("Failed to clear cache: {}", e))?;
        }
        
        self.installed_packages.clear();
        info!("Cache cleared");
        
        Ok(())
    }

    /// 获取缓存大小
    pub fn cache_size(&self) -> usize {
        self.installed_packages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_manager() {
        let temp_dir = std::env::temp_dir().join("test-cpm");
        let manager = CPMManager::new(&temp_dir);
        assert_eq!(manager.cache_size(), 0);
    }

    #[test]
    fn test_set_registry() {
        let temp_dir = std::env::temp_dir().join("test-cpm-registry");
        let mut manager = CPMManager::new(&temp_dir);
        manager.set_registry("https://registry.npmmirror.com");
        assert_eq!(manager.registry_url, "https://registry.npmmirror.com");
    }

    #[test]
    fn test_parse_package_json() {
        let temp_dir = std::env::temp_dir().join("test-cpm-parse");
        std::fs::create_dir_all(&temp_dir).ok();
        
        // 创建测试 package.json
        let package_json = r#"{
            "name": "test-project",
            "version": "1.0.0",
            "dependencies": {
                "vue": "^3.0.0"
            }
        }"#;
        
        std::fs::write(temp_dir.join("package.json"), package_json).ok();
        
        let manager = CPMManager::new(&temp_dir);
        let result = manager.parse_package_json();
        
        assert!(result.is_ok());
        let pkg = result.unwrap();
        assert_eq!(pkg.name, "test-project");
        assert_eq!(pkg.version, "1.0.0");
    }

    #[test]
    fn test_install_package() {
        let temp_dir = std::env::temp_dir().join("test-cpm-install");
        std::fs::create_dir_all(&temp_dir).ok();
        
        let mut manager = CPMManager::new(&temp_dir);
        let result = manager.install_package("test-pkg", "1.0.0");
        
        assert!(result.is_ok());
        let pkg = result.unwrap();
        assert_eq!(pkg.name, "test-pkg");
        assert_eq!(pkg.version, "1.0.0");
        assert!(pkg.installed);
        assert_eq!(manager.cache_size(), 1);
    }

    #[test]
    fn test_uninstall_package() {
        let temp_dir = std::env::temp_dir().join("test-cpm-uninstall");
        std::fs::create_dir_all(&temp_dir).ok();
        
        let mut manager = CPMManager::new(&temp_dir);
        manager.install_package("test-pkg", "1.0.0").ok();
        
        let result = manager.uninstall_package("test-pkg", "1.0.0");
        assert!(result.is_ok());
        assert_eq!(manager.cache_size(), 0);
    }
}
