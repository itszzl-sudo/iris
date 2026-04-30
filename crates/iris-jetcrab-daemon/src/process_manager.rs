//! 进程管理器 - 启动/监控/重启 iris-jetcrab dev 子进程

use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 子进程状态
#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Stopped,
    Running,
    Failed(String),
}

/// 进程管理器
pub struct ProcessManager {
    /// 子进程句柄
    child: Option<Child>,
    /// 项目根目录
    project_root: String,
    /// HTTP 端口
    http_port: u16,
    /// 运行状态
    pub status: Arc<AtomicBool>,
}

impl ProcessManager {
    pub fn new(project_root: &str, http_port: u16) -> Self {
        Self {
            child: None,
            project_root: project_root.to_string(),
            http_port,
            status: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 启动 dev server 子进程
    pub fn start(&mut self) -> Result<(), String> {
        if self.child.is_some() {
            return Err("Server already running".to_string());
        }

        let binary_name = if cfg!(windows) { "iris-jetcrab.exe" } else { "iris-jetcrab" };

        // 查找二进制文件位置
        let binary_path = find_binary(binary_name).ok_or_else(|| {
            format!("Cannot find {} binary. Build with: cargo build -p iris-jetcrab-cli", binary_name)
        })?;

        let child = Command::new(&binary_path)
            .args(&[
                "dev",
                "--root",
                &self.project_root,
                "--port",
                &self.http_port.to_string(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start server: {}", e))?;

        tracing::info!("Dev server started (PID: {})", child.id());
        self.child = Some(child);
        self.status.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// 停止 dev server
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(windows)]
            {
                let _ = Command::new("taskkill")
                    .args(&["/f", "/pid", &child.id().to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            }
            #[cfg(not(windows))]
            {
                let _ = child.kill();
            }
            let _ = child.wait();
            self.status.store(false, Ordering::SeqCst);
            tracing::info!("Dev server stopped");
        }
    }

    /// 检查服务器是否在运行（通过 HTTP 健康检查）
    pub async fn check_health(&self) -> bool {
        let url = format!("http://127.0.0.1:{}/api/project-info", self.http_port);
        match reqwest::get(&url).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// 重启服务器
    pub fn restart(&mut self) -> Result<(), String> {
        self.stop();
        // 等端口释放
        std::thread::sleep(Duration::from_millis(500));
        self.start()
    }

    /// 切换项目
    pub fn switch_project(&mut self, new_root: &str) -> Result<(), String> {
        self.project_root = new_root.to_string();
        self.restart()
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 在工作目录和 PATH 中查找二进制文件
fn find_binary(name: &str) -> Option<std::path::PathBuf> {
    // 先检查 cargo 编译输出目录
    let target_paths = vec![
        std::path::PathBuf::from("target/debug").join(name),
        std::path::PathBuf::from("target/release").join(name),
    ];

    for path in &target_paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    // 在 PATH 中查找
    which::which(name).ok()
}
