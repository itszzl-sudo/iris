//! `iris-ai edit` 子命令 — 用 AI 编辑代码文件

use std::fs;

use anyhow::{Result, Context};
use colored::Colorize;
use iris_ai::{AiAssistant, AiConfig};
use tracing::info;

/// 运行 edit 子命令
pub async fn run(
    file_path: String,
    instruction: String,
    output: Option<String>,
    dry_run: bool,
    model_path: Option<String>,
    temperature: f32,
) -> Result<()> {
    // 1. 读取源文件
    let code = fs::read_to_string(&file_path)
        .with_context(|| format!("无法读取文件: {}", file_path))?;

    let file_len = code.len();
    let file_lines = code.lines().count();
    info!("📄 读取文件: {} ({} 行, {} bytes)", file_path, file_lines, file_len);

    // 2. 构建 AI 配置
    let mut config = AiConfig::default()
        .with_temperature(temperature);

    if let Some(path) = model_path {
        config = config.with_model_path(path);
    }

    // 3. 初始化 AI 助手
    println!("{}", "🧠 初始化 AI 引擎...".cyan());
    let mut assistant = AiAssistant::new(config).build()?;

    println!("{}", "🤖 AI 正在分析代码...".cyan());
    let response = assistant.edit_code(&file_path, &instruction, &code)?;

    // 4. 处理结果
    if response.is_empty() {
        anyhow::bail!("AI 返回了空响应，请重试");
    }

    // 显示 diff
    print_diff(&file_path, &code, &response);

    // 5. 写入文件
    if dry_run {
        println!("\n{}", "🔍 干运行模式 — 未写入文件".yellow());
        println!("  使用 {} 查看完整输出", "--dry-run".yellow());
    } else {
        let target = output.unwrap_or_else(|| file_path.clone());
        fs::write(&target, &response)
            .with_context(|| format!("无法写入文件: {}", target))?;

        let new_len = response.len();
        let delta = new_len as i64 - file_len as i64;
        let delta_str = if delta >= 0 {
            format!("+{} bytes", delta).green()
        } else {
            format!("{} bytes", delta).red()
        };
        println!("\n{} 已写入 {} ({})", "✅".green(), target, delta_str);
    }

    println!("\n{}", "✨ 完成".green());
    Ok(())
}

/// 打印代码 diff（简化版：只显示新旧文件变化摘要）
fn print_diff(file_path: &str, old_code: &str, new_code: &str) {
    if old_code == new_code {
        println!("{}", "⚠️  代码未发生变化".yellow());
        return;
    }

    let old_lines: Vec<_> = old_code.lines().collect();
    let new_lines: Vec<_> = new_code.lines().collect();

    let added = new_lines.len() as i64 - old_lines.len() as i64;
    let added_str = if added >= 0 {
        format!("+{}", added).green()
    } else {
        format!("{}", added).red()
    };

    println!("\n{} {} ({} lines → {} lines, {})",
        "📊 改动摘要:".blue(),
        file_path,
        old_lines.len(),
        new_lines.len(),
        added_str,
    );

    // 简单的行级别 diff（仅打印不一致的行）
    let max_lines = old_lines.len().max(new_lines.len());
    for i in 0..max_lines {
        let old_line = old_lines.get(i);
        let new_line = new_lines.get(i);

        match (old_line, new_line) {
            (Some(o), Some(n)) if o != n => {
                println!("  {} {}: {}", "-".red(), i + 1, o);
                println!("  {} {}: {}", "+".green(), i + 1, n);
            }
            (Some(o), None) => {
                println!("  {} {}: {}", "-".red(), i + 1, o);
            }
            (None, Some(n)) => {
                println!("  {} {}: {}", "+".green(), i + 1, n);
            }
            _ => {} // 相同行，忽略
        }
    }
}
