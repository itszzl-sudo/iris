//! Iris Engine —— Rust+WebGPU 下一代无构建前端运行时
//!
//! 这是 Iris 的元 crate（meta crate），重新导出所有子模块的公共 API。
//! 开发者通常只需依赖此 crate 即可使用完整的 Iris 引擎能力。
//!
//! # 架构概览
//! - [`core`](iris_core) —— 底层内核底座（窗口、异步、IO）
//! - [`gpu`](iris_gpu) —— WebGPU 硬件渲染管线
//! - [`layout`](iris_layout) —— 浏览器级布局 & CSS 引擎
//! - [`dom`](iris_dom) —— 跨端 DOM/BOM 抽象与事件系统
//! - [`js`](iris_js) —— JS 沙箱运行时（QuickJS + Vue3 runtime）
//! - [`sfc`](iris_sfc) —— SFC/TS 即时转译层
//!
//! # 快速开始
//! ```ignore
//! use iris::core::{run_app, Application, Context};
//!
//! struct MyApp;
//! impl Application for MyApp {
//!     fn initialize(&mut self, _ctx: &Context, _event_loop: &ActiveEventLoop) {}
//!     fn window_event(&mut self, _ctx: &Context, _el: &ActiveEventLoop, _id: WindowId, _e: WindowEvent) {}
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     run_app(MyApp)?;
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]

pub use iris_core as core;
pub use iris_dom as dom;
pub use iris_gpu as gpu;
pub use iris_js as js;
pub use iris_layout as layout;
pub use iris_sfc as sfc;

/// Iris 引擎版本号。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化整个 Iris 引擎各层。
///
/// 按架构层级自下而上依次初始化：
/// core → gpu → layout → dom → js → sfc
pub fn init() {
    iris_core::init();
    iris_gpu::init();
    iris_layout::init();
    iris_dom::init();
    iris_js::init();
    iris_sfc::init();
}
