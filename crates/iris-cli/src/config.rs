//! Iris 项目配置

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};

/// Iris 项目配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrisConfig {
    /// 项目名称
    pub name: String,
    
    /// 项目版本
    pub version: String,
    
    /// 源代码目录
    #[serde(default = "default_src_dir")]
    pub src_dir: PathBuf,
    
    /// 输出目录
    #[serde(default = "default_out_dir")]
    pub out_dir: PathBuf,
    
    /// 入口文件
    #[serde(default = "default_entry")]
    pub entry: String,
    
    /// 开发服务器配置
    #[serde(default)]
    pub dev_server: DevServerConfig,
    
    /// 构建配置
    #[serde(default)]
    pub build: BuildConfig,
}

/// 开发服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevServerConfig {
    /// 服务器端口
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// 是否启用热重载
    #[serde(default = "default_true")]
    pub hot_reload: bool,
    
    /// 是否自动打开浏览器
    #[serde(default = "default_false")]
    pub open: bool,
}

/// 构建配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// 是否压缩输出
    #[serde(default = "default_true")]
    pub minify: bool,
    
    /// 是否生成 sourcemap
    #[serde(default = "default_false")]
    pub sourcemap: bool,
    
    /// 目标平台
    #[serde(default = "default_target")]
    pub target: String,
}

fn default_src_dir() -> PathBuf {
    PathBuf::from("src")
}

fn default_out_dir() -> PathBuf {
    PathBuf::from("dist")
}

fn default_entry() -> String {
    "main.vue".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_target() -> String {
    "web".to_string()
}

impl Default for IrisConfig {
    fn default() -> Self {
        Self {
            name: "iris-app".to_string(),
            version: "0.1.0".to_string(),
            src_dir: default_src_dir(),
            out_dir: default_out_dir(),
            entry: default_entry(),
            dev_server: DevServerConfig::default(),
            build: BuildConfig::default(),
        }
    }
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            hot_reload: default_true(),
            open: default_false(),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            minify: default_true(),
            sourcemap: default_false(),
            target: default_target(),
        }
    }
}

impl IrisConfig {
    /// 从项目根目录加载配置
    pub fn load(project_root: &Path) -> Result<Self> {
        let config_path = project_root.join("iris.config.json");
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
            
            let config: IrisConfig = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
            
            Ok(config)
        } else {
            // 返回默认配置
            Ok(Self::default())
        }
    }
    
    /// 保存配置到文件
    #[allow(dead_code)]
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let config_path = project_root.join("iris.config.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
        Ok(())
    }
    
    /// 检测项目类型
    pub fn detect_project_type(project_root: &Path) -> ProjectType {
        // 检查是否有 package.json
        if project_root.join("package.json").exists() {
            // 检查依赖
            if let Ok(content) = std::fs::read_to_string(project_root.join("package.json")) {
                if let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(deps) = package_json.get("dependencies").and_then(|d| d.as_object()) {
                        if deps.contains_key("vue") {
                            return ProjectType::Vue3;
                        }
                    }
                    if let Some(dev_deps) = package_json.get("devDependencies").and_then(|d| d.as_object()) {
                        if dev_deps.contains_key("vue") {
                            return ProjectType::Vue3;
                        }
                    }
                }
            }
        }
        
        // 检查是否有 Vue SFC 文件
        let src_dir = project_root.join("src");
        if src_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "vue" {
                            return ProjectType::Vue3;
                        }
                    }
                }
            }
        }
        
        ProjectType::Unknown
    }
}

/// 项目类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    /// Vue 3 项目
    Vue3,
    /// 未知类型
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_default_config() {
        let config = IrisConfig::default();
        assert_eq!(config.name, "iris-app");
        assert_eq!(config.src_dir, PathBuf::from("src"));
        assert_eq!(config.out_dir, PathBuf::from("dist"));
        assert_eq!(config.dev_server.port, 3000);
    }
    
    #[test]
    fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = IrisConfig::default();
        config.name = "test-app".to_string();
        
        config.save(temp_dir.path()).unwrap();
        let loaded = IrisConfig::load(temp_dir.path()).unwrap();
        
        assert_eq!(loaded.name, "test-app");
    }
    
    #[test]
    fn test_detect_vue3_project() {
        let temp_dir = TempDir::new().unwrap();
        
        // 创建 package.json
        let package_json = r#"{
            "name": "test-app",
            "dependencies": {
                "vue": "^3.3.0"
            }
        }"#;
        
        fs::write(
            temp_dir.path().join("package.json"),
            package_json
        ).unwrap();
        
        let project_type = IrisConfig::detect_project_type(temp_dir.path());
        assert_eq!(project_type, ProjectType::Vue3);
    }
}
