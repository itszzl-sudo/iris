//! 开发服务器命令

use clap::Args;
use anyhow::Result;
use colored::Colorize;
use crate::config::IrisConfig;
use crate::utils::{self, print_success, print_info};

/// 开发服务器命令参数
#[derive(Args)]
pub struct DevCommand {
    /// 项目根目录
    #[arg(short, long, default_value = ".")]
    pub root: String,
    
    /// 开发服务器端口
    #[arg(short, long)]
    pub port: Option<u16>,
    
    /// 禁用热重载
    #[arg(long)]
    pub no_hot_reload: bool,
    
    /// 自动打开浏览器
    #[arg(short, long)]
    pub open: bool,
}

impl DevCommand {
    pub fn execute(&self) -> Result<()> {
        println!("{}", "Starting development server...".bright_cyan().bold());
        println!();
        
        // 找到项目根目录
        let project_root = utils::find_project_root(std::path::Path::new(&self.root))?;
        print_success(&format!("Project root: {}", project_root.display()));
        
        // 加载配置
        let mut config = IrisConfig::load(&project_root)?;
        
        // 覆盖配置
        if let Some(port) = self.port {
            config.dev_server.port = port;
        }
        if self.no_hot_reload {
            config.dev_server.hot_reload = false;
        }
        if self.open {
            config.dev_server.open = true;
        }
        
        // 显示配置
        self.print_config(&config);
        
        // 检测项目类型
        let project_type = IrisConfig::detect_project_type(&project_root);
        match project_type {
            crate::config::ProjectType::Vue3 => {
                print_success("Detected Vue 3 project");
            }
            crate::config::ProjectType::Unknown => {
                utils::print_warning("Unknown project type, using default configuration");
            }
        }
        
        println!();
        print_info("Development server would start here");
        print_info("In production, this would:");
        println!("  1. Compile Vue SFC files");
        println!("  2. Start WebGPU renderer");
        println!("  3. Initialize JavaScript runtime");
        println!("  4. Setup file watcher for hot reload");
        println!("  5. Open browser window");
        println!();
        print_success("Development mode ready!");
        println!();
        
        Ok(())
    }
    
    fn print_config(&self, config: &IrisConfig) {
        println!("{}", "Configuration:".bright_cyan().bold());
        println!("  Project: {}", config.name);
        println!("  Version: {}", config.version);
        println!("  Source:  {}", config.src_dir.display());
        println!("  Output:  {}", config.out_dir.display());
        println!("  Entry:   {}", config.entry);
        println!("  Port:    {}", config.dev_server.port);
        println!(
            "  Hot Reload: {}",
            if config.dev_server.hot_reload { "Yes" } else { "No" }
        );
        println!(
            "  Open Browser: {}",
            if config.dev_server.open { "Yes" } else { "No" }
        );
        println!();
    }
}
