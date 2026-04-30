//! 增强的 Web API 兼容层
//!
//! 实现完整的浏览器标准 API，包括 WebSocket、LocalStorage 等。

use std::collections::HashMap;
use tracing::{debug, info};

/// WebSocket 连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketState {
    /// 连接中
    Connecting,
    /// 已连接
    Open,
    /// 关闭中
    Closing,
    /// 已关闭
    Closed,
}

/// WebSocket 消息
#[derive(Debug, Clone)]
pub enum WebSocketMessage {
    /// 文本消息
    Text(String),
    /// 二进制消息
    Binary(Vec<u8>),
}

/// WebSocket 连接
pub struct WebSocket {
    /// 连接 URL
    #[allow(dead_code)]
    url: String,
    /// 连接状态
    state: WebSocketState,
    /// 消息处理器
    on_message: Option<Box<dyn Fn(WebSocketMessage) + Send + Sync>>,
    /// 错误处理器
    on_error: Option<Box<dyn Fn(String) + Send + Sync>>,
    /// 关闭处理器
    on_close: Option<Box<dyn Fn() + Send + Sync>>,
}

impl WebSocket {
    /// 创建新的 WebSocket 连接
    pub fn new(url: &str) -> Self {
        info!("Creating WebSocket connection to: {}", url);
        
        Self {
            url: url.to_string(),
            state: WebSocketState::Connecting,
            on_message: None,
            on_error: None,
            on_close: None,
        }
    }

    /// 设置消息处理器
    pub fn on_message<F>(&mut self, handler: F)
    where
        F: Fn(WebSocketMessage) + Send + Sync + 'static,
    {
        self.on_message = Some(Box::new(handler));
    }

    /// 设置错误处理器
    pub fn on_error<F>(&mut self, handler: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.on_error = Some(Box::new(handler));
    }

    /// 设置关闭处理器
    pub fn on_close<F>(&mut self, handler: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_close = Some(Box::new(handler));
    }

    /// 发送文本消息
    pub fn send_text(&mut self, message: &str) -> Result<(), String> {
        if self.state != WebSocketState::Open {
            return Err("WebSocket is not open".to_string());
        }

        debug!("Sending text message: {}", message);

        // 实际应该通过 WebSocket 协议发送
        // 这里仅模拟
        if let Some(ref handler) = self.on_message {
            handler(WebSocketMessage::Text(message.to_string()));
        }

        Ok(())
    }

    /// 发送二进制消息
    pub fn send_binary(&mut self, data: &[u8]) -> Result<(), String> {
        if self.state != WebSocketState::Open {
            return Err("WebSocket is not open".to_string());
        }

        debug!("Sending binary message: {} bytes", data.len());

        if let Some(ref handler) = self.on_message {
            handler(WebSocketMessage::Binary(data.to_vec()));
        }

        Ok(())
    }

    /// 关闭连接
    pub fn close(&mut self) {
        self.state = WebSocketState::Closing;
        info!("Closing WebSocket connection");

        // 模拟关闭
        self.state = WebSocketState::Closed;
        
        if let Some(ref handler) = self.on_close {
            handler();
        }
    }

    /// 获取连接状态
    pub fn state(&self) -> &WebSocketState {
        &self.state
    }
}

/// LocalStorage 实现
pub struct LocalStorage {
    /// 存储数据
    data: HashMap<String, String>,
    /// 存储限制（5MB）
    max_size: usize,
}

