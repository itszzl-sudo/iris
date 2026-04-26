//! Iris Engine —— Rust+WebGPU 下一代无构建前端运行时
//!
//! 这是 Iris 的元 crate（meta crate），重新导出所有子模块的公共 API，
//! 并提供完整的运行时编排能力。
//!
//! # 架构概览
//! - [`core`](iris_core) —— 底层内核底座（窗口、异步、IO）
//! - [`gpu`](iris_gpu) —— WebGPU 硬件渲染管线
//! - [`layout`](iris_layout) —— 浏览器级布局 & CSS 引擎
//! - [`dom`](iris_dom) —— 跨端 DOM/BOM 抽象与事件系统
//! - [`js`](iris_js) —— JS 沙箱运行时（Boa Engine + Vue3 runtime）
//! - [`sfc`](iris_sfc) —— SFC/TS 即时转译层
//! - [`orchestrator`] —— 运行时编排器（Phase 4）
//!
//! # 快速开始
//!
//! ## 方式 1: 使用 Application trait
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
//!
//! ## 方式 2: 使用 RuntimeOrchestrator
//! ```ignore
//! use iris::orchestrator::RuntimeOrchestrator;
//!
//! let mut runtime = RuntimeOrchestrator::new();
//! runtime.initialize()?;
//! runtime.load_vue_app("examples/App.vue")?;
//! runtime.run()?;
//! ```

#![warn(missing_docs)]

pub use iris_core as core;
pub use iris_dom as dom;
pub use iris_gpu as gpu;
pub use iris_js as js;
pub use iris_layout as layout;
pub use iris_sfc as sfc;

/// 运行时编排器（Phase 4）
///
/// 负责将 iris-sfc、iris-js、iris-dom、iris-layout 和 iris-gpu 连接在一起，
/// 形成完整的 Vue 3 运行时。
pub mod orchestrator;

/// VNode 到 GPU 渲染适配器（Phase 5）
///
/// 将虚拟 DOM 树转换为 GPU 绘制命令，实现高效的渲染管线。
pub mod vnode_renderer;

/// 脏矩形管理器（性能优化）
///
/// 用于优化渲染性能，只重绘发生变化的区域。
pub mod dirty_rect_manager;
pub use dirty_rect_manager::{DirtyRect, DirtyRectManager, DirtyRectStats};

/// 动画引擎（CSS Transitions & Animations）
///
/// 实现 CSS 过渡动画和关键帧动画，支持缓动函数和时间轴控制。
pub mod animation_engine;
pub use animation_engine::{
    EasingFunction, TransitionConfig, ElementAnimationState, TransitionAnimation, AnimatedValue,
};

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
