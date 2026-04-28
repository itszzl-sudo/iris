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
pub mod bridge;

// 重新导出常用类型
pub use runtime::JetCrabRuntime;
pub use module::ModuleLoader;
pub use bridge::JetCrabBridge;
