//! AI 助手配置

use std::path::PathBuf;

/// 推理设备
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AiDevice {
    /// CPU 推理（默认）
    Cpu,
    /// GPU (CUDA) 推理
    Gpu,
}

/// AI 助手配置
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// GGUF 模型文件路径（设置后不再自动下载）
    pub model_path: Option<PathBuf>,
    /// HuggingFace 模型仓库（自动下载时使用）
    pub model_repo: String,
    /// GGUF 文件名（自动下载时使用）
    pub model_file: String,
    /// 推理设备
    pub device: AiDevice,
    /// 温度参数（0.0~1.0，越低越精确）
    pub temperature: f32,
    /// Top-p 采样参数
    pub top_p: f32,
    /// 最大生成 token 数
    pub max_tokens: usize,
    /// 模型缓存目录
    pub cache_dir: Option<PathBuf>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            // Qwen2.5-Coder-0.5B Q4_K_M，约 350MB，CPU 可流畅运行
            model_repo: "Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF".to_string(),
            model_file: "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf".to_string(),
            device: AiDevice::Cpu,
            temperature: 0.15,
            top_p: 0.9,
            max_tokens: 4096,
            cache_dir: None,
        }
    }
}

impl AiConfig {
    /// 设置模型文件路径（跳过自动下载）
    pub fn with_model_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.model_path = Some(path.into());
        self
    }

    /// 设置 HuggingFace 仓库和文件名
    pub fn with_model_repo(mut self, repo: impl Into<String>, file: impl Into<String>) -> Self {
        self.model_repo = repo.into();
        self.model_file = file.into();
        self
    }

    /// 设置推理设备
    pub fn with_device(mut self, device: AiDevice) -> Self {
        self.device = device;
        self
    }

    /// 设置温度
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 1.0);
        self
    }

    /// 设置模型缓存目录
    pub fn with_cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(dir.into());
        self
    }
}
