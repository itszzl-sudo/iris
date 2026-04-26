//! Iris JS —— JS 沙箱运行时层
//!
//! 独立隔离执行环境，基于 QuickJS（轻量高性能、Wasm 友好、Rust 深度绑定）。
//! 内置预加载 Vue3 完整运行时（runtime-core / runtime-dom）。
//! 自研 ESM 解析器，支持 import/export 与第三方包引入。
//!
//! # 架构设计
//!
//! ```text
//! QuickJS Runtime → Context → (BOM API + Vue Runtime + ESM Modules)
//! ```
//!
//! # 示例
//!
//! ```rust
//! use iris_js::vm::JsRuntime;
//! use iris_js::vue::inject_vue_runtime;
//! use iris_js::module::ModuleRegistry;
//!
//! let mut runtime = JsRuntime::new();
//! inject_vue_runtime(&mut runtime).unwrap();
//!
//! let result = runtime.eval("Vue.version");
//! ```

#![warn(missing_docs)]

pub mod vm;
pub mod module;
pub mod vue;

use iris_core;
use iris_dom;

/// 初始化 JS 运行时。
///
/// 创建 QuickJS 虚拟机实例，注入全局 API 与 Vue3 运行时。
pub fn init() {
    iris_core::init();
    iris_dom::init();
    println!("iris-js initialized");
}
