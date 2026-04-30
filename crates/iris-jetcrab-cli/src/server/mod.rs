//! HTTP 服务器模块
//! 
//! 负责：
//! 1. 启动 Web 服务器
//! 2. 处理 HTTP 请求
//! 3. 调用 iris-jetcrab-engine 编译模块
//! 4. WebSocket HMR

mod http_server;
mod routes;
mod hmr;
mod compiler_cache;
pub mod ai_inspector;

pub use http_server::start;
