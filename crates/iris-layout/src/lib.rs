//! Iris Layout —— 浏览器级布局 & 样式引擎
//!
//! 复刻标准浏览器 CSS 体系，对标 Chromium 基础能力。
//! 包含 HTML 解析、CSS 解析、选择器匹配、样式继承、Flex/流式布局计算。

#![warn(missing_docs)]

use iris_core;

/// 初始化布局引擎。
pub fn init() {
    iris_core::init();
    println!("iris-layout initialized");
}

/// HTML 解析与 DOM 树构建。
pub mod html {
    /// 解析 HTML 字符串，生成标准 DOM 节点树。
    pub fn parse(html: &str) {
        let _ = html;
        // TODO: use html5ever
    }
}

/// CSS 解析与样式计算。
pub mod css {
    /// 解析 CSS 样式表。
    pub fn parse_stylesheet(css: &str) {
        let _ = css;
        // TODO: use cssparser
    }
}

/// 布局计算系统。
pub mod layout {
    /// 执行盒模型与 Flex 布局计算。
    pub fn compute_layout() {
        // TODO: implement
    }
}
