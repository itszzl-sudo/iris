//! Iris Runtime CLI
//!
//! 命令行工具用于构建和开发基于 Iris 运行时 的 Vue 3 应用程序
//!
//! # 使用方法
//!
//! ```bash
//! # 开发模式（带热重载）
//! iris-runtime dev
//!
//! # 生产构建
//! iris-runtime build
//!
//! # 查看信息
//! iris-runtime info
//! ```

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process;

mod commands;
mod config;
mod utils;

use commands::{build::BuildCommand, dev::DevCommand, info::InfoCommand};

/// Iris Runtime CLI - Build and develop Vue 3 applications with Rust+WebGPU runtime
#[derive(Parser)]
#[command(name = "iris-runtime")]
#[command(author = "Iris Team")]
#[command(version = "0.1.0")]
#[command(about = "Iris Runtime CLI for Vue 3 applications", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start development server with hot reload
    Dev(DevCommand),
    
    /// Build for production
    Build(BuildCommand),
    
    /// Show project information
    Info(InfoCommand),
}

fn main() {
    let cli = Cli::parse();

    // 初始化日志
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    // 打印 Iris 标志
    print_iris_logo();

    // 执行命令
    let result = match cli.command {
        Commands::Dev(cmd) => cmd.execute(),
        Commands::Build(cmd) => cmd.execute(),
        Commands::Info(cmd) => cmd.execute(),
    };

    if let Err(err) = result {
        eprintln!("\n{}", "Error:".red().bold());
        eprintln!("{}\n", err);
        process::exit(1);
    }
}

/// 打印 Iris 标志
fn print_iris_logo() {
    println!("{}", "╔══════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║                                                          ║".bright_cyan());
    println!("{}", "║   🌈  Iris Runtime CLI v0.1.0                           ║".bright_cyan());
    println!("{}", "║   Vue 3 Applications with Rust + WebGPU                 ║".bright_cyan());
    println!("{}", "║                                                          ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
}
