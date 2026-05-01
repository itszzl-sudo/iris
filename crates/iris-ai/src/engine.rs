//! candle 推理引擎
//!
//! 加载 GGUF 量化模型，在本地运行 LLM 推理。
//! 使用 candle 0.10 量化 API：
//! - `candle_core::quantized::gguf_file::Content` 读取 GGUF 文件头
//! - `candle_transformers::models::quantized_qwen2::ModelWeights` 加载 Qwen2 权重
//! - KV-cache 自回归生成

use std::path::Path;
use std::fs::File;

use anyhow::{Result, Context};
use tracing::{info, debug, warn};
use candle_core::quantized::gguf_file;
use candle_core::quantized::tokenizer::TokenizerFromGguf;
use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;

use crate::config::{AiConfig, AiDevice};
use crate::downloader::ModelDownloader;

/// 推理引擎
pub struct InferenceEngine {
    config: AiConfig,
    model_loaded: bool,
}

impl InferenceEngine {
    /// 创建推理引擎
    pub fn new(config: AiConfig) -> Self {
        Self { config, model_loaded: false }
    }

    /// 加载模型（验证 GGUF 文件头 + 自动下载）
    pub fn load(&mut self) -> Result<()> {
        let model_path = self.resolve_model_path()?;
        let meta = std::fs::metadata(&model_path)
            .context("无法读取模型文件")?;
        let size_mb = meta.len() as f64 / (1024.0 * 1024.0);

        if size_mb < 10.0 {
            warn!("模型文件过小 ({:.1} MB)，可能无效", size_mb);
        }

        // 验证 GGUF 文件头
        {
            let mut file = File::open(&model_path)
                .context("无法打开模型文件进行验证")?;
            let content = gguf_file::Content::read(&mut file)
                .context("GGUF 文件头验证失败 — 文件可能已损坏或不是有效的 GGUF 模型")?;
            let metadata_count = content.metadata.len();
            let tensor_count = content.tensor_infos.len();
            info!(
                "📋 GGUF 模型: {} 个元数据项, {} 个张量",
                metadata_count, tensor_count,
            );
            // 尝试读取架构信息
            if let Some(arch) = content.metadata.get("general.architecture") {
                info!("  架构: {:?}", arch);
            }
        }

        info!(
            "✅ 模型就绪: {} ({:.1} MB, {:?})",
            model_path.display(), size_mb, self.config.device
        );
        self.model_loaded = true;
        Ok(())
    }

    /// 模型是否已加载
    pub fn is_loaded(&self) -> bool {
        self.model_loaded
    }

    /// 运行推理
    ///
    /// 给定完整的 prompt（已包含 system/user/assistant 格式标记），
    /// 生成模型补全文本。
    pub fn generate(&mut self, prompt: &str) -> Result<String> {
        if !self.model_loaded {
            anyhow::bail!("模型未加载，请先调用 load()");
        }

        let model_path = self.resolve_model_path()?;
        let device = self.get_device()?;

        // 1. 打开 GGUF 文件并解析文件头
        let mut file = File::open(&model_path)
            .context("无法打开模型文件")?;
        let content = gguf_file::Content::read(&mut file)
            .context("无法读取 GGUF 文件头")?;

        // 2. 从 GGUF 元数据创建 tokenizer
        let tokenizer = tokenizers::Tokenizer::from_gguf(&content)
            .context("无法从 GGUF 创建 tokenizer")?;

        let eos = tokenizer.token_to_id("").unwrap_or(151645);

        // 3. 加载量化模型权重
        let mut model = candle_transformers::models::quantized_qwen2::ModelWeights::from_gguf(
            content, &mut file, &device,
        )
        .context("无法加载模型权重")?;

        drop(file); // 不再需要文件句柄

        // 4. 编码 prompt
        let encoding = tokenizer.encode(prompt, true)
            .map_err(|e| anyhow::anyhow!("编码 prompt 失败: {}", e))?;
        let input_ids = encoding.get_ids().to_vec();
        debug!("输入 token 数: {}", input_ids.len());

        let prompt_len = input_ids.len();
        let max_tokens = self.config.max_tokens.min(4096);

        // 5. 自回归生成
        let mut generated: Vec<u32> = Vec::new();
        let mut next_token: u32;
        let mut index_pos: usize = 0;

        // 首次前向：传入完整 prompt
        {
            let input = Tensor::new(input_ids.as_slice(), &device)?
                .unsqueeze(0)?;
            let logits = model.forward(&input, index_pos)?;
            index_pos = prompt_len;
            let logits = logits.squeeze(0)?;
            let mut lp = LogitsProcessor::new(42,
                Some(self.config.temperature as f64),
                Some(self.config.top_p as f64),
            );
            next_token = lp.sample(&logits)?;
        }

        // 逐 token 生成
        for i in 0..max_tokens {
            if next_token == eos {
                debug!("遇到 EOS token，停止生成");
                break;
            }
            generated.push(next_token);

            let input = Tensor::new(&[next_token], &device)?
                .unsqueeze(0)?;
            let logits = model.forward(&input, index_pos)?;
            index_pos += 1;

            let logits = logits.squeeze(0)?;
            let mut lp = LogitsProcessor::new(42,
                Some(self.config.temperature as f64),
                Some(self.config.top_p as f64),
            );
            next_token = lp.sample(&logits)?;

            if i > 0 && i % 100 == 0 {
                debug!("已生成 {} tokens...", i + 1);
            }
        }

        // 6. 解码
        let output = tokenizer.decode(&generated, true)
            .map_err(|e| anyhow::anyhow!("解码生成文本失败: {}", e))?;
        debug!("推理完成: {} tokens", generated.len());
        Ok(output)
    }

