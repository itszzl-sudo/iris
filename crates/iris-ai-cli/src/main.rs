//! Iris AI CLI — 本地 AI 代码编辑助手
//!
//! 基于 Qwen2.5-Coder GGUF 量化模型，在本地运行 AI 推理，
//! 为 Vue / CSS / JS / TS 代码提供 AI 辅助编辑能力。
//!
//! # 用法
//!
//! ```bash
//! # 编辑文件（直接修改）
//! iris-ai edit src/App.vue "把按钮颜色改为蓝色"
//!
//! # 编辑文件（输出到新文件）
//! iris-ai edit src/App.vue "添加一个计数器" -o src/App_new.vue
//!
//! # 查看改动但不写入
//! iris-ai edit src/style.css "让所有文字居中" --dry-run
//!
//! # 下载模型
//! iris-ai download --progress
//!
//! # 查看信息
//! iris-ai info
//! ```

mod commands;

use clap::{Parser, Subcommand};
use anyhow::Result;

/// Iris AI CLI - 本地 AI 代码编辑助手
#[derive(Parser)]
#[command(name = "iris-ai")]
#[command(about = "Local AI code editing assistant (Qwen2.5-Coder GGUF)", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 编辑文件 — 用 AI 修改代码并写回
    Edit {
        /// 目标文件路径
        file: String,

        /// 修改指令（如 "把按钮颜色改为蓝色"）
        instruction: String,

        /// 输出文件路径（默认直接修改源文件）
        #[arg(short, long)]
        output: Option<String>,

        /// 只显示 diff，不修改文件
        #[arg(long)]
        dry_run: bool,

        /// GGUF 模型文件路径（可选，跳过自动下载）
        #[arg(long)]
        model_path: Option<String>,

        /// 温度参数 0.0~1.0（越低越精确，默认 0.15）
        #[arg(long, default_value_t = 0.15)]
        temperature: f32,
    },

    /// 下载模型 — 支持断点续传和实时进度
    Download {
        /// 显示实时下载进度条
        #[arg(short, long)]
        progress: bool,

        /// HuggingFace 仓库名（可选）
        #[arg(long)]
        repo: Option<String>,

        /// GGUF 文件名（可选）
        #[arg(long)]
        file: Option<String>,
    },

    /// 显示 AI 助手配置信息
    Info,

    /// 审查代码 — 用 AI 分析代码质量并发现问题
    Review {
        /// 要审查的文件路径（支持多个）
        #[arg(required = true)]
        files: Vec<String>,

        /// 输出格式 (text / json)
        #[arg(long, default_value = "text")]
        format: String,

        /// GGUF 模型文件路径（可选，跳过自动下载）
        #[arg(long)]
        model_path: Option<String>,

        /// 温度参数 0.0~1.0（越低越精确，默认 0.15）
        #[arg(long, default_value_t = 0.15)]
        temperature: f32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive("iris_ai=info".parse()?)
                .from_env_lossy(),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Edit { file, instruction, output, dry_run, model_path, temperature } => {
            commands::edit::run(file, instruction, output, dry_run, model_path, temperature).await?
        }
        Commands::Download { progress, repo, file } => {
            commands::download::run(progress, repo, file).await?
        }
        Commands::Info => {
            commands::info::run()?
        }
        Commands::Review { files, format, model_path, temperature } => {
            commands::review::run(files, format, model_path, temperature).await?
        }
    }

    Ok(())
}
