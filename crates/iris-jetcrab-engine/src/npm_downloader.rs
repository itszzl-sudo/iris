//! npm 包下载器
//!
//! 功能：
//! - 直接从 npm registry 下载包
//! - 不依赖外部工具（npm、yarn 等）
//! - 支持 scoped packages (@vue/runtime-core)
//! - 自动解压 tarball
//! - 缓存机制避免重复下载

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::env;

use flate2::read::GzDecoder;
use tar::Archive;
use tracing::{debug, info, warn};
use anyhow::{Result, Context};

/// npm registry 基础 URL
const NPM_REGISTRY: &str = "https://registry.npmjs.org";

/// 内部包列表（不需要从 npm registry 下载）
/// 这些包是 Iris 框架的核心组件，由项目自身提供
const INTERNAL_PACKAGES: &[&str] = &[
    "iris",
    "iris-runtime",
    "iris-core",
    "iris-gpu",
    "iris-layout",
    "iris-dom",
    "iris-sfc",
    "iris-cssom",
    "iris-jetcrab",
    "iris-jetcrab-engine",
    "iris-jetcrab-cli",
];

/// 包版本信息
#[derive(Debug, Clone)]
pub struct PackageVersion {
    pub version: String,
    pub tarball_url: String,
}

/// npm 包下载器
pub struct NpmDownloader {
    /// node_modules 目录路径
    node_modules_path: PathBuf,
    /// HTTP 客户端（使用 ureq）
    client: ureq::Agent,
    /// 进度回调函数
    progress_callback: Option<Box<dyn Fn(&str, &str, u8, &str) + Send + Sync>>,
}

