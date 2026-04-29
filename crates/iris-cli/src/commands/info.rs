//! 项目信息命令

use clap::Args;
use anyhow::Result;
use colored::Colorize;
use crate::config::IrisConfig;
use crate::utils::{self, print_success, print_warning};

/// 项目信息命令参数
#[derive(Args)]
pub struct InfoCommand {
    /// 项目根目录
    #[arg(short, long, default_value = ".")]
    pub root: String,
}

impl InfoCommand {
    pub fn execute(&self) -> Result<()> {
        println!("{}", "Project Information:".bright_cyan().bold());
        println!();
        
        // 找到项目根目录
        let project_root = utils::find_project_root(std::path::Path::new(&self.root))?;
        print_success(&format!("Project root: {}", project_root.display()));
        println!();
        
        // 检测项目类型
        let project_type = IrisConfig::detect_project_type(&project_root);
        println!("{}", "Project Type:".bright_cyan().bold());
        match project_type {
            crate::config::ProjectType::Vue3 => {
                println!("  ✓ Vue 3 project detected");
            }
            crate::config::ProjectType::Unknown => {
                println!("  ⚠ Unknown project type");
            }
        }
        println!();
        
        // 加载配置
        match IrisConfig::load(&project_root) {
            Ok(config) => {
                println!("{}", "Configuration:".bright_cyan().bold());
                println!("  Name:    {}", config.name);
                println!("  Version: {}", config.version);
                println!("  Source:  {}", config.src_dir.display());
                println!("  Output:  {}", config.out_dir.display());
                println!("  Entry:   {}", config.entry);
                println!();
            }
            Err(err) => {
                print_warning(&format!("Could not load configuration: {}", err));
                println!();
            }
        }
        
        // 显示依赖信息
        self.print_dependencies(&project_root)?;
        
        // 显示 Iris 运行时信息
        self.print_runtime_info();
        
        Ok(())
    }
    
    fn print_dependencies(&self, project_root: &std::path::Path) -> Result<()> {
        let package_json_path = project_root.join("package.json");
        
        if !package_json_path.exists() {
            print_warning("No package.json found");
            println!();
            return Ok(());
        }
        
        let content = utils::read_text_file(&package_json_path)?;
        let package_json: serde_json::Value = serde_json::from_str(&content)?;
        
        println!("{}", "Dependencies:".bright_cyan().bold());
        
        // 显示 Vue 版本
        if let Some(version) = self.get_dep_version(&package_json, "vue") {
            println!("  Vue: {}", version);
        }
        
        // 显示 Iris 相关依赖
        if let Some(version) = self.get_dep_version(&package_json, "iris-sfc") {
            println!("  Iris SFC: {}", version);
        }
        
        println!();
        Ok(())
    }
    
    fn get_dep_version<'a>(
        &self,
        package_json: &'a serde_json::Value,
        dep_name: &str,
    ) -> Option<String> {
        // 检查 dependencies
        if let Some(deps) = package_json.get("dependencies") {
            if let Some(version) = deps.get(dep_name) {
                return Some(version.as_str()?.to_string());
            }
        }
        
        // 检查 devDependencies
        if let Some(deps) = package_json.get("devDependencies") {
            if let Some(version) = deps.get(dep_name) {
                return Some(version.as_str()?.to_string());
            }
        }
        
        None
    }
    
    fn print_runtime_info(&self) {
        println!("{}", "Iris Runtime:".bright_cyan().bold());
        println!("  Version:  0.1.0");
        println!("  Backend:  WebGPU");
        println!("  Language: Rust");
        println!("  JS Engine: Boa");
        println!("  Compiler: swc");
        println!();
        
        println!("{}", "Features:".bright_cyan().bold());
        println!("  ✓ Vue SFC compilation");
        println!("  ✓ WebGPU rendering");
        println!("  ✓ CSS layout engine");
        println!("  ✓ JavaScript runtime");
        println!("  ✓ Hot reload");
        println!("  ✓ Developer tools");
        println!();
    }
}
