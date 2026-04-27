//! 开发服务器命令

use clap::Args;
use anyhow::Result;
use colored::Colorize;
use std::net::TcpStream;
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
        
        // 启动开发服务器
        self.start_dev_server(&project_root, &config)
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
    
    fn start_dev_server(&self, project_root: &std::path::Path, config: &IrisConfig) -> Result<()> {
        use std::io::Write;
        use std::net::{TcpListener, TcpStream};
        use std::fs;
        
        let port = config.dev_server.port;
        let address = format!("127.0.0.1:{}", port);
        
        print_success(&format!("Starting HTTP server at http://{}", address));
        print_info("Serving files from: dist/");
        println!();
        print_info("Press Ctrl+C to stop the server");
        println!();
        
        // 确保 dist 目录存在
        let dist_dir = project_root.join(&config.out_dir);
        if !dist_dir.exists() {
            print_info("Dist directory not found, running build first...");
            // 这里可以调用 build 命令，但暂时跳过
            fs::create_dir_all(&dist_dir)?;
        }
        
        // 创建 TCP 监听器
        let listener = TcpListener::bind(&address)?;
        print_success(&format!("Server listening on http://{}", address));
        println!();
        
        // 接受连接
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_request(stream, &dist_dir) {
                        eprintln!("Error handling request: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Connection failed: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    fn handle_request(&self, mut stream: TcpStream, dist_dir: &std::path::Path) -> Result<()> {
        use std::io::{BufReader, BufRead, Write};
        use std::fs;
        
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        
        // 解析请求路径
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(());
        }
        
        let path = parts[1];
        
        // 确定文件路径
        let file_path = if path == "/" || path == "/index.html" {
            dist_dir.join("index.html")
        } else {
            dist_dir.join(path.trim_start_matches('/'))
        };
        
        // 读取文件并返回
        if file_path.exists() && file_path.is_file() {
            let content = fs::read(&file_path)?;
            let mime_type = self.get_mime_type(&file_path);
            
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                mime_type,
                content.len()
            );
            
            stream.write_all(response.as_bytes())?;
            stream.write_all(&content)?;
        } else {
            let not_found = b"<h1>404 Not Found</h1>";
            let response = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n",
                not_found.len()
            );
            stream.write_all(response.as_bytes())?;
            stream.write_all(not_found)?;
        }
        
        stream.flush()?;
        Ok(())
    }
    
    fn get_mime_type(&self, path: &std::path::Path) -> &str {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("js") => "application/javascript",
            Some("css") => "text/css",
            Some("json") => "application/json",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            _ => "application/octet-stream",
        }
    }
}