impl NpmDownloader {
    /// 创建新的下载器
    pub fn new(node_modules_path: PathBuf) -> Self {
        Self {
            node_modules_path,
            client: ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(10))
                .timeout_read(std::time::Duration::from_secs(30))
                .build(),
            progress_callback: None,
        }
    }

    /// 设置进度回调函数
    /// 
    /// 回调函数参数：(package_name, version, progress_percent, status)
    pub fn with_progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, &str, u8, &str) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    /// 报告进度
    fn report_progress(&self, package: &str, version: &str, progress: u8, status: &str) {
        if let Some(callback) = &self.progress_callback {
            callback(package, version, progress, status);
        }
    }

    /// 检查是否为内部包（不需要从 npm 下载）
    pub fn is_internal_package(package_name: &str) -> bool {
        INTERNAL_PACKAGES.contains(&package_name)
    }

    /// 确保 node_modules 目录存在
    fn ensure_node_modules(&self) -> Result<()> {
        if !self.node_modules_path.exists() {
            fs::create_dir_all(&self.node_modules_path)
                .context("Failed to create node_modules directory")?;
            debug!("Created node_modules directory: {:?}", self.node_modules_path);
        }
        Ok(())
    }

    /// 检查包是否已安装
    pub fn is_package_installed(&self, package_name: &str) -> bool {
        let package_path = self.get_package_path(package_name);
        package_path.exists() && package_path.join("package.json").exists()
    }

    /// 下载并安装 npm 包
    ///
    /// # 参数
    ///
    /// * `package_name` - 包名（如 "vue", "@vue/runtime-core"）
    /// * `version` - 版本号（如 "3.5.33"，可选，默认 latest）
    ///
    /// # 返回
    ///
    /// 返回包的安装路径
    pub fn download_and_install(&self, package_name: &str, version: Option<&str>) -> Result<PathBuf> {
        // 检查是否为内部包
        if Self::is_internal_package(package_name) {
            debug!("Skipping internal package: {}", package_name);
            // 返回一个不存在的路径，让调用者知道不需要下载
            return Err(anyhow::anyhow!(
                "Package '{}' is an internal Iris package, skipping download",
                package_name
            ));
        }

        // 检查是否已安装
        if self.is_package_installed(package_name) {
            debug!("Package already installed: {}", package_name);
            return Ok(self.get_package_path(package_name));
        }

        info!("Downloading npm package: {}@{}", 
              package_name, version.unwrap_or("latest"));

        // 1. 获取包信息
        self.report_progress(package_name, version.unwrap_or("latest"), 5, "resolving");
        let version = self.resolve_version(package_name, version)?;
        
        // 2. 下载 tarball
        self.report_progress(package_name, &version.version, 20, "downloading");
        let tarball_data = self.download_tarball(&version.tarball_url)?;
        self.report_progress(package_name, &version.version, 70, "downloading");
        
        // 3. 解压并安装
        self.report_progress(package_name, &version.version, 80, "extracting");
        let package_path = self.extract_and_install(package_name, &tarball_data)?;
        self.report_progress(package_name, &version.version, 100, "installed");

        info!("Installed npm package: {}@{} -> {:?}", 
              package_name, version.version, package_path);

        Ok(package_path)
    }

    /// 解析版本号，获取 tarball URL
    fn resolve_version(&self, package_name: &str, version: Option<&str>) -> Result<PackageVersion> {
        let version = version.unwrap_or("latest");
        
        // 构建 API URL
        let url = if version == "latest" {
            format!("{}/{}", NPM_REGISTRY, package_name)
        } else {
            format!("{}/{}/{}", NPM_REGISTRY, package_name, version)
        };

        debug!("Fetching package info from: {}", url);

        // 发送 HTTP 请求
        let response = self.client.get(&url)
            .call()
            .context(format!("Failed to fetch package info: {}", package_name))?;

        let json: serde_json::Value = response.into_json()
            .context("Failed to parse package info JSON")?;

        // 解析 tarball URL
        if version == "latest" {
            // 获取 latest 标签指向的版本
            let latest_version = json.get("dist-tags")
                .and_then(|v| v.get("latest"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("No 'latest' version found for {}", package_name))?;

            let tarball_url = json.get("versions")
                .and_then(|v| v.get(latest_version))
                .and_then(|v| v.get("dist"))
                .and_then(|v| v.get("tarball"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("No tarball URL found for {}@{}", package_name, latest_version))?;

            Ok(PackageVersion {
                version: latest_version.to_string(),
                tarball_url: tarball_url.to_string(),
            })
        } else {
            // 直接获取指定版本
            let tarball_url = json.get("dist")
                .and_then(|v| v.get("tarball"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("No tarball URL found for {}@{}", package_name, version))?;

            Ok(PackageVersion {
                version: version.to_string(),
                tarball_url: tarball_url.to_string(),
            })
        }
    }

    /// 下载 tarball
    fn download_tarball(&self, url: &str) -> Result<Vec<u8>> {
        debug!("Downloading tarball from: {}", url);

        let response = self.client.get(url)
            .call()
            .context(format!("Failed to download tarball: {}", url))?;

        let mut data = Vec::new();
        response.into_reader()
            .read_to_end(&mut data)
            .context("Failed to read tarball data")?;

        debug!("Downloaded tarball: {} bytes", data.len());

        Ok(data)
    }

    /// 解压 tarball 并安装到 node_modules
    fn extract_and_install(&self, package_name: &str, tarball_data: &[u8]) -> Result<PathBuf> {
        let package_path = self.get_package_path(package_name);
        
        // 确保父目录存在
        if let Some(parent) = package_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create package parent directory")?;
        }

        // 创建包目录
        fs::create_dir_all(&package_path)
            .context("Failed to create package directory")?;

        // 解压 tarball
        debug!("Extracting tarball to: {:?}", package_path);

        let decoder = GzDecoder::new(tarball_data);
        let mut archive = Archive::new(decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let entry_path = entry.path()?;
            
            // tarball 中的文件通常在 "package/" 目录下
            let relative_path = entry_path
                .strip_prefix("package")
                .unwrap_or(&entry_path);
            
            let target_path = package_path.join(relative_path);

            // 检查是否为目录
            let entry_type = entry.header().entry_type();
            if entry_type.is_dir() {
                fs::create_dir_all(&target_path)?;
            } else if entry_type.is_file() {
                // 确保父目录存在
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                // 写入文件
                let mut file = fs::File::create(&target_path)?;
                io::copy(&mut entry, &mut file)?;
            }
        }

        debug!("Extracted {} files", package_path.read_dir()?.count());

        Ok(package_path)
    }

    /// 获取包的本地路径
    fn get_package_path(&self, package_name: &str) -> PathBuf {
        self.node_modules_path.join(package_name)
    }

    /// 批量下载多个包
    pub fn download_multiple(&self, packages: &[(String, Option<String>)]) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        
        for (name, version) in packages {
            match self.download_and_install(name, version.as_deref()) {
                Ok(path) => paths.push(path),
                Err(e) => {
                    warn!("Failed to download package {}: {}", name, e);
                }
            }
        }
        
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    #[ignore] // 需要网络连接
    fn test_download_vue() {
        let temp_dir = std::env::temp_dir().join("test_npm_download");
        let _ = fs::remove_dir_all(&temp_dir);
        
        let downloader = NpmDownloader::new(temp_dir.clone());
        let result = downloader.download_and_install("vue", Some("3.5.33"));
        
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.join("package.json").exists());
        
        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[ignore] // 需要网络连接
    fn test_download_scoped_package() {
        let temp_dir = std::env::temp_dir().join("test_npm_scoped");
        let _ = fs::remove_dir_all(&temp_dir);
        
        let downloader = NpmDownloader::new(temp_dir.clone());
        let result = downloader.download_and_install("@vue/runtime-core", Some("3.5.33"));
        
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.join("package.json").exists());
        
        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_package_installed() {
        let temp_dir = std::env::temp_dir().join("test_npm_check");
        let _ = fs::remove_dir_all(&temp_dir);
        
        let downloader = NpmDownloader::new(temp_dir.clone());
        assert!(!downloader.is_package_installed("vue"));
        
        // 创建模拟的包
        let package_path = temp_dir.join("vue");
        fs::create_dir_all(&package_path).unwrap();
        fs::write(package_path.join("package.json"), "{}").unwrap();
        
        assert!(downloader.is_package_installed("vue"));
        
        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
