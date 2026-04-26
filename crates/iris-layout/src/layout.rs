//! 布局计算
//!
//! 实现盒模型和基础布局算法 (Flex/流式布局)。

use crate::dom::DOMNode;
use crate::style::ComputedStyles;

/// 盒模型
///
/// 表示元素在页面上占用的空间。
#[derive(Debug, Clone)]
pub struct BoxModel {
    /// 内容宽度
    pub content_width: f32,
    /// 内容高度
    pub content_height: f32,
    /// 内边距 (上, 右, 下, 左)
    pub padding: (f32, f32, f32, f32),
    /// 边框 (上, 右, 下, 左)
    pub border: (f32, f32, f32, f32),
    /// 外边距 (上, 右, 下, 左)
    pub margin: (f32, f32, f32, f32),
}

impl BoxModel {
    /// 创建新的盒模型
    pub fn new() -> Self {
        Self {
            content_width: 0.0,
            content_height: 0.0,
            padding: (0.0, 0.0, 0.0, 0.0),
            border: (0.0, 0.0, 0.0, 0.0),
            margin: (0.0, 0.0, 0.0, 0.0),
        }
    }

    /// 总宽度 (内容 + padding + border + margin)
    pub fn total_width(&self) -> f32 {
        self.content_width
            + self.padding.1
            + self.padding.3
            + self.border.1
            + self.border.3
            + self.margin.1
            + self.margin.3
    }

    /// 总高度 (内容 + padding + border + margin)
    pub fn total_height(&self) -> f32 {
        self.content_height
            + self.padding.0
            + self.padding.2
            + self.border.0
            + self.border.2
            + self.margin.0
            + self.margin.2
    }
}

/// 布局框
///
/// 表示元素在页面上的最终位置和尺寸。
#[derive(Debug, Clone)]
pub struct LayoutBox {
    /// X 坐标 (相对于父容器)
    pub x: f32,
    /// Y 坐标 (相对于父容器)
    pub y: f32,
    /// 宽度
    pub width: f32,
    /// 高度
    pub height: f32,
    /// 盒模型
    pub box_model: BoxModel,
}

impl LayoutBox {
    /// 创建新的布局框
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            box_model: BoxModel::new(),
        }
    }

    /// 创建指定位置的布局框
    pub fn with_position(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            box_model: BoxModel::new(),
        }
    }
}

/// 布局类型
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutType {
    /// 流式布局 (块级元素)
    Flow,
    /// Flex 布局
    Flex,
    /// 内联布局
    Inline,
}

/// 计算单个节点的布局
///
/// # 示例
///
/// ```rust
/// use iris_layout::dom::DOMNode;
/// use iris_layout::style::ComputedStyles;
/// use iris_layout::layout::{compute_node_layout, LayoutBox};
///
/// let node = DOMNode::new_element("div");
/// let styles = ComputedStyles::new();
/// let layout = compute_node_layout(&node, &styles, 800.0, 600.0, 0.0, 0.0);
///
/// assert_eq!(layout.x, 0.0);
/// assert_eq!(layout.y, 0.0);
/// ```
pub fn compute_node_layout(
    node: &DOMNode,
    styles: &ComputedStyles,
    parent_width: f32,
    parent_height: f32,
    offset_x: f32,
    offset_y: f32,
) -> LayoutBox {
    let mut layout = LayoutBox::new();

    // 1. 解析盒模型属性
    parse_box_model(&mut layout, styles, parent_width);

    // 2. 设置位置
    layout.x = offset_x;
    layout.y = offset_y;

    // 3. 如果是块级元素，宽度默认占满父容器
    if node.is_element() {
        if layout.width == 0.0 {
            layout.width = parent_width - layout.box_model.margin.1 - layout.box_model.margin.3;
        }
    }

    layout
}

/// 解析盒模型属性
fn parse_box_model(layout: &mut LayoutBox, styles: &ComputedStyles, parent_width: f32) {
    // 解析宽度
    if let Some(width) = styles.get("width") {
        layout.width = parse_length(width, parent_width);
        layout.box_model.content_width = layout.width;
    }

    // 解析高度
    if let Some(height) = styles.get("height") {
        layout.height = parse_length(height, 0.0); // 需要父高度
        layout.box_model.content_height = layout.height;
    }

    // 解析 padding
    if let Some(padding) = styles.get("padding") {
        let values = parse_spacing(padding);
        layout.box_model.padding = values;
    }

    // 解析 margin
    if let Some(margin) = styles.get("margin") {
        let values = parse_spacing(margin);
        layout.box_model.margin = values;
    }

    // 解析 border
    if let Some(border) = styles.get("border-width") {
        let values = parse_spacing(border);
        layout.box_model.border = values;
    }
}

