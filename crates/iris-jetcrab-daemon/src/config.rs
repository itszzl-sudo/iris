//! 守护进程配置管理
//!
//! 配置文件路径: %APPDATA%/iris-jetcrab/config.toml

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 守护进程全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    /// HTTP 开发服务器端口
    pub http_port: u16,
    /// Mock API 服务器端口
    pub mock_port: u16,
    /// 已知的 Vue 工程目录列表
    pub projects: Vec<String>,
    /// 默认项目路径（快捷启动）
    pub default_project: Option<String>,
    /// 自动启动的项目路径（程序启动时自动运行）
    pub auto_start: Option<String>,
    /// 是否显示桌面悬浮图标
    pub show_icon: bool,
    /// HTTP 开发服务器自动启动
    pub auto_start_server: bool,
    /// 管理 API 端口（内部）
    pub daemon_port: u16,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            http_port: 3000,
            mock_port: 3100,
            projects: Vec::new(),
            default_project: None,
            auto_start: None,
            show_icon: true,
            auto_start_server: false,
            daemon_port: 19999,
        }
    }
}

impl DaemonConfig {
    /// 获取配置文件路径
    pub fn config_path() -> PathBuf {
        let base = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("iris-jetcrab");
        std::fs::create_dir_all(&base).ok();
        base.join("config.toml")
    }

    /// 加载配置，如果不存在则使用默认值
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            let config = DaemonConfig::default();
            config.save();
            return config;
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!("Failed to parse config, using defaults: {}", e);
                    DaemonConfig::default()
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read config, using defaults: {}", e);
                DaemonConfig::default()
            }
        }
    }

    /// 保存配置到文件
    pub fn save(&self) {
        let path = Self::config_path();
        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    tracing::error!("Failed to save config: {}", e);
                }
            }
            Err(e) => tracing::error!("Failed to serialize config: {}", e),
        }
    }

    /// 添加 Vue 工程目录
    pub fn add_project(&mut self, path: String) {
        let normalized = path.replace('\\', "/");
        if !self.projects.contains(&normalized) {
            self.projects.push(normalized);
            self.save();
        }
    }

    /// 移除 Vue 工程目录
    pub fn remove_project(&mut self, path: &str) {
        let normalized = path.replace('\\', "/");
        self.projects.retain(|p| p != &normalized);
        if self.default_project.as_deref() == Some(&normalized) {
            self.default_project = None;
        }
        if self.auto_start.as_deref() == Some(&normalized) {
            self.auto_start = None;
        }
        self.save();
    }
}
