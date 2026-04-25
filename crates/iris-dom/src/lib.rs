//! Iris DOM —— 跨端统一抽象层
//!
//! 抹平浏览器与桌面原生环境的差异。
//! 提供统一的事件系统（鼠标、键盘、滚动、点击命中检测）
//! 与轻量 BOM/DOM 模拟 API（window/document/Event）。
//! 无真实 DOM，仅做逻辑模拟，实际绘制全部走 WebGPU。

#![warn(missing_docs)]

use iris_core;
use iris_layout;

/// 初始化 DOM 抽象层。
pub fn init() {
    iris_core::init();
    iris_layout::init();
    println!("iris-dom initialized");
}

/// 轻量 DOM 节点模拟。
pub mod node {
    /// 虚拟 DOM 节点。
    pub struct VNode {
        // TODO: fields
    }
}

/// 统一事件系统。
pub mod event {
    /// 事件类型枚举。
    pub enum EventType {
        /// 鼠标事件
        Mouse,
        /// 键盘事件
        Keyboard,
        /// 滚轮/触摸滚动
        Scroll,
        /// 点击命中
        HitTest,
    }

    /// 分发事件到目标节点。
    pub fn dispatch() {
        // TODO: implement
    }
}

/// BOM API 模拟（window / document / location）。
pub mod bom {
    /// 模拟 `window` 全局对象。
    pub struct Window {
        // TODO: fields
    }
}
