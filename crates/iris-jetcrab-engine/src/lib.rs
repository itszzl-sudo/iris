//! Iris JetCrab Engine —— Vue 项目运行时编排
//!
//! 使用 JetCrab 作为 JavaScript 执行引擎，提供完整的 Vue 项目加载、编译和渲染能力。
//!
//! # 架构设计
//!
//! ```text
//! Vue 项目目录
//!     ↓
//! iris-jetcrab-engine（编排层）
//!     ├─ 读取 index.html
//!     ├─ 解析入口文件
//!     ↓
//! iris-sfc（编译 Vue SFC）
//!     ↓
//! iris-jetcrab（执行 JavaScript）
//!     ↓
//! iris-dom（DOM API）
//!     ↓
//! iris-layout（布局计算）
//!     ↓
//! iris-gpu（WebGPU 渲染）
//! ```
//!
//! # 示例
//!
//! ```rust,ignore
//! use iris_jetcrab_engine::JetCrabEngine;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 创建引擎实例
//!     let mut engine = JetCrabEngine::new();
//!     
//!     // 初始化
//!     engine.initialize().await?;
//!     
//!     // 加载 Vue 项目
//!     engine.load_project("/path/to/vue-project").await?;
//!     
//!     // 运行
//!     engine.run().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # 特性
//!
//! - ✅ Vue 项目目录扫描和解析
//! - ✅ index.html 入口检测
//! - ✅ SFC 自动编译
//! - ✅ JetCrab JS 运行时集成
//! - ✅ 模块依赖解析
//! - ✅ 热更新支持（HMR）
//! - ✅ WebGPU 硬件渲染
//! - ✅ 文件监听

#![warn(missing_docs)]

pub mod engine;
pub mod project_scanner;
pub mod module_graph;
pub mod hmr;
pub mod sfc_compiler;

// 重新导出常用类型
pub use engine::JetCrabEngine;
pub use engine::EngineConfig;
pub use project_scanner::ProjectScanner;
pub use project_scanner::ProjectInfo;
pub use module_graph::ModuleGraph;
pub use hmr::HMRManager;
pub use sfc_compiler::{CompiledModule, StyleBlock, compile_sfc, resolve_module};

/// Iris JetCrab Engine 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化 JetCrab Engine
pub fn init() {
    tracing::info!("Initializing Iris JetCrab Engine v{}", VERSION);
    
    // 初始化共享核心层
    iris_core::init();
    iris_gpu::init();
    iris_layout::init();
    iris_dom::init();
    iris_sfc::init();
    
    // 初始化 JetCrab 运行时
    iris_jetcrab::init();
    
    tracing::info!("JetCrab Engine initialized successfully");
}