/// 解析长度值 (支持 px 和 %)
fn parse_length(value: &str, reference: f32) -> f32 {
    let value = value.trim();

    if value.ends_with('%') {
        // 百分比
        let percent: f32 = value[..value.len() - 1]
            .parse()
            .unwrap_or(0.0);
        reference * percent / 100.0
    } else if value.ends_with("px") {
        // 像素
        value[..value.len() - 2].parse().unwrap_or(0.0)
    } else {
        // 默认解析为像素
        value.parse().unwrap_or(0.0)
    }
}

/// 解析间距值 (支持 1-4 个值)
///
/// - 1 个值: 所有边
/// - 2 个值: 上下, 左右
/// - 3 个值: 上, 左右, 下
/// - 4 个值: 上, 右, 下, 左
fn parse_spacing(value: &str) -> (f32, f32, f32, f32) {
    let values: Vec<f32> = value
        .split_whitespace()
        .map(|v| parse_length(v, 0.0))
        .collect();

    match values.len() {
        1 => (values[0], values[0], values[0], values[0]),
        2 => (values[0], values[1], values[0], values[1]),
        3 => (values[0], values[1], values[2], values[1]),
        4 => (values[0], values[1], values[2], values[3]),
        _ => (0.0, 0.0, 0.0, 0.0),
    }
}

/// 计算整个 DOM 树的布局
///
/// 递归处理所有节点，考虑父容器的尺寸和偏移。
///
/// # 示例
///
/// ```rust
/// use iris_layout::html::parse_html;
/// use iris_layout::css::parse_stylesheet;
/// use iris_layout::layout::compute_layout;
///
/// let html = r#"<div><p>Hello</p></div>"#;
/// let css = "div { padding: 10px; }";
///
/// let mut dom_tree = parse_html(html);
/// let stylesheet = parse_stylesheet(css);
///
/// compute_layout(dom_tree.root_mut(), &stylesheet, 800.0, 600.0);
/// ```
pub fn compute_layout(
    node: &mut DOMNode,
    stylesheet: &crate::css::Stylesheet,
    viewport_width: f32,
    viewport_height: f32,
) {
    use crate::style::compute_tree_styles;

    // 先计算样式
    compute_tree_styles(node, stylesheet, None);

    // 再计算布局
    compute_layout_recursive(node, viewport_width, viewport_height, 0.0, 0.0);
}

/// 递归计算布局
fn compute_layout_recursive(
    node: &mut DOMNode,
    parent_width: f32,
    parent_height: f32,
    offset_x: f32,
    offset_y: f32,
) {
    if !node.is_element() {
        return;
    }

    // 获取计算样式 (简化：这里应该从节点获取存储的样式)
    let styles = ComputedStyles::new();

    // 计算当前节点布局
    let layout = compute_node_layout(node, &styles, parent_width, parent_height, offset_x, offset_y);

    // 递归处理子节点
    let mut current_y = layout.y + layout.height;
    for child in &mut node.children {
        compute_layout_recursive(
            child,
            layout.width,
            layout.height,
            layout.x,
            current_y,
        );

        // 更新下一个子节点的 Y 偏移
        // 简化实现：简单堆叠
        current_y += 20.0; // 假设每个子节点高度 20px
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_length_px() {
        assert_eq!(parse_length("100px", 0.0), 100.0);
        assert_eq!(parse_length("50px", 0.0), 50.0);
    }

    #[test]
    fn test_parse_length_percent() {
        assert_eq!(parse_length("50%", 800.0), 400.0);
        assert_eq!(parse_length("100%", 600.0), 600.0);
    }

    #[test]
    fn test_parse_spacing_one_value() {
        let (top, right, bottom, left) = parse_spacing("10px");
        assert_eq!((top, right, bottom, left), (10.0, 10.0, 10.0, 10.0));
    }

    #[test]
    fn test_parse_spacing_two_values() {
        let (top, right, bottom, left) = parse_spacing("10px 20px");
        assert_eq!((top, right, bottom, left), (10.0, 20.0, 10.0, 20.0));
    }

    #[test]
    fn test_parse_spacing_four_values() {
        let (top, right, bottom, left) = parse_spacing("10px 20px 30px 40px");
        assert_eq!((top, right, bottom, left), (10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn test_box_model_total_size() {
        let mut box_model = BoxModel::new();
        box_model.content_width = 100.0;
        box_model.content_height = 50.0;
        box_model.padding = (10.0, 20.0, 10.0, 20.0);
        box_model.border = (0.0, 0.0, 0.0, 0.0);
        box_model.margin = (5.0, 10.0, 5.0, 10.0);

        assert_eq!(box_model.total_width(), 160.0); // 100 + 20 + 20 + 0 + 0 + 10 + 10
        assert_eq!(box_model.total_height(), 80.0); // 50 + 10 + 10 + 0 + 0 + 5 + 5
    }

    #[test]
    fn test_compute_simple_layout() {
        let node = DOMNode::new_element("div");
        let styles = ComputedStyles::new();
        let layout = compute_node_layout(&node, &styles, 800.0, 600.0, 0.0, 0.0);

        // 块级元素应该占满父容器
        assert_eq!(layout.width, 800.0);
    }
}
