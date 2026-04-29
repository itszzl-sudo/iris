//! 工具函数

use anyhow::{Result, anyhow};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::fs;

/// 查找项目根目录（包含 package.json 的目录）
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let mut current = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()?.join(start)
    };

    loop {
        if current.join("package.json").exists() {
            return Ok(current);
        }

        if !current.pop() {
            return Err(anyhow!(
                "Could not find package.json in current directory or parent directories"
            ));
        }
    }
}

/// 检查是否为 Vue 项目
pub fn is_vue_project(project_root: &Path) -> bool {
    // 检查 package.json 中是否有 vue 依赖
    if let Ok(content) = fs::read_to_string(project_root.join("package.json")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let has_vue = json.get("dependencies")
                .and_then(|d| d.get("vue"))
                .is_some()
                || json.get("devDependencies")
                    .and_then(|d| d.get("vue"))
                    .is_some();
            
            if has_vue {
                return true;
            }
        }
    }

    // 检查是否有 Vue 文件
    let src_dir = project_root.join("src");
    if src_dir.exists() {
        if let Ok(entries) = fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "vue" {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// 查找入口文件
pub fn find_entry_file(project_root: &Path) -> Result<PathBuf> {
    // 优先查找 src/main.js 或 src/main.ts
    for entry in &["src/main.js", "src/main.ts", "src/main.jsx", "src/main.tsx"] {
        let path = project_root.join(entry);
        if path.exists() {
            return Ok(path);
        }
    }

    // 查找 src/App.vue
    let app_vue = project_root.join("src/App.vue");
    if app_vue.exists() {
        return Ok(app_vue);
    }

    // 查找任意 .vue 文件
    let src_dir = project_root.join("src");
    if src_dir.exists() {
        if let Ok(entries) = fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "vue" {
                        return Ok(entry.path());
                    }
                }
            }
        }
    }

    Err(anyhow!("No entry file found (src/main.js, src/main.ts, or src/*.vue)"))
}

/// 统计 Vue 文件数量
pub fn count_vue_files(project_root: &Path) -> Result<usize> {
    let mut count = 0;
    let src_dir = project_root.join("src");
    
    if src_dir.exists() {
        for entry in walkdir::WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Some(ext) = entry.path().extension() {
                if ext == "vue" {
                    count += 1;
                }
            }
        }
    }
    
    Ok(count)
}

/// 打印项目信息
pub fn print_project_info(root: &str) -> Result<()> {
    println!("{}", "ℹ️  Iris JetCrab - Project Info".bright_cyan().bold());
    println!();

    let project_root = find_project_root(Path::new(root))?;
    
    if !is_vue_project(&project_root) {
        println!("{}", "❌ Error: Not a Vue project".bright_red().bold());
        return Ok(());
    }

    println!("{} {}", "📁 Project:".bright_blue(), project_root.display().to_string().bright_white());
    
    if let Ok(entry) = find_entry_file(&project_root) {
        let relative = entry.strip_prefix(&project_root)?;
        println!("{} {}", "📄 Entry:".bright_blue(), relative.display().to_string().bright_white());
    }
    
    let vue_count = count_vue_files(&project_root)?;
    println!("{} {}", "📦 Vue Files:".bright_blue(), vue_count.to_string().bright_white());

    Ok(())
}
