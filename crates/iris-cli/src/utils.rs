//! 通用工具函数

use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use colored::Colorize;

/// 打印成功消息
pub fn print_success(msg: &str) {
    println!("{} {}", "✓".green(), msg);
}

/// 打印警告消息
pub fn print_warning(msg: &str) {
    println!("{} {}", "⚠".yellow(), msg);
}

/// 打印错误消息
#[allow(dead_code)]
pub fn print_error(msg: &str) {
    eprintln!("{} {}", "✗".red(), msg);
}

/// 打印信息消息
pub fn print_info(msg: &str) {
    println!("{} {}", "ℹ".blue(), msg);
}

/// 格式化文件大小
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// 获取项目根目录
pub fn find_project_root(start_dir: &Path) -> Result<PathBuf> {
    let mut current = start_dir.to_path_buf();
    
    loop {
        // 检查是否存在 iris.config.json 或 package.json
        if current.join("iris.config.json").exists() 
            || current.join("package.json").exists()
            || current.join("Cargo.toml").exists() {
            return Ok(current);
        }
        
        // 向上一级
        if !current.pop() {
            return Err(anyhow::anyhow!("Could not find project root"));
        }
    }
}

/// 确保目录存在
pub fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))
}

/// 复制文件
pub fn copy_file(src: &Path, dest: &Path) -> Result<u64> {
    std::fs::copy(src, dest)
        .with_context(|| format!("Failed to copy {} to {}", src.display(), dest.display()))
}

/// 递归复制目录
pub fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    ensure_dir(dest)?;
    
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir(&src_path, &dest_path)?;
        } else {
            copy_file(&src_path, &dest_path)?;
        }
    }
    
    Ok(())
}

/// 删除目录
pub fn remove_dir(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove directory: {}", path.display()))?;
    }
    Ok(())
}

/// 读取文本文件
pub fn read_text_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))
}

/// 写入文本文件
pub fn write_text_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write file: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    }
    
    #[test]
    fn test_ensure_dir() {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("test/nested/dir");
        
        ensure_dir(&new_dir).unwrap();
        assert!(new_dir.exists());
    }
    
    #[test]
    fn test_write_and_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, World!";
        
        write_text_file(&file_path, content).unwrap();
        let read_content = read_text_file(&file_path).unwrap();
        
        assert_eq!(read_content, content);
    }
}
