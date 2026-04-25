//! Iris JS —— JS 沙箱运行时层
//!
//! 独立隔离执行环境，基于 QuickJS（轻量高性能、Wasm 友好、Rust 深度绑定）。
//! 内置预加载 Vue3 完整运行时（runtime-core / runtime-dom）。
//! 自研 ESM 解析器，支持 import/export 与第三方包引入。

#![warn(missing_docs)]

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

/// QuickJS 虚拟机封装。
pub mod vm {
    /// JS 运行时实例。
    pub struct JsRuntime {
        // TODO: fields
    }

    impl JsRuntime {
        /// 创建新的 JS 运行时。
        pub fn new() -> Self {
            Self {}
        }

        /// 执行 JS 脚本字符串。
        pub fn eval(&mut self, _script: &str) {
            // TODO: implement
        }
    }
}

/// ESM 模块系统。
pub mod module {
    /// 解析并加载 ES Module。
    pub fn load_module(_specifier: &str) {
        // TODO: implement
    }
}

/// Vue3 运行时预加载。
pub mod vue {
    /// 注入 Vue3 runtime-core 与 runtime-dom 到 JS 全局环境。
    pub fn inject_vue_runtime() {
        // TODO: implement
    }
}
