//! Iris JetCrab CLI
//!
//! Vue 项目开发服务器（运行时按需编译）
//! 
//! 架构：
//! - iris-jetcrab-cli: HTTP 服务器 + 路由处理
//! - iris-jetcrab-engine: 编译引擎（与 iris-engine 对等）

mod server;
mod utils;

use clap::{Parser, Subcommand};
use anyhow::Result;

/// Iris JetCrab CLI - Vue 项目开发工具
#[derive(Parser)]
#[command(name = "iris-jetcrab")]
#[command(about = "Vue project development server (runtime on-demand compilation)", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动开发服务器
    Dev {
        /// 项目根目录
        #[arg(short, long, default_value = ".")]
        root: String,

        /// 开发服务器端口
        #[arg(short, long, default_value_t = 3000)]
        port: u16,

        /// 自动打开浏览器
        #[arg(short, long)]
        open: bool,

        /// 禁用热更新
        #[arg(long)]
        no_hmr: bool,

        /// 调试模式
        #[arg(short, long)]
        debug: bool,
    },

    /// 显示项目信息
    Info {
        /// 项目根目录
        #[arg(short, long, default_value = ".")]
        root: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev { root, port, open, no_hmr, debug } => {
            server::start(root, port, open, !no_hmr, debug).await
        }
        Commands::Info { root } => {
            utils::print_project_info(&root)
        }
    }
}
