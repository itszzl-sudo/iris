//! `iris-ai info` 子命令 — 显示 AI 助手配置信息

use anyhow::Result;
use colored::Colorize;
use iris_ai::AiConfig;

/// 运行 info 子命令
pub fn run() -> Result<()> {
    let config = AiConfig::default();

    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "  Iris AI — 本地代码编辑助手".bold().cyan());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();
    println!("  {}  {}", "模型仓库:".bold(), config.model_repo);
    println!("  {}  {}", "模型文件:".bold(), config.model_file);
    println!("  {}  {:?}", "推理设备:".bold(), config.device);
    println!("  {}  {:.2}", "温度参数:".bold(), config.temperature);
    println!("  {}  {:.2}", "Top-p:".bold(), config.top_p);
    println!("  {}  {}", "最大 Token:".bold(), config.max_tokens);
    println!();
    println!("  {}  ", "支持的文件类型:".bold());
    println!("    • Vue SFC   (.vue)");
    println!("    • CSS/SCSS  (.css .scss .less)");
    println!("    • JavaScript (.js .jsx .mjs)");
    println!("    • TypeScript (.ts .tsx .mts)");
    println!("    • HTML      (.html .htm)");
    println!();
    println!("  {}  ", "缓存目录:".bold());
    let cache = config.cache_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        std::path::Path::new(&home).join(".cache").join("iris-ai")
    });
    println!("    {}", cache.display());

    // 检查模型是否已下载
    let model_cache = cache.join(&config.model_file);
    if model_cache.exists() {
        let size = std::fs::metadata(&model_cache)
            .map(|m| m.len() as f64 / 1_000_000.0)
            .unwrap_or(0.0);
        println!("    {} (已缓存: {:.1} MB)", "✅".green(), size);
    } else {
        println!("    {} (未下载，运行 iris-ai download 获取)", "⬜".yellow());
    }

    println!();
    println!("  {}  {}", "使用示例:".bold(), "iris-ai edit <文件> \"<指令>\"".cyan());
    println!();

    Ok(())
}
