//! Web API 兼容层
//!
//! 实现浏览器 Web API 的 JetCrab 版本。

use tracing::debug;

/// Console API
pub struct Console;

impl Console {
    /// console.log()
    pub fn log(args: &[String]) {
        println!("[LOG] {}", args.join(" "));
    }

    /// console.error()
    pub fn error(args: &[String]) {
        eprintln!("[ERROR] {}", args.join(" "));
    }

    /// console.warn()
    pub fn warn(args: &[String]) {
        eprintln!("[WARN] {}", args.join(" "));
    }

    /// console.info()
    pub fn info(args: &[String]) {
        println!("[INFO] {}", args.join(" "));
    }
}

/// Process API
pub struct Process;

impl Process {
    /// 获取当前工作目录
    pub fn cwd() -> Result<String, String> {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| e.to_string())
    }

    /// 获取环境变量
    pub fn env(key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    /// 设置环境变量
    pub fn set_env(key: &str, value: &str) {
        std::env::set_var(key, value);
        debug!("Set env: {}={}", key, value);
    }

    /// 获取命令行参数
    pub fn argv() -> Vec<String> {
        std::env::args().collect()
    }

    /// 获取进程 PID
    pub fn pid() -> u32 {
        std::process::id()
    }

    /// 退出进程
    pub fn exit(code: i32) -> ! {
        std::process::exit(code)
    }
}

/// Fetch API（简化版本）
pub struct FetchResponse {
    /// HTTP 状态码
    pub status: u16,
    /// 响应体
    pub body: String,
    /// 响应头
    pub headers: std::collections::HashMap<String, String>,
}

impl FetchResponse {
    /// 解析 JSON 响应
    pub fn json(&self) -> Result<serde_json::Value, String> {
        serde_json::from_str(&self.body)
            .map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    /// 获取文本响应
    pub fn text(&self) -> &str {
        &self.body
    }
}

/// Fetch API
pub async fn fetch(url: &str) -> Result<FetchResponse, String> {
    debug!("Fetching: {}", url);

    // 使用 reqwest 进行 HTTP 请求
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Fetch failed: {}", e))?;

    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(FetchResponse {
        status,
        body,
        headers,
    })
}

/// Timer API
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{sleep, Duration};

static TIMER_ID: AtomicU32 = AtomicU32::new(1);

/// setTimeout
pub fn set_timeout<F>(callback: F, delay_ms: u64) -> u32
where
    F: FnOnce() + Send + 'static,
{
    let id = TIMER_ID.fetch_add(1, Ordering::SeqCst);

    tokio::spawn(async move {
        sleep(Duration::from_millis(delay_ms)).await;
        callback();
    });

    id
}

/// setInterval
pub fn set_interval<F>(callback: F, interval_ms: u64) -> u32
where
    F: Fn() + Send + 'static,
{
    let id = TIMER_ID.fetch_add(1, Ordering::SeqCst);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
        loop {
            interval.tick().await;
            callback();
        }
    });

    id
}

/// clearTimeout / clearInterval
pub fn clear_timer(_timer_id: u32) {
    // TODO: 实现定时器清理
    debug!("Timer {} cleared", _timer_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_log() {
        // 只是验证不会 panic
        Console::log(&["test".to_string()]);
    }

    #[test]
    fn test_process_cwd() {
        let cwd = Process::cwd();
        assert!(cwd.is_ok());
    }

    #[test]
    fn test_process_env() {
        Process::set_env("TEST_VAR", "test_value");
        let value = Process::env("TEST_VAR");
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_process_argv() {
        let argv = Process::argv();
        assert!(!argv.is_empty());
    }

    #[test]
    fn test_process_pid() {
        let pid = Process::pid();
        assert!(pid > 0);
    }
}
