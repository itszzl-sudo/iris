//! `iris-ai download` 子命令 — 下载 GGUF 模型（支持断点续传）

use std::time::Instant;

use anyhow::Result;
use colored::Colorize;
use iris_ai::{AiConfig, ModelDownloader, DownloadProgress, DownloadStatus};
use tracing::info;

/// 运行 download 子命令
pub async fn run(
    show_progress: bool,
    repo: Option<String>,
    file: Option<String>,
) -> Result<()> {
    let mut config = AiConfig::default();

    if let Some(r) = repo {
        if let Some(f) = file {
            config = config.with_model_repo(r, f);
        }
    }

    let cache_dir = config.cache_dir.clone().unwrap_or_else(|| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        std::path::Path::new(&home).join(".cache").join("iris-ai")
    });

    let mut downloader = ModelDownloader::new(
        &config.model_repo,
        &config.model_file,
        &cache_dir,
    );

    if show_progress {
        downloader = downloader.with_progress_callback(move |p| {
            print_progress(p);
        });
    }

    info!(
        "🌐 开始下载: {}/{}",
        config.model_repo, config.model_file
    );
    println!("{}", "📥 正在下载模型...".cyan());
    println!("  仓库: {}", config.model_repo);
    println!("  文件: {}", config.model_file);
    println!("  缓存: {}", cache_dir.display());

    let start = Instant::now();
    let result = downloader.get_or_download()?;
    let elapsed = start.elapsed();

    let size_mb = result.file_size as f64 / 1_000_000.0;
    let action = if result.resumed { "续传" } else { "下载" };

    println!(
        "\n{} {}完成: {:.1} MB (耗时 {:.1}s)",
        "✅".green(),
        action,
        size_mb,
        elapsed.as_secs_f64()
    );
    println!("  路径: {}", result.model_path.display());

    Ok(())
}

/// 打印下载进度
fn print_progress(p: &DownloadProgress) {
    // 每 5% 打印一次
    let prev_pct = unsafe { LAST_PCT };
    let current_pct = (p.percentage / 5.0).floor() as u32 * 5;
    if current_pct > prev_pct {
        unsafe { LAST_PCT = current_pct };
        print!("\r  {} {:3.0}% | {} | {}  ",
            match p.status {
                DownloadStatus::Resuming => "🔄".yellow(),
                DownloadStatus::Completed => "✅".green(),
                _ => "⬇️".cyan(),
            },
            p.percentage,
            p.speed_display,
            p.eta_display,
        );
    }

    // 完成时换行
    if p.status == DownloadStatus::Completed {
        println!();
    }
}

/// 上一个打印的百分比（仅用于抑制频繁输出）
static mut LAST_PCT: u32 = 0;
