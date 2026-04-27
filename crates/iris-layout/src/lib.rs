//! Iris Layout —— 浏览器级布局 & 样式引擎
//!
//! 复刻标准浏览器 CSS 体系，对标 Chromium 基础能力。
//! 包含 HTML 解析、CSS 解析、选择器匹配、样式继承、Flex/流式布局计算。
//!
//! # 架构设计
//!
//! ```text
//! HTML 字符串 → html5ever → DOM 树 → 样式计算 → 布局计算 → LayoutBox
//! ```
//!
//! # 示例
//!
//! ```rust
//! use iris_layout::html::parse_html;
//! use iris_layout::css::parse_stylesheet;
//! use iris_layout::layout::compute_layout;
//!
//! let html = r#"<div class="container"><p>Hello</p></div>"#;
//! let dom = parse_html(html);
//! ```

#![warn(missing_docs)]

pub mod dom;
pub mod html;
pub mod css;
pub mod style;
pub mod layout;
pub mod vdom;
pub mod domtree;

// 重新导出常用类型
pub use layout::{
    LayoutBox, BoxModel, LayoutType,
    FlexContainer, FlexItem, FlexLine,
    FlexDirection, FlexWrap, AlignContent,
    JustifyContent, AlignItems,
};
pub use css::{Selector, Stylesheet, CSSRule, SelectorType};
pub use dom::{DOMNode, NodeType};
pub use vdom::{VNode, VTree, VElement, Patch};
pub use domtree::DOMTree;

use iris_core;

/// 初始化布局引擎。
pub fn init() {
    iris_core::init();
    println!("iris-layout initialized");
}
