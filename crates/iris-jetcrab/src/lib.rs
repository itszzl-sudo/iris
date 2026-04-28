//! Iris JetCrab Runtime Integration
//!
//! 使用 JetCrab 作为 JavaScript 执行引擎，提供完整的 npm 包支持和 WASM 集成。
//!
//! # 架构设计
//!
//! ```text
//! Vue SFC → iris-sfc → JavaScript Code
//!                      ↓
//!              iris-jetcrab (JetCrab Runtime)
//!                      ↓
//!              Web APIs + DOM APIs
//!                      ↓
//!              iris-dom → iris-layout → iris-gpu
//! ```
//!
//! # 示例
//!
//! ```rust,ignore
//! use iris_jetcrab::JetCrabRuntime;
//!
//! let mut runtime = JetCrabRuntime::new();
//! runtime.init().unwrap();
//!
//! // 执行 JavaScript
//! let result = runtime.eval("console.log('Hello from JetCrab!')");
//! ```
//!
//! # 特性
//!
//! - ✅ JetCrab Chitin 引擎集成
//! - ✅ CPM 包管理支持
//! - ✅ ESM 模块系统
//! - ✅ Web API 兼容层
//! - ✅ WASM 原生支持
//! - ✅ 异步 I/O (Tokio)

#![warn(missing_docs)]

pub mod runtime;
pub mod module;
pub mod web_apis;
pub mod web_apis_enhanced;  // 增强的 Web API
pub mod bridge;
pub mod esm;      // 增强版 ESM 模块加载器
pub mod cpm;      // CPM 包管理集成
pub mod wasm_bridge;  // WASM 桥接

// 重新导出常用类型
pub use runtime::JetCrabRuntime;
pub use module::ModuleLoader;
pub use module::ModuleInfo;
pub use bridge::JetCrabBridge;
pub use esm::ESMModuleLoader;
pub use esm::ESMModuleInfo;
pub use cpm::CPMManager;
pub use cpm::PackageInfo;

// 增强的 Web API
pub use web_apis_enhanced::WebSocket;
pub use web_apis_enhanced::WebSocketState;
pub use web_apis_enhanced::WebSocketMessage;
pub use web_apis_enhanced::LocalStorage;
pub use web_apis_enhanced::SessionStorage;
pub use web_apis_enhanced::XMLHttpRequest;

// WASM 桥接
pub use wasm_bridge::WasmLoader;
pub use wasm_bridge::WasmModuleInfo;
pub use wasm_bridge::WasmInstance;
pub use wasm_bridge::JsFFIBridge;

/// Iris JetCrab 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化 JetCrab 运行时
pub fn init() {
    tracing::info!("Initializing Iris JetCrab Runtime v{}", VERSION);
    // JetCrab 运行时初始化逻辑
    tracing::info!("JetCrab Runtime initialized successfully");
}
