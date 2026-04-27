//! 构建命令

use clap::Args;
use anyhow::Result;
use colored::Colorize;
use std::time::Instant;
use walkdir::WalkDir;
use crate::config::IrisConfig;
use crate::utils::{self, print_success, print_info, print_warning};

/// 构建命令参数
#[derive(Args)]
pub struct BuildCommand {
    /// 项目根目录
    #[arg(short, long, default_value = ".")]
    pub root: String,
    
    /// 输出目录
    #[arg(short, long)]
    pub out_dir: Option<String>,
    
    /// 禁用压缩
    #[arg(long)]
    pub no_minify: bool,
    
    /// 生成 sourcemap
    #[arg(long)]
    pub sourcemap: bool,
    
    /// 分析构建产物
    #[arg(long)]
    pub analyze: bool,
}

impl BuildCommand {
    pub fn execute(&self) -> Result<()> {
        println!("{}", "Building for production...".bright_cyan().bold());
        println!();
        
        let start_time = Instant::now();
        
        // 找到项目根目录
        let project_root = utils::find_project_root(std::path::Path::new(&self.root))?;
        print_success(&format!("Project root: {}", project_root.display()));
        
        // 加载配置
        let mut config = IrisConfig::load(&project_root)?;
        
        // 覆盖配置
        if let Some(out_dir) = &self.out_dir {
            config.out_dir = out_dir.into();
        }
        if self.no_minify {
            config.build.minify = false;
        }
        if self.sourcemap {
            config.build.sourcemap = true;
        }
        
        // 显示配置
        self.print_config(&config);
        
        // 清理输出目录
        print_info("Cleaning output directory...");
        let out_path = project_root.join(&config.out_dir);
        utils::remove_dir(&out_path)?;
        utils::ensure_dir(&out_path)?;
        
        // 编译 Vue SFC 文件
        print_info("Compiling Vue SFC files...");
        let sfc_count = self.compile_sfc_files(&project_root, &config)?;
        print_success(&format!("Compiled {} SFC files", sfc_count));
        
        // 复制静态资源
        print_info("Copying static assets...");
        self.copy_assets(&project_root, &config)?;
        
        // 生成构建产物
        print_info("Generating build artifacts...");
        self.generate_artifacts(&project_root, &config)?;
        
        let duration = start_time.elapsed();
        println!();
        print_success(&format!("Build completed in {:.2}s", duration.as_secs_f64()));
        
        // 显示构建产物信息
        if self.analyze {
            println!();
            self.analyze_build(&out_path)?;
        }
        
        println!();
        println!("{}", "Build artifacts:".bright_cyan().bold());
        self.print_build_info(&out_path)?;
        println!();
        
        Ok(())
    }
    
    fn print_config(&self, config: &IrisConfig) {
        println!("{}", "Build Configuration:".bright_cyan().bold());
        println!("  Project:   {}", config.name);
        println!("  Version:   {}", config.version);
        println!("  Output:    {}", config.out_dir.display());
        println!(
            "  Minify:    {}",
            if config.build.minify { "Yes" } else { "No" }
        );
        println!(
            "  Sourcemap: {}",
            if config.build.sourcemap { "Yes" } else { "No" }
        );
        println!("  Target:    {}", config.build.target);
        println!();
    }
    
    fn compile_sfc_files(&self, project_root: &std::path::Path, config: &IrisConfig) -> Result<usize> {
        let src_dir = project_root.join(&config.src_dir);
        
        if !src_dir.exists() {
            print_warning(&format!("Source directory not found: {}", src_dir.display()));
            return Ok(0);
        }
        
        let mut count = 0;
        for entry in WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok()) {
            if entry.path().extension().map_or(false, |ext| ext == "vue") {
                // 在实际实现中，这里会调用 iris-sfc 编译器
                count += 1;
            }
        }
        
        Ok(count)
    }
    
    fn copy_assets(&self, project_root: &std::path::Path, config: &IrisConfig) -> Result<()> {
        let src_dir = project_root.join(&config.src_dir);
        let out_dir = project_root.join(&config.out_dir);
        
        // 复制 public 目录
        let public_dir = project_root.join("public");
        if public_dir.exists() {
            utils::copy_dir(&public_dir, &out_dir)?;
        }
        
        Ok(())
    }
    
    fn generate_artifacts(&self, project_root: &std::path::Path, config: &IrisConfig) -> Result<()> {
        let out_dir = project_root.join(&config.out_dir);
        
        // 生成 index.html
        let index_html = self.generate_index_html(config);
        utils::write_text_file(&out_dir.join("index.html"), &index_html)?;
        
        // 生成 manifest.json（如果是 Web 目标）
        if config.build.target == "web" {
            let manifest = self.generate_manifest(config);
            utils::write_text_file(&out_dir.join("manifest.json"), &manifest)?;
        }
        
        Ok(())
    }
    
    fn generate_index_html(&self, config: &IrisConfig) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
</head>
<body>
    <div id="app"></div>
    <script type="module" src="/main.js"></script>
</body>
</html>"#,
            config.name
        )
    }
    
    fn generate_manifest(&self, config: &IrisConfig) -> String {
        serde_json::json!({
            "name": config.name,
            "version": config.version,
            "description": format!("{} built with Iris Runtime", config.name),
            "start_url": "/index.html",
            "display": "standalone",
            "background_color": "#ffffff",
            "theme_color": "#000000"
        })
        .to_string()
    }
    
    fn analyze_build(&self, out_path: &std::path::Path) -> Result<()> {
        println!("{}", "Build Analysis:".bright_cyan().bold());
        
        let mut total_size = 0u64;
        let mut file_count = 0u64;
        
        for entry in WalkDir::new(out_path)
            .into_iter()
            .filter_map(|e| e.ok()) {
            if entry.path().is_file() {
                if let Ok(metadata) = entry.path().metadata() {
                    total_size += metadata.len();
                    file_count += 1;
                }
            }
        }
        
        println!("  Total files: {}", file_count);
        println!("  Total size:  {}", utils::format_bytes(total_size));
        println!();
        
        Ok(())
    }
    
    fn print_build_info(&self, out_path: &std::path::Path) -> Result<()> {
        if !out_path.exists() {
            print_warning("Output directory is empty");
            return Ok(());
        }
        
        for entry in std::fs::read_dir(out_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Ok(metadata) = path.metadata() {
                    println!(
                        "  {} ({})",
                        path.file_name().unwrap().to_string_lossy(),
                        utils::format_bytes(metadata.len())
                    );
                }
            }
        }
        
        Ok(())
    }
}
