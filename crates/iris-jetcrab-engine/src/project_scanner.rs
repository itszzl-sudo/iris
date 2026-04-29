//! Vue 项目目录扫描器
//!
//! 负责扫描和解析 Vue 项目结构

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{debug, warn};
use serde::{Serialize, Deserialize};

/// Vue 项目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// 项目根目录
    pub root_dir: PathBuf,
    /// index.html 路径
    pub index_html_path: PathBuf,
    /// src 目录路径
    pub src_dir: PathBuf,
    /// 入口文件路径
    pub entry_file: PathBuf,
    /// package.json 路径
    pub package_json_path: Option<PathBuf>,
    /// 构建工具类型
    pub build_tool: Option<BuildTool>,
    /// Vue 版本
    pub vue_version: Option<String>,
}

/// 构建工具类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildTool {
    /// Vite
    Vite,
    /// Vue CLI (Webpack)
    VueCli,
    /// 未知
    Unknown,
}

/// 项目扫描器
pub struct ProjectScanner {
    /// 项目根目录
    root_dir: PathBuf,
}

impl ProjectScanner {
    /// 创建新的扫描器
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    /// 扫描项目目录
    pub fn scan(&self) -> Result<ProjectInfo> {
        debug!("Scanning project directory: {:?}", self.root_dir);

        // 1. 验证目录存在
        if !self.root_dir.exists() {
            anyhow::bail!("Project directory does not exist: {:?}", self.root_dir);
        }

        // 2. 查找 index.html
        let index_html_path = self.find_index_html()?;

        // 3. 查找 src 目录
        let src_dir = self.find_src_dir()?;

        // 4. 查找入口文件
        let entry_file = self.find_entry_file(&src_dir)?;

        // 5. 查找 package.json
        let package_json_path = self.find_package_json();

        // 6. 检测构建工具
        let build_tool = self.detect_build_tool();

        // 7. 检测 Vue 版本
        let vue_version = self.detect_vue_version();

        let project_info = ProjectInfo {
            root_dir: self.root_dir.clone(),
            index_html_path,
            src_dir,
            entry_file,
            package_json_path,
            build_tool,
            vue_version,
        };

        debug!("Project scan complete: {:?}", project_info);

        Ok(project_info)
    }

    /// 查找 index.html
    fn find_index_html(&self) -> Result<PathBuf> {
        // 优先查找根目录的 index.html
        let root_index = self.root_dir.join("index.html");
        if root_index.exists() {
            debug!("Found index.html in root: {:?}", root_index);
            return Ok(root_index);
        }

        // 查找 public/index.html（Vue CLI 项目）
        let public_index = self.root_dir.join("public").join("index.html");
        if public_index.exists() {
            debug!("Found index.html in public: {:?}", public_index);
            return Ok(public_index);
        }

        anyhow::bail!("index.html not found in project directory");
    }

    /// 查找 src 目录
    fn find_src_dir(&self) -> Result<PathBuf> {
        let src_dir = self.root_dir.join("src");
        
        if src_dir.exists() && src_dir.is_dir() {
            debug!("Found src directory: {:?}", src_dir);
            Ok(src_dir)
        } else {
            anyhow::bail!("src directory not found in project");
        }
    }

    /// 查找入口文件
    fn find_entry_file(&self, src_dir: &Path) -> Result<PathBuf> {
        // 常见的入口文件名
        let entry_names = [
            "main.js",
            "main.ts",
            "main.jsx",
            "main.tsx",
            "index.js",
            "index.ts",
        ];

        for name in &entry_names {
            let entry_path = src_dir.join(name);
            if entry_path.exists() {
                debug!("Found entry file: {:?}", entry_path);
                return Ok(entry_path);
            }
        }

        // 如果没找到，返回第一个 .js/.ts 文件
        if let Some(first_file) = self.find_first_js_file(src_dir)? {
            debug!("Using first JS file as entry: {:?}", first_file);
            return Ok(first_file);
        }

        anyhow::bail!("Entry file not found in src directory");
    }

    /// 查找第一个 JS/TS 文件
    fn find_first_js_file(&self, dir: &Path) -> Result<Option<PathBuf>> {
        if !dir.exists() {
            return Ok(None);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str());
                if matches!(ext, Some("js") | Some("ts") | Some("jsx") | Some("tsx")) {
                    return Ok(Some(path));
                }
            }
        }

        Ok(None)
    }

    /// 查找 package.json
    fn find_package_json(&self) -> Option<PathBuf> {
        let package_json = self.root_dir.join("package.json");
        
        if package_json.exists() {
            debug!("Found package.json: {:?}", package_json);
            Some(package_json)
        } else {
            warn!("package.json not found");
            None
        }
    }

    /// 检测构建工具
    fn detect_build_tool(&self) -> Option<BuildTool> {
        let package_json_path = self.root_dir.join("package.json");
        
        if !package_json_path.exists() {
            return None;
        }

        let content = match fs::read_to_string(&package_json_path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read package.json: {}", e);
                return None;
            }
        };

        let package_json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to parse package.json: {}", e);
                return None;
            }
        };

        // 检查依赖项
        let deps = package_json.get("dependencies").and_then(|d| d.as_object());
        let dev_deps = package_json.get("devDependencies").and_then(|d| d.as_object());

        let all_deps = deps.into_iter().chain(dev_deps.into_iter()).flatten();

        for (dep_name, dep_value) in all_deps {
            let dep_str = dep_value.as_str().unwrap_or("");
            
            if dep_str.contains("vite") {
                return Some(BuildTool::Vite);
            }
            
            if dep_name.contains("vue-loader") || dep_name.contains("@vue/cli") {
                return Some(BuildTool::VueCli);
            }
        }

        Some(BuildTool::Unknown)
    }

    /// 检测 Vue 版本
    fn detect_vue_version(&self) -> Option<String> {
        let package_json_path = self.root_dir.join("package.json");
        
        if !package_json_path.exists() {
            return None;
        }

        let content = match fs::read_to_string(&package_json_path) {
            Ok(c) => c,
            Err(_) => return None,
        };

        let package_json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return None,
        };

        // 检查 vue 依赖
        if let Some(deps) = package_json.get("dependencies").and_then(|d| d.as_object()) {
            if let Some(vue_version) = deps.get("vue").and_then(|v| v.as_str()) {
                return Some(vue_version.to_string());
            }
        }

        if let Some(dev_deps) = package_json.get("devDependencies").and_then(|d| d.as_object()) {
            if let Some(vue_version) = dev_deps.get("vue").and_then(|v| v.as_str()) {
                return Some(vue_version.to_string());
            }
        }

        None
    }
}
