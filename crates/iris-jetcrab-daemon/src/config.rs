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

    // ── AI 云厂商模型服务 ───────────────────────────────
    /// AI 服务商 (openai / anthropic / custom)
    pub ai_provider: String,
    /// API Key
    pub ai_api_key: String,
    /// 模型名称 (如 gpt-4o, claude-3-sonnet)
    pub ai_model: String,
    /// 自定义 API 端点
    pub ai_endpoint: String,

    // ── AI 本地模型 ────────────────────────────────────
    /// HuggingFace 模型仓库（不可编辑，仅展示）
    pub ai_model_repo: String,
    /// GGUF 文件名（不可编辑，仅展示）
    pub ai_model_file: String,
    /// 推理设备 (cpu / cuda / vulkan / metal)
    pub ai_device: String,
    /// 温度参数 (0.0~1.0)
    pub ai_temperature: f32,
    /// 最大生成 token 数
    pub ai_max_tokens: usize,
    /// 是否已下载完成
    pub ai_model_downloaded: bool,

    // ── Iris 内置包管理器 ────────────────────────────────
    /// NPM registry 镜像
    pub npm_registry: String,
    /// NPM 代理
    pub npm_proxy: Option<String>,
    /// 本地存储目录（Iris 内置包管理器）
    pub local_storage_dir: Option<String>,

    // ── Mock API Server ────────────────────────────────
    /// 是否启用 Mock 服务器
    pub mock_enabled: bool,
    /// 模拟延迟（毫秒）
    pub mock_delay_ms: u64,
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
            // AI 云服务
            ai_provider: "openai".into(),
            ai_api_key: String::new(),
            ai_model: "gpt-4o".into(),
            ai_endpoint: "https://api.openai.com/v1".into(),
            // AI 本地模型
            ai_model_repo: "Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF".into(),
            ai_model_file: "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf".into(),
            ai_device: Self::detect_optimal_device(),
            ai_temperature: Self::default_temperature(),
            ai_max_tokens: Self::default_max_tokens(),
            ai_model_downloaded: false,
            // Iris 内置包管理器
            npm_registry: "https://registry.npmjs.org/".into(),
            npm_proxy: None,
            local_storage_dir: None,
            // Mock
            mock_enabled: false,
            mock_delay_ms: 0,
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

    /// 检测系统最优推理设备
    pub fn detect_optimal_device() -> String {
        #[cfg(target_os = "windows")]
        {
            // 检测 NVIDIA CUDA
            if std::path::Path::new("C:\\Windows\\System32\\nvcuda.dll").exists() {
                return "cuda".into();
            }
            // 检测 Vulkan
            if std::path::Path::new("C:\\Windows\\System32\\vulkan-1.dll").exists() {
                return "vulkan".into();
            }
        }
        "cpu".into()
    }

    pub fn default_temperature() -> f32 {
        0.15
    }

    pub fn default_max_tokens() -> usize {
        4096
    }

    /// 按分区恢复默认值
    /// section: "general" / "ai" / "npm" / "mock"
    pub fn reset_section(&mut self, section: &str) {
        let defaults = DaemonConfig::default();
        match section {
            "general" => {
                self.http_port = defaults.http_port;
                self.mock_port = defaults.mock_port;
                self.daemon_port = defaults.daemon_port;
                self.show_icon = defaults.show_icon;
                self.auto_start_server = defaults.auto_start_server;
            }
            "ai" => {
                self.ai_provider = defaults.ai_provider;
                self.ai_api_key = defaults.ai_api_key;
                self.ai_model = defaults.ai_model;
                self.ai_endpoint = defaults.ai_endpoint;
                self.ai_model_repo = defaults.ai_model_repo;
                self.ai_model_file = defaults.ai_model_file;
                self.ai_device = defaults.ai_device;
                self.ai_temperature = defaults.ai_temperature;
                self.ai_max_tokens = defaults.ai_max_tokens;
            }
            "npm" => {
                self.npm_registry = defaults.npm_registry;
                self.npm_proxy = defaults.npm_proxy;
                self.local_storage_dir = defaults.local_storage_dir;
            }
            "mock" => {
                self.mock_enabled = defaults.mock_enabled;
                self.mock_delay_ms = defaults.mock_delay_ms;
            }
            _ => {}
        }
        self.save();
    }
}