    /// 获取推理设备
    fn get_device(&self) -> Result<Device> {
        match self.config.device {
            AiDevice::Cpu => Ok(Device::Cpu),
            AiDevice::Gpu => {
                Device::new_cuda(0)
                    .or_else(|_| {
                        warn!("CUDA 不可用，回退到 CPU");
                        Ok(Device::Cpu)
                    })
            }
        }
    }

    /// 解析模型文件路径（自动下载）
    fn resolve_model_path(&self) -> Result<std::path::PathBuf> {
        if let Some(ref path) = self.config.model_path {
            if path.exists() {
                return Ok(path.clone());
            }
            anyhow::bail!("模型文件不存在: {}", path.display());
        }

        let cache = self.config.cache_dir.clone().unwrap_or_else(|| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".into());
            Path::new(&home).join(".cache").join("iris-ai")
        });

        let dl = ModelDownloader::new(
            &self.config.model_repo,
            &self.config.model_file,
            &cache,
        );
        let result = dl.get_or_download()?;
        Ok(result.model_path)
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AiConfig;

    fn test_config() -> AiConfig {
        AiConfig::default()
            .with_model_path(std::path::PathBuf::from("/tmp/non-existent-test-model.gguf"))
    }

    #[test]
    fn test_engine_new_not_loaded() {
        let engine = InferenceEngine::new(test_config());
        assert!(!engine.is_loaded(), "引擎初始状态应为未加载");
    }

    #[test]
    fn test_engine_load_fails_nonexistent() {
        let mut engine = InferenceEngine::new(test_config());
        let result = engine.load();
        assert!(result.is_err(), "不存在的文件应返回错误");
        assert!(!engine.is_loaded(), "加载失败后仍应标记为未加载");
    }

    #[test]
    fn test_engine_load_fails_invalid_file() {
        let dir = std::env::temp_dir().join("iris-ai-test-engine");
        let _ = std::fs::create_dir_all(&dir);
        let fake_path = dir.join("fake-model.gguf");
        // 写入无效数据（不是 GGUF 格式）
        std::fs::write(&fake_path, b"NOT_A_GGUF_FILE\x00\x01\x02\x03").unwrap();

        let mut engine = InferenceEngine::new(
            AiConfig::default().with_model_path(fake_path.clone())
        );
        let result = engine.load();
        assert!(result.is_err(), "无效 GGUF 文件应返回错误");
        assert!(
            format!("{:?}", result).contains("GGUF"),
            "错误信息应包含 GGUF 相关描述: {:?}",
            result
        );
        assert!(!engine.is_loaded());

        // 清理
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_fails_when_not_loaded() {
        let mut engine = InferenceEngine::new(test_config());
        let result = engine.generate("test prompt");
        assert!(result.is_err(), "未加载模型时应返回错误");
        assert!(
            format!("{:?}", result).contains("未加载"),
            "错误信息应提示模型未加载"
        );
    }
}
