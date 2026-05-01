//! `iris-ai review` 子命令 — 用 AI 审查代码质量
//!
//! # 用法
//!
//! ```bash
//! # 审查单个文件
//! iris-ai review src/App.vue
//!
//! # 审查多个文件
//! iris-ai review src/*.vue src/*.ts
//!
//! # JSON 格式输出
//! iris-ai review src/App.vue --format json
//! ```

use std::fs;
use std::time::Instant;

use anyhow::{Result, Context};
use colored::Colorize;
use iris_ai::{AiAssistant, AiConfig};
use iris_ai::review::{ReviewReport, ReviewSeverity};

/// 运行 review 子命令
pub async fn run(
    files: Vec<String>,
    format: String,
    model_path: Option<String>,
    temperature: f32,
) -> Result<()> {
    let start = Instant::now();

    // 1. 构建 AI 配置
    let mut config = AiConfig::default()
        .with_temperature(temperature);
    if let Some(path) = model_path {
        config = config.with_model_path(path);
    }

    // 2. 初始化 AI 助手
    println!("{}", "🧠 初始化 AI 引擎...".cyan());
    let mut assistant = AiAssistant::new(config).build()?;

    // 3. 逐个审查文件
    let mut all_reports: Vec<ReviewReport> = Vec::new();
    let total_files = files.len();

    for (i, file_path) in files.iter().enumerate() {
        println!(
            "\n{} [{}/{}] 审查: {}",
            "📄".cyan(),
            i + 1,
            total_files,
            file_path.bold()
        );

        let code = fs::read_to_string(file_path)
            .with_context(|| format!("无法读取文件: {}", file_path))?;

        let report = assistant.review_code(file_path, &code)?;
        all_reports.push(report);
    }

    // 4. 输出审查结果
    let total_elapsed = start.elapsed().as_secs_f64();

    match format.as_str() {
        "json" => print_json(&all_reports, total_elapsed)?,
        _ => print_text(&all_reports, total_elapsed),
    }

    Ok(())
}

/// 以文本格式输出审查报告
fn print_text(reports: &[ReviewReport], total_elapsed: f64) {
    let mut total_critical = 0usize;
    let mut total_warning = 0usize;
    let mut total_info = 0usize;
    let mut total_suggestion = 0usize;

    for report in reports {
        let counts = report.severity_counts();
        total_critical += counts[0];
        total_warning += counts[1];
        total_info += counts[2];
        total_suggestion += counts[3];

        println!("\n{}", "═══════════════════════════════════════".cyan());
        println!("  {} {}", "📋 审查报告:".bold().white(), report.file_path.bold());
        println!("{}", "═══════════════════════════════════════".cyan());

        // 概要
        println!("\n  {} {}", "📝 总体评价:".bold(), report.summary.cyan());

        // 问题统计
        let total = report.issues.len();
        if total > 0 {
            println!("\n  {} 共发现 {} 个问题:", "🔍".bold(), total);
            if counts[0] > 0 {
                println!("    {} {} {}", "🔴".red(), counts[0], "严重".red());
            }
            if counts[1] > 0 {
                println!("    {} {} {}", "🟡".yellow(), counts[1], "警告".yellow());
            }
            if counts[2] > 0 {
                println!("    {} {} {}", "🔵".cyan(), counts[2], "提示".cyan());
            }
            if counts[3] > 0 {
                println!("    {} {} {}", "🟢".green(), counts[3], "建议".green());
            }
        } else {
            println!("\n  {} 代码质量良好，未发现明显问题。", "✅".green());
        }

        // 逐个输出问题
        for issue in report.sorted_issues() {
            let sev_color: fn(&str) -> colored::ColoredString = match issue.severity {
                ReviewSeverity::Critical => |s| s.red(),
                ReviewSeverity::Warning => |s| s.yellow(),
                ReviewSeverity::Info => |s| s.cyan(),
                ReviewSeverity::Suggestion => |s| s.green(),
            };

            println!(
                "\n  {} {} [{}]",
                sev_color("●"),
                issue.severity.label(),
                issue.issue_type.label(),
            );

            if let Some((start, end)) = issue.line_range {
                if start == end {
                    println!("     {} 第 {} 行", "📍".cyan(), start);
                } else {
                    println!("     {} 第 {}-{} 行", "📍".cyan(), start, end);
                }
            }

            if !issue.description.is_empty() {
                println!("    {}", issue.description);
            }

            if !issue.suggestion.is_empty() {
                println!("    {} {}", "💡".yellow(), issue.suggestion.yellow());
            }
        }

        // 审查耗时
        println!(
            "\n  ⏱  {} 耗时 {:.1}s",
            "审查完成,".cyan(),
            report.elapsed_secs
        );
    }

    // 全局统计
    println!("\n{}", "═══════════════════════════════════════".cyan());
    println!("  {} 审查统计", "📊".bold().white());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("  审查文件: {} 个", reports.len());
    println!(
        "  发现问题: {} 个 (严重 {} / 警告 {} / 提示 {} / 建议 {})",
        total_critical + total_warning + total_info + total_suggestion,
        total_critical, total_warning, total_info, total_suggestion,
    );
    println!("  总耗时: {:.1}s", total_elapsed);
}

/// 以 JSON 格式输出审查报告
fn print_json(reports: &[ReviewReport], total_elapsed: f64) -> Result<()> {
    let output = serde_json::json!({
        "summary": {
            "files_reviewed": reports.len(),
            "total_elapsed_secs": total_elapsed,
            "total_issues": reports.iter().map(|r| r.issues.len()).sum::<usize>(),
        },
        "reports": reports.iter().map(|r| {
            let counts = r.severity_counts();
            serde_json::json!({
                "file": r.file_path,
                "summary": r.summary,
                "elapsed_secs": r.elapsed_secs,
                "severity_counts": {
                    "critical": counts[0],
                    "warning": counts[1],
                    "info": counts[2],
                    "suggestion": counts[3],
                },
                "issues": r.issues.iter().map(|i| {
                    serde_json::json!({
                        "severity": format!("{:?}", i.severity),
                        "issue_type": i.issue_type.label(),
                        "line_start": i.line_range.map(|(s, _)| s),
                        "line_end": i.line_range.map(|(_, e)| e),
                        "description": i.description,
                        "suggestion": i.suggestion,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
