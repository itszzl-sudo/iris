//! Iris DOM —— 跨端统一抽象层
//!
//! 抹平浏览器与桌面原生环境的差异。
//! 提供统一的事件系统（鼠标、键盘、滚动、点击命中检测）
//! 与轻量 BOM/DOM 模拟 API（window/document/Event）。
//! 无真实 DOM，仅做逻辑模拟，实际绘制全部走 WebGPU。
//!
//! # 架构设计
//!
//! ```text
//! VNode (虚拟 DOM) ←→ EventDispatcher (事件系统) ←→ BOM API (Window/Document)
//! ```
//!
//! # 示例
//!
//! ```rust
//! use iris_dom::vnode::VNode;
//! use iris_dom::bom::{Window, Document};
//! use iris_dom::event::{EventDispatcher, EventType, Event};
//!
//! // 创建虚拟 DOM
//! let mut div = VNode::element("div");
//! div.set_attr("class", "container");
//!
//! // 创建 BOM 环境
//! let window = Window::new(800, 600);
//! let document = Document::new();
//!
//! // 创建事件分发器
//! let mut dispatcher = EventDispatcher::new();
//! ```

#![warn(missing_docs)]

pub mod vnode;
pub mod event;
pub mod bom;

use iris_core;
use iris_layout;

/// 初始化 DOM 抽象层。
pub fn init() {
    iris_core::init();
    iris_layout::init();
    println!("iris-dom initialized");
}
