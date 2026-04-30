//! Iris AI — 本地小型语言模型集成
//!
//! 使用 candle + GGUF 在本地运行 Qwen2.5-Coder 模型，
//! 为 Vue / CSS / JS / TS 代码提供 AI 辅助编辑能力。
//!
//! # 架构
//!
//! ```text
//! iris-ai
//!   ├── config.rs      - AiConfig / AiDevice
//!   ├── downloader.rs  - 断点续传/进度/网速的模型下载器
//!   ├── engine.rs      - candle 推理引擎
//!   └── prompt.rs      - code-edit prompt 模板
//! ```

#![warn(missing_docs)]

mod config;
pub mod downloader;
mod engine;
pub mod prompt;

pub use config::{AiConfig, AiDevice};
pub use downloader::{ModelDownloader, DownloadProgress, DownloadStatus, DownloadResult};
pub use engine::InferenceEngine;

use anyhow::Result;
use tracing::info;

/// AI 助手 — 代码修改的主入口
pub struct AiAssistant {
    config: AiConfig,
    engine: Option<InferenceEngine>,
}

impl AiAssistant {
    /// 创建新的 AI 助手
    pub fn new(config: AiConfig) -> Self {
        Self { config, engine: None }
    }

    /// 构建并初始化（会自动下载模型 ~350MB）
    pub fn build(mut self) -> Result<Self> {
        info!("=== Iris AI Assistant ===");
        info!("  Model: {}/{}", self.config.model_repo, self.config.model_file);
        info!("  Device: {:?}", self.config.device);

        let mut engine = InferenceEngine::new(self.config.clone());
        engine.load()?;
        self.engine = Some(engine);
        info!("✅ AI Assistant ready");
        Ok(self)
    }

    /// 执行代码修改
    ///
    /// * `file_path` - 源文件路径（用于检测语言类型）
    /// * `instruction` - 修改指令
    /// * `code` - 源代码内容
    pub fn edit_code(&mut self, file_path: &str, instruction: &str, code: &str) -> Result<String> {
        let engine = self.engine.as_mut()
            .ok_or_else(|| anyhow::anyhow!("AI not initialized"))?;
        let prompt = prompt::build_code_edit_prompt(file_path, instruction, code);
        info!("AI editing: {}", file_path);
        let response = engine.generate(&prompt)?;
        let extracted = prompt::extract_code_from_response(&response);
        Ok(extracted.to_string())
    }

    /// 检查 AI 模型是否已加载就绪
    pub fn is_ready(&self) -> bool {
        self.engine.as_ref().map(|e| e.is_loaded()).unwrap_or(false)
    }
}