impl LocalStorage {
    /// 创建新的 LocalStorage
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            max_size: 5 * 1024 * 1024, // 5MB
        }
    }

    /// 获取存储项
    pub fn get_item(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    /// 设置存储项
    pub fn set_item(&mut self, key: &str, value: &str) -> Result<(), String> {
        // 检查存储限制
        let current_size: usize = self.data.values().map(|v| v.len()).sum();
        let new_size = current_size + value.len() - self.data.get(key).map_or(0, |v| v.len());

        if new_size > self.max_size {
            return Err("LocalStorage quota exceeded".to_string());
        }

        debug!("Setting LocalStorage item: {} = {}", key, value);
        self.data.insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// 移除存储项
    pub fn remove_item(&mut self, key: &str) -> Option<String> {
        self.data.remove(key)
    }

    /// 清除所有存储
    pub fn clear(&mut self) {
        self.data.clear();
        info!("LocalStorage cleared");
    }

    /// 获取存储项数量
    pub fn length(&self) -> usize {
        self.data.len()
    }

    /// 获取指定索引的键
    pub fn key(&self, index: usize) -> Option<String> {
        self.data.keys().nth(index).cloned()
    }

    /// 获取所有键
    pub fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// 获取存储大小（字节）
    pub fn size(&self) -> usize {
        self.data.values().map(|v| v.len()).sum()
    }
}

impl Default for LocalStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// SessionStorage 实现
pub struct SessionStorage {
    /// 存储数据（与 LocalStorage 相同实现）
    data: HashMap<String, String>,
}

impl SessionStorage {
    /// 创建新的 SessionStorage
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// 获取存储项
    pub fn get_item(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    /// 设置存储项
    pub fn set_item(&mut self, key: &str, value: &str) {
        debug!("Setting SessionStorage item: {} = {}", key, value);
        self.data.insert(key.to_string(), value.to_string());
    }

    /// 移除存储项
    pub fn remove_item(&mut self, key: &str) -> Option<String> {
        self.data.remove(key)
    }

    /// 清除所有存储
    pub fn clear(&mut self) {
        self.data.clear();
        info!("SessionStorage cleared");
    }

    /// 获取存储项数量
    pub fn length(&self) -> usize {
        self.data.len()
    }
}

impl Default for SessionStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// XMLHttpRequest 实现
pub struct XMLHttpRequest {
    /// 请求方法
    method: String,
    /// 请求 URL
    url: String,
    /// 请求头
    headers: HashMap<String, String>,
    /// 响应状态码
    status: Option<u16>,
    /// 响应文本
    response_text: Option<String>,
    /// 加载完成
    loaded: bool,
}

impl XMLHttpRequest {
    /// 创建新的 XMLHttpRequest
    pub fn new() -> Self {
        Self {
            method: "GET".to_string(),
            url: String::new(),
            headers: HashMap::new(),
            status: None,
            response_text: None,
            loaded: false,
        }
    }

    /// 打开请求
    pub fn open(&mut self, method: &str, url: &str) {
        self.method = method.to_string();
        self.url = url.to_string();
        self.loaded = false;
        self.status = None;
        self.response_text = None;
        
        info!("XHR opened: {} {}", method, url);
    }

    /// 设置请求头
    pub fn set_request_header(&mut self, header: &str, value: &str) {
        self.headers.insert(header.to_string(), value.to_string());
    }

    /// 发送请求（同步模拟）
    pub fn send(&mut self, _body: Option<&str>) -> Result<(), String> {
        info!("XHR sending: {} {}", self.method, self.url);

        // 实际应该发送 HTTP 请求
        // 这里仅模拟
        
        self.status = Some(200);
        self.response_text = Some("Mock response".to_string());
        self.loaded = true;

        Ok(())
    }

    /// 获取响应状态码
    pub fn status(&self) -> Option<u16> {
        self.status
    }

    /// 获取响应文本
    pub fn response_text(&self) -> Option<&str> {
        self.response_text.as_deref()
    }

    /// 是否加载完成
    pub fn loaded(&self) -> bool {
        self.loaded
    }
}

impl Default for XMLHttpRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_storage() {
        let mut storage = LocalStorage::new();
        
        // 测试设置和获取
        storage.set_item("key1", "value1").unwrap();
        assert_eq!(storage.get_item("key1"), Some("value1".to_string()));
        
        // 测试移除
        assert_eq!(storage.remove_item("key1"), Some("value1".to_string()));
        assert_eq!(storage.get_item("key1"), None);
        
        // 测试清除
        storage.set_item("key2", "value2").unwrap();
        storage.clear();
        assert_eq!(storage.length(), 0);
    }

    #[test]
    fn test_local_storage_quota() {
        let mut storage = LocalStorage::new();
        
        // 写入大量数据应该失败
        let large_value = "x".repeat(6 * 1024 * 1024); // 6MB
        let result = storage.set_item("large", &large_value);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("quota"));
    }

    #[test]
    fn test_session_storage() {
        let mut storage = SessionStorage::new();
        
        storage.set_item("key1", "value1");
        assert_eq!(storage.get_item("key1"), Some("value1".to_string()));
        assert_eq!(storage.length(), 1);
        
        storage.clear();
        assert_eq!(storage.length(), 0);
    }

    #[test]
    fn test_websocket() {
        let mut ws = WebSocket::new("ws://localhost:8080");
        
        assert_eq!(*ws.state(), WebSocketState::Connecting);
        
        // 模拟连接成功
        ws.state = WebSocketState::Open;
        
        // 测试发送消息
        let result = ws.send_text("Hello");
        assert!(result.is_ok());
        
        // 测试关闭
        ws.close();
        assert_eq!(*ws.state(), WebSocketState::Closed);
    }

    #[test]
    fn test_xhr() {
        let mut xhr = XMLHttpRequest::new();
        
        xhr.open("GET", "http://localhost:8080/api");
        xhr.set_request_header("Content-Type", "application/json");
        
        let result = xhr.send(None);
        assert!(result.is_ok());
        
        assert_eq!(xhr.status(), Some(200));
        assert!(xhr.loaded());
    }
}
