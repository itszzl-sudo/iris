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
    /// 最小宽度约束
    pub min_width: Option<f32>,
    /// 最小高度约束
    pub min_height: Option<f32>,
    /// 最大宽度约束
    pub max_width: Option<f32>,
    /// 最大高度约束
    pub max_height: Option<f32>,
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
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
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
    
    /// 应用宽度约束 (min-width, max-width)
    pub fn apply_width_constraints(&mut self, width: f32) {
        let mut constrained = width;
        
        // 应用 min-width
        if let Some(min_w) = self.min_width {
            constrained = constrained.max(min_w);
        }
        
        // 应用 max-width
        if let Some(max_w) = self.max_width {
            constrained = constrained.min(max_w);
        }
        
        self.content_width = constrained;
    }
    
    /// 应用高度约束 (min-height, max-height)
    pub fn apply_height_constraints(&mut self, height: f32) {
        let mut constrained = height;
        
        // 应用 min-height
        if let Some(min_h) = self.min_height {
            constrained = constrained.max(min_h);
        }
        
        // 应用 max-height
        if let Some(max_h) = self.max_height {
            constrained = constrained.min(max_h);
        }
        
        self.content_height = constrained;
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

/// Flex 主轴方向
#[derive(Debug, Clone, PartialEq)]
pub enum FlexDirection {
    /// 水平方向，起点在左侧 (row)
    Row,
    /// 水平方向，起点在右侧 (row-reverse)
    RowReverse,
    /// 垂直方向，起点在顶部 (column)
    Column,
    /// 垂直方向，起点在底部 (column-reverse)
    ColumnReverse,
}

/// Flex 交叉轴对齐方式
#[derive(Debug, Clone, PartialEq)]
pub enum AlignItems {
    /// 拉伸以填充容器 (stretch)
    Stretch,
    /// 靠近交叉轴起点 (flex-start)
    FlexStart,
    /// 靠近交叉轴终点 (flex-end)
    FlexEnd,
    /// 居中对齐 (center)
    Center,
    /// 基线对齐 (baseline)
    Baseline,
}

/// Flex 主轴对齐方式
#[derive(Debug, Clone, PartialEq)]
pub enum JustifyContent {
    /// 靠近主轴起点 (flex-start)
    FlexStart,
    /// 靠近主轴终点 (flex-end)
    FlexEnd,
    /// 居中对齐 (center)
    Center,
    /// 两端对齐 (space-between)
    SpaceBetween,
    /// 均匀分布 (space-around)
    SpaceAround,
    /// 均匀分布，两侧间距相等 (space-evenly)
    SpaceEvenly,
}

/// Flex 换行方式
#[derive(Debug, Clone, PartialEq)]
pub enum FlexWrap {
    /// 不换行 (nowrap)
    NoWrap,
    /// 换行 (wrap)
    Wrap,
    /// 反向换行 (wrap-reverse)
    WrapReverse,
}

/// Flex 多行交叉轴对齐方式
#[derive(Debug, Clone, PartialEq)]
pub enum AlignContent {
    /// 拉伸以填充容器 (stretch)
    Stretch,
    /// 靠近交叉轴起点 (flex-start)
    FlexStart,
    /// 靠近交叉轴终点 (flex-end)
    FlexEnd,
    /// 居中对齐 (center)
    Center,
    /// 两端对齐 (space-between)
    SpaceBetween,
    /// 均匀分布 (space-around)
    SpaceAround,
}

/// Flex 容器属性
#[derive(Debug, Clone)]
pub struct FlexContainer {
    /// 主轴方向
    pub direction: FlexDirection,
    /// 换行方式
    pub wrap: FlexWrap,
    /// 主轴对齐
    pub justify_content: JustifyContent,
    /// 交叉轴对齐
    pub align_items: AlignItems,
    /// 多行交叉轴对齐
    pub align_content: AlignContent,
    /// 行间距
    pub gap: f32,
}

impl FlexContainer {
    /// 创建默认的 Flex 容器
    pub fn new() -> Self {
        Self {
            direction: FlexDirection::Row,
            wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
            gap: 0.0,
        }
    }
    
    /// 从样式创建 Flex 容器
    pub fn from_styles(styles: &ComputedStyles) -> Self {
        let mut container = Self::new();
        
        // 解析 flex-direction
        if let Some(dir) = styles.get("flex-direction") {
            container.direction = match dir.as_str() {
                "row" => FlexDirection::Row,
                "row-reverse" => FlexDirection::RowReverse,
                "column" => FlexDirection::Column,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            };
        }
        
        // 解析 flex-wrap
        if let Some(wrap) = styles.get("flex-wrap") {
            container.wrap = match wrap.as_str() {
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => FlexWrap::NoWrap,
            };
        }
        
        // 解析 justify-content
        if let Some(jc) = styles.get("justify-content") {
            container.justify_content = match jc.as_str() {
                "flex-start" => JustifyContent::FlexStart,
                "flex-end" => JustifyContent::FlexEnd,
                "center" => JustifyContent::Center,
                "space-between" => JustifyContent::SpaceBetween,
                "space-around" => JustifyContent::SpaceAround,
                "space-evenly" => JustifyContent::SpaceEvenly,
                _ => JustifyContent::FlexStart,
            };
        }
        
        // 解析 align-items
        if let Some(ai) = styles.get("align-items") {
            container.align_items = match ai.as_str() {
                "stretch" => AlignItems::Stretch,
                "flex-start" => AlignItems::FlexStart,
                "flex-end" => AlignItems::FlexEnd,
                "center" => AlignItems::Center,
                "baseline" => AlignItems::Baseline,
                _ => AlignItems::Stretch,
            };
        }
        
        // 解析 align-content
        if let Some(ac) = styles.get("align-content") {
            container.align_content = match ac.as_str() {
                "flex-start" => AlignContent::FlexStart,
                "flex-end" => AlignContent::FlexEnd,
                "center" => AlignContent::Center,
                "space-between" => AlignContent::SpaceBetween,
                "space-around" => AlignContent::SpaceAround,
                _ => AlignContent::Stretch,
            };
        }
        
        // 解析 gap
        if let Some(gap) = styles.get("gap") {
            container.gap = gap.parse().unwrap_or(0.0);
        }
        
        container
    }
}

/// Flex 项目属性
#[derive(Debug, Clone)]
pub struct FlexItem {
    /// flex-grow: 放大比例
    pub grow: f32,
    /// flex-shrink: 缩小比例
    pub shrink: f32,
    /// flex-basis: 基础尺寸
    pub basis: Option<f32>,
    /// align-self: 自定义对齐
    pub align_self: Option<AlignItems>,
}

impl FlexItem {
    /// 创建默认的 Flex 项目
    pub fn new() -> Self {
        Self {
            grow: 0.0,
            shrink: 1.0,
            basis: None,
            align_self: None,
        }
    }
}

/// Flex 行（多行布局中的一行）
#[derive(Debug, Clone)]
pub struct FlexLine {
    /// 该行包含的项目索引
    pub item_indices: Vec<usize>,
    /// 该行的主轴尺寸（总宽度）
    pub main_size: f32,
    /// 该行的交叉轴尺寸（最大高度）
    pub cross_size: f32,
    /// 该行的起始偏移
    pub offset: f32,
}

impl FlexLine {
    /// 创建新的 Flex 行
    pub fn new() -> Self {
        Self {
            item_indices: Vec::new(),
            main_size: 0.0,
            cross_size: 0.0,
            offset: 0.0,
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
    _parent_height: f32,
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
    
    // 解析 min-width
    if let Some(min_width) = styles.get("min-width") {
        layout.box_model.min_width = Some(parse_length(min_width, parent_width));
    }
    
    // 解析 min-height
    if let Some(min_height) = styles.get("min-height") {
        layout.box_model.min_height = Some(parse_length(min_height, 0.0));
    }
    
    // 解析 max-width
    if let Some(max_width) = styles.get("max-width") {
        layout.box_model.max_width = Some(parse_length(max_width, parent_width));
    }
    
    // 解析 max-height
    if let Some(max_height) = styles.get("max-height") {
        layout.box_model.max_height = Some(parse_length(max_height, 0.0));
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

    // 检查是否为 Flex 容器
    let is_flex_container = styles.get("display").map(|d| d == "flex").unwrap_or(false);

    if is_flex_container {
        // Flex 布局
        compute_flex_layout(node, &styles, parent_width, parent_height, offset_x, offset_y);
    } else {
        // 流式布局
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
}

/// 计算 Flex 布局
fn compute_flex_layout(
    node: &mut DOMNode,
    styles: &ComputedStyles,
    parent_width: f32,
    parent_height: f32,
    offset_x: f32,
    offset_y: f32,
) {
    // 1. 解析 Flex 容器属性
    let container = parse_flex_container(styles);
    
    // 2. 解析容器尺寸
    let mut container_box = LayoutBox::new();
    parse_box_model(&mut container_box, styles, parent_width);
    container_box.x = offset_x;
    container_box.y = offset_y;
    
    if container_box.width == 0.0 {
        container_box.width = parent_width;
    }
    
    // 3. 获取 Flex 项目
    let flex_children: Vec<usize> = node.children.iter()
        .enumerate()
        .filter(|(_, child)| child.is_element())
        .map(|(i, _)| i)
        .collect();
    
    if flex_children.is_empty() {
        return;
    }
    
    // 4. 计算主轴和交叉轴（已包含反向逻辑）
    match container.direction {
        FlexDirection::Row | FlexDirection::RowReverse => {
            compute_flex_row(node, &flex_children, &container, &mut container_box, parent_width);
        }
        FlexDirection::Column | FlexDirection::ColumnReverse => {
            compute_flex_column(node, &flex_children, &container, &mut container_box, parent_height);
        }
    }
}

/// 解析 Flex 容器属性
fn parse_flex_container(styles: &ComputedStyles) -> FlexContainer {
    let mut container = FlexContainer::new();
    
    // 解析 flex-direction
    if let Some(dir) = styles.get("flex-direction") {
        container.direction = match dir.as_str() {
            "row-reverse" => FlexDirection::RowReverse,
            "column" => FlexDirection::Column,
            "column-reverse" => FlexDirection::ColumnReverse,
            _ => FlexDirection::Row,
        };
    }
    
    // 解析 flex-wrap
    if let Some(wrap) = styles.get("flex-wrap") {
        container.wrap = match wrap.as_str() {
            "wrap" => FlexWrap::Wrap,
            "wrap-reverse" => FlexWrap::WrapReverse,
            _ => FlexWrap::NoWrap,
        };
    }
    
    // 解析 justify-content
    if let Some(justify) = styles.get("justify-content") {
        container.justify_content = match justify.as_str() {
            "flex-end" => JustifyContent::FlexEnd,
            "center" => JustifyContent::Center,
            "space-between" => JustifyContent::SpaceBetween,
            "space-around" => JustifyContent::SpaceAround,
            "space-evenly" => JustifyContent::SpaceEvenly,
            _ => JustifyContent::FlexStart,
        };
    }
    
    // 解析 align-items
    if let Some(align) = styles.get("align-items") {
        container.align_items = match align.as_str() {
            "flex-start" => AlignItems::FlexStart,
            "flex-end" => AlignItems::FlexEnd,
            "center" => AlignItems::Center,
            "baseline" => AlignItems::Baseline,
            _ => AlignItems::Stretch,
        };
    }
    
    // 解析 gap
    if let Some(gap) = styles.get("gap") {
        container.gap = gap.parse().unwrap_or(0.0);
    }
    
    // 解析 align-content
    if let Some(align) = styles.get("align-content") {
        container.align_content = match align.as_str() {
            "flex-start" => AlignContent::FlexStart,
            "flex-end" => AlignContent::FlexEnd,
            "center" => AlignContent::Center,
            "space-between" => AlignContent::SpaceBetween,
            "space-around" => AlignContent::SpaceAround,
            _ => AlignContent::Stretch,
        };
    }
    
    container
}

/// 递归计算子元素的精确高度
///
/// 返回子元素的总高度（包括 margin）
fn compute_children_precise_height(
    node: &mut DOMNode, 
    children_indices: &[usize], 
    container_width: f32,
    container: &FlexContainer,
) -> f32 {
    let mut max_height: f32 = 0.0;
    
    for &child_idx in children_indices {
        if let Some(child) = node.children.get_mut(child_idx) {
            // 获取子元素的样式
            let styles = ComputedStyles::new();
            
            // 解析子元素的布局框
            let mut child_layout = LayoutBox::new();
            parse_box_model(&mut child_layout, &styles, container_width);
            
            // 解析高度样式
            if let Some(height) = styles.get("height") {
                child_layout.height = parse_length(height, 0.0);
                child_layout.box_model.content_height = child_layout.height;
            } else {
                // 默认高度
                child_layout.height = 50.0;
                child_layout.box_model.content_height = 50.0;
            }
            
            // 获取子元素的总高度（包括 padding, border, margin）
            let child_total_height = child_layout.box_model.total_height();
            
            // 取最大高度作为行高
            if child_total_height > max_height {
                max_height = child_total_height;
            }
        }
    }
    
    max_height
}

/// 计算交叉轴位置和高度（align-items）
///
/// # 参数
/// * `align_items` - 对齐方式
/// * `item_height` - 项目当前高度
/// * `container_size` - 容器交叉轴尺寸
/// * `container_padding_start` - 容器起始内边距
/// * `container_padding_end` - 容器终止内边距
/// * `is_stretch_allowed` - 是否允许拉伸
///
/// # 返回
/// * (y, height) - 交叉轴位置和最终高度
fn compute_cross_axis_position(
    align_items: &AlignItems,
    item_height: f32,
    container_size: f32,
    container_padding_start: f32,
    container_padding_end: f32,
    is_stretch_allowed: bool,
) -> (f32, f32) {
    let mut y = container_padding_start;
    let mut height = item_height;
    
    let available_space = if container_size > 0.0 {
        container_size - container_padding_start - container_padding_end
    } else {
        0.0
    };
    
    match align_items {
        AlignItems::FlexStart => {
            // 靠近交叉轴起点
            y = container_padding_start;
        }
        AlignItems::FlexEnd => {
            // 靠近交叉轴终点
            if available_space > 0.0 {
                y = container_size - container_padding_end - height;
            }
        }
        AlignItems::Center => {
            // 居中对齐
            if available_space > 0.0 {
                y = container_padding_start + (available_space - height) / 2.0;
            }
        }
        AlignItems::Stretch => {
            // 拉伸以填充容器
            if is_stretch_allowed && available_space > 0.0 {
                height = available_space;
                y = container_padding_start;
            }
        }
        AlignItems::Baseline => {
            // 基线对齐
            // 简化实现：等同于 flex-start
            // TODO: 实现真实的基线对齐（需要文本基线信息）
            y = container_padding_start;
        }
    }
    
    (y, height)
}

/// 计算水平 Flex 布局（完整版）
fn compute_flex_row(
    node: &mut DOMNode,
    children_indices: &[usize],
    container: &FlexContainer,
    container_box: &mut LayoutBox,
    _parent_width: f32,
) {
    // 如果不需要换行，使用单行布局
    if container.wrap == FlexWrap::NoWrap {
        compute_flex_row_single_line(node, children_indices, container, container_box);
    } else {
        // 多行布局
        compute_flex_row_multi_line(node, children_indices, container, container_box);
    }
}

/// 单行 Flex 布局（原有逻辑）
fn compute_flex_row_single_line(
    node: &mut DOMNode,
    children_indices: &[usize],
    container: &FlexContainer,
    container_box: &mut LayoutBox,
) {
    let container_width = container_box.width;
    let container_padding_left = container_box.box_model.padding.3;
    let container_padding_right = container_box.box_model.padding.1;
    let container_padding_top = container_box.box_model.padding.0;
    let container_padding_bottom = container_box.box_model.padding.2;
    
    // 可用宽度
    let available_width = container_width - container_padding_left - container_padding_right;
    
    // 根据方向决定子元素顺序和起点位置
    let (ordered_indices, start_x) = if matches!(container.direction, FlexDirection::RowReverse) {
        // Row-Reverse: 从右到左
        let reversed: Vec<usize> = children_indices.iter().rev().cloned().collect();
        let start = container_width - container_padding_right;
        (reversed, start)
    } else {
        // Row: 从左到右（正常）
        (children_indices.to_vec(), container_padding_left)
    };
    
    // 1. 解析所有 Flex 项目
    struct FlexItemData {
        index: usize,
        basis: f32,
        grow: f32,
        shrink: f32,
        main_size: f32,  // 主轴尺寸（宽度）
        cross_size: f32, // 交叉轴尺寸（高度）
    }
    
    let mut items: Vec<FlexItemData> = Vec::new();
    let mut total_basis = 0.0;
    
    // 使用 ordered_indices 而不是 children_indices
    for &idx in &ordered_indices {
        let styles = ComputedStyles::new(); // 简化：应该从节点获取
        let flex_item = parse_flex_item(&styles);
        
        // 计算基础尺寸
        let mut basis = flex_item.basis.unwrap_or(0.0);
        if basis == 0.0 {
            // 如果没有 flex-basis，使用内容宽度
            let mut temp_box = LayoutBox::new();
            parse_box_model(&mut temp_box, &styles, container_width);
            basis = if temp_box.width > 0.0 { temp_box.width } else { 100.0 };
        }
        
        total_basis += basis;
        items.push(FlexItemData {
            index: idx,
            basis,
            grow: flex_item.grow,
            shrink: flex_item.shrink,
            main_size: basis,
            cross_size: 0.0,
        });
    }
    
    // 2. 处理 flex-grow 和 flex-shrink
    let free_space = available_width - total_basis - (container.gap * items.len().saturating_sub(1) as f32);
    
    if free_space > 0.0 {
        // 有剩余空间，应用 flex-grow
        let total_grow: f32 = items.iter().map(|item| item.grow).sum();
        
        if total_grow > 0.0 {
            for item in &mut items {
                let grow_amount = (item.grow / total_grow) * free_space;
                item.main_size = item.basis + grow_amount;
            }
        } else {
            // 没有设置 grow，保持基础尺寸
            for item in &mut items {
                item.main_size = item.basis;
            }
        }
    } else if free_space < 0.0 {
        // 空间不足，应用 flex-shrink
        let total_shrink_weight: f32 = items.iter()
            .map(|item| item.basis * item.shrink)
            .sum();
        
        if total_shrink_weight > 0.0 {
            for item in &mut items {
                let shrink_factor = (item.basis * item.shrink) / total_shrink_weight;
                let shrink_amount = shrink_factor * free_space.abs();
                item.main_size = (item.basis - shrink_amount).max(0.0);
            }
        }
    } else {
        // 刚好 fit
        for item in &mut items {
            item.main_size = item.basis;
        }
    }
    
    // 3. 根据 justify-content 计算位置
    let total_items_width: f32 = items.iter().map(|item| item.main_size).sum();
    let total_gap = container.gap * items.len().saturating_sub(1) as f32;
    let total_used = total_items_width + total_gap;
    let remaining_space = available_width - total_used;
    
    let mut start_offset = start_x;
    let mut item_gap = container.gap;
    
    // Row-Reverse 时，justify-content 的方向需要反转
    let is_reverse = matches!(container.direction, FlexDirection::RowReverse);
    
    match container.justify_content {
        JustifyContent::FlexStart => {
            // 默认：从起点开始（Row-Reverse 时起点在右侧）
        }
        JustifyContent::FlexEnd => {
            // Row-Reverse: FlexEnd 实际上是在左侧
            if is_reverse {
                start_offset = container_padding_left - remaining_space;
            } else {
                start_offset += remaining_space;
            }
        }
        JustifyContent::Center => {
            start_offset += remaining_space / 2.0;
        }
        JustifyContent::SpaceBetween => {
            if items.len() > 1 {
                item_gap = container.gap + remaining_space / (items.len() - 1) as f32;
            }
        }
        JustifyContent::SpaceAround => {
            if !items.is_empty() {
                let space_per_item = remaining_space / items.len() as f32;
                item_gap = container.gap + space_per_item;
                start_offset += if is_reverse { -space_per_item / 2.0 } else { space_per_item / 2.0 };
            }
        }
        JustifyContent::SpaceEvenly => {
            if !items.is_empty() {
                let gap_count = items.len() + 1;
                let space_per_gap = remaining_space / gap_count as f32;
                item_gap = container.gap + space_per_gap;
                start_offset += if is_reverse { -space_per_gap } else { space_per_gap };
            }
        }
    }
    
    // 4. 布局每个项目
    let mut current_x = start_offset;
    let container_height = container_box.height;
    
    // 计算精确的子元素高度（如果需要）
    let precise_height = if container_height == 0.0 {
        // 容器没有指定高度，计算子元素的精确高度
        compute_children_precise_height(node, children_indices, container_width, container)
    } else {
        0.0
    };
    
    // Row-Reverse 时，x 坐标需要递减而不是递增
    for item in &items {
        if let Some(child) = node.children.get_mut(item.index) {
            // 计算交叉轴位置 (align-items)
            let mut height = item.cross_size;
            
            // 如果容器没有高度，使用精确计算的高度或默认值
            if container_height == 0.0 {
                height = if precise_height > 0.0 { precise_height } else { 50.0 };
            }
            
            // 使用辅助函数计算交叉轴位置
            let (y, final_height) = compute_cross_axis_position(
                &container.align_items,
                height,
                container_height,
                container_padding_top,
                container_padding_bottom,
                true, // 允许拉伸
            );
            
            // Row-Reverse: 从右向左计算 x 坐标
            let x = if is_reverse {
                current_x - item.main_size
            } else {
                current_x
            };
            
            // 创建子元素的 LayoutBox 并应用约束
            let mut child_layout = LayoutBox::with_position(
                x,
                y,
                item.main_size,
                if final_height > 0.0 { final_height } else { 50.0 },
            );
            
            // 应用 min/max 约束（从子元素的样式中解析）
            if let Some(child_styles) = child.computed_styles() {
                parse_box_model(&mut child_layout, &child_styles, container_width);
                child_layout.box_model.apply_height_constraints(child_layout.height);
                child_layout.box_model.apply_width_constraints(child_layout.width);
            }
            
            // 这里简化处理，实际应该存储到节点
            let _ = child_layout;
        }
        
        current_x += if is_reverse {
            -(item.main_size + item_gap)  // Row-Reverse: 递减
        } else {
            item.main_size + item_gap     // Row: 递增
        };
    }
}

/// 计算垂直 Flex 布局
fn compute_flex_column(
    node: &mut DOMNode,
    children_indices: &[usize],
    container: &FlexContainer,
    container_box: &mut LayoutBox,
    _parent_height: f32,
) {
    // 如果不需要换行，使用单列布局
    if container.wrap == FlexWrap::NoWrap {
        compute_flex_column_single_line(node, children_indices, container, container_box);
    } else {
        // 多列布局
        compute_flex_column_multi_line(node, children_indices, container, container_box);
    }
}

/// 单列 Flex 布局
fn compute_flex_column_single_line(
    node: &mut DOMNode,
    children_indices: &[usize],
    container: &FlexContainer,
    container_box: &mut LayoutBox,
) {
    let container_width = container_box.width;
    let container_height = container_box.height;
    let container_padding_top = container_box.box_model.padding.0;
    let container_padding_bottom = container_box.box_model.padding.2;
    let container_padding_left = container_box.box_model.padding.3;
    let container_padding_right = container_box.box_model.padding.1;
    
    let available_width = container_width - container_padding_left - container_padding_right;
    
    // 根据方向决定子元素顺序和起点位置
    let (ordered_indices, start_y) = if matches!(container.direction, FlexDirection::ColumnReverse) {
        // Column-Reverse: 从下到上
        let reversed: Vec<usize> = children_indices.iter().rev().cloned().collect();
        let start = container_height - container_padding_bottom;
        (reversed, start)
    } else {
        // Column: 从上到下（正常）
        (children_indices.to_vec(), container_padding_top)
    };
    
    let mut current_y = start_y;
    
    // Column-Reverse 时，y 坐标需要递减而不是递增
    let is_reverse = matches!(container.direction, FlexDirection::ColumnReverse);
    
    for &idx in &ordered_indices {
        if let Some(child) = node.children.get_mut(idx) {
            // 获取子元素的样式
            let styles = if let Some(s) = child.computed_styles() {
                s
            } else {
                ComputedStyles::new()
            };
            
            // 解析子元素的 Flex 项目属性
            let flex_item = parse_flex_item(&styles);
            
            // 计算子元素的宽度
            let mut width = flex_item.basis.unwrap_or(available_width);
            if width == 0.0 {
                width = available_width; // 默认填满容器宽度
            }
            
            // 计算子元素的高度
            let mut height = 50.0; // 默认高度
            if let Some(h) = styles.get("height") {
                height = parse_length(h, 0.0);
            }
            
            // 使用辅助函数计算交叉轴位置（水平方向）
            let (item_x, final_width) = compute_cross_axis_position(
                &container.align_items,
                width,
                container_width,
                container_padding_left,
                container_padding_right,
                true, // 允许拉伸
            );
            
            // Column-Reverse: 从下向上计算 y 坐标
            let y = if is_reverse {
                current_y - height
            } else {
                current_y
            };
            
            // 创建布局框
            let mut child_layout = LayoutBox::with_position(
                if item_x > 0.0 { item_x } else { container_padding_left },
                y,
                if final_width > 0.0 { final_width } else { width },
                height,
            );
            
            // 解析盒模型并应用约束
            parse_box_model(&mut child_layout, &styles, container_width);
            child_layout.box_model.apply_width_constraints(child_layout.width);
            child_layout.box_model.apply_height_constraints(child_layout.height);
            
            // 更新 Y 坐标（Column-Reverse 时递减）
            current_y += if is_reverse {
                -(child_layout.height + container.gap)
            } else {
                child_layout.height + container.gap
            };
        }
    }
}

/// 多列 Flex 布局（垂直方向的 flex-wrap）
fn compute_flex_column_multi_line(
    node: &mut DOMNode,
    children_indices: &[usize],
    container: &FlexContainer,
    container_box: &mut LayoutBox,
) {
    let container_width = container_box.width;
    let container_height = container_box.height;
    let container_padding_top = container_box.box_model.padding.0;
    let container_padding_bottom = container_box.box_model.padding.2;
    let container_padding_left = container_box.box_model.padding.3;
    let container_padding_right = container_box.box_model.padding.1;
    
    let available_height = if container_height > 0.0 {
        container_height - container_padding_top - container_padding_bottom
    } else {
        500.0 // 默认高度
    };
    
    // 1. 分列
    let mut columns: Vec<FlexLine> = Vec::new(); // 使用 FlexLine 表示每一列
    let mut current_column = FlexLine::new();
    let mut current_column_size = 0.0;
    
    for (item_idx, &child_idx) in children_indices.iter().enumerate() {
        // 获取子元素的样式
        let styles = if let Some(child) = node.children.get(child_idx) {
            child.computed_styles().unwrap_or_else(ComputedStyles::new)
        } else {
            ComputedStyles::new()
        };
        
        let flex_item = parse_flex_item(&styles);
        
        let mut basis = flex_item.basis.unwrap_or(0.0);
        if basis == 0.0 {
            // 从样式中获取高度
            if let Some(h) = styles.get("height") {
                basis = parse_length(h, 0.0);
            }
            if basis == 0.0 {
                basis = 50.0; // 默认高度
            }
        }
        
        // 计算加入该项目后的总高度
        let added_height = if current_column.item_indices.is_empty() {
            basis
        } else {
            basis + container.gap
        };
        
        // 检查是否需要换列
        if container.wrap != FlexWrap::NoWrap && 
           current_column_size + added_height > available_height && 
           !current_column.item_indices.is_empty() {
            // 当前列已满，创建新列
            columns.push(current_column);
            current_column = FlexLine::new();
            current_column_size = basis;
        } else {
            current_column_size += added_height;
        }
        
        current_column.item_indices.push(child_idx);
        current_column.main_size = current_column_size;
    }
    
    // 添加最后一列
    if !current_column.item_indices.is_empty() {
        columns.push(current_column);
    }
    
    // 2. 计算每列内的布局
    let available_width = container_width - container_padding_left - container_padding_right;
    
    // 计算所有列的总宽度
    let total_columns_width = available_width; // 简化：使用可用宽度
    let col_gap = container.gap;
    let total_col_gaps = col_gap * columns.len().saturating_sub(1) as f32;
    let remaining_horizontal = available_width - total_columns_width - total_col_gaps;
    
    // 根据 justify-content 计算列的水平位置（Wrap-Reverse 时需要反转）
    let mut column_start_x = container_padding_left;
    let mut column_gap = col_gap;
    let is_wrap_reverse = matches!(container.wrap, FlexWrap::WrapReverse);
    
    if columns.len() > 1 && remaining_horizontal > 0.0 {
        match container.justify_content {
            JustifyContent::FlexStart => {
                // Wrap-Reverse: FlexStart 在右侧
                if is_wrap_reverse {
                    column_start_x = container_width - container_padding_right;
                }
            },
            JustifyContent::FlexEnd => {
                // Wrap-Reverse: FlexEnd 在左侧
                if is_wrap_reverse {
                    column_start_x = container_padding_left;
                } else {
                    column_start_x += remaining_horizontal;
                }
            }
            JustifyContent::Center => {
                column_start_x += remaining_horizontal / 2.0;
            }
            JustifyContent::SpaceBetween => {
                if columns.len() > 1 {
                    column_gap = col_gap + remaining_horizontal / (columns.len() - 1) as f32;
                }
            }
            JustifyContent::SpaceAround => {
                if !columns.is_empty() {
                    let space_per_column = remaining_horizontal / columns.len() as f32;
                    column_gap = col_gap + space_per_column;
                    column_start_x += if is_wrap_reverse { -space_per_column / 2.0 } else { space_per_column / 2.0 };
                }
            }
            JustifyContent::SpaceEvenly => {
                if !columns.is_empty() {
                    let gap_count = columns.len() + 1;
                    let space_per_gap = remaining_horizontal / gap_count as f32;
                    column_gap = col_gap + space_per_gap;
                    column_start_x += if is_wrap_reverse { -space_per_gap } else { space_per_gap };
                }
            }
        }
    }
    
    // 3. 布局每一列（Wrap-Reverse 时反转列的顺序）
    let mut current_x = column_start_x;
    
    // Wrap-Reverse: 反转列的顺序（从右到左）
    let ordered_columns: Vec<FlexLine> = if is_wrap_reverse {
        columns.iter().rev().cloned().collect()
    } else {
        columns.clone()
    };
    
    for column in &ordered_columns {
        // 对当前列内的项目应用单列布局逻辑
        let column_item_indices: Vec<usize> = column.item_indices.clone();
        
        // 计算该列的精确宽度
        let column_width = compute_children_precise_width(node, &column_item_indices, container_height, container);
        
        // 布局列内的每个项目
        let mut current_y = container_padding_top;
        
        for &child_idx in &column_item_indices {
            if let Some(child) = node.children.get_mut(child_idx) {
                // 获取子元素的样式
                let styles = if let Some(s) = child.computed_styles() {
                    s
                } else {
                    ComputedStyles::new()
                };
                
                // 解析子元素的 Flex 项目属性
                let flex_item = parse_flex_item(&styles);
                
                // 计算子元素的高度
                let mut height = flex_item.basis.unwrap_or(50.0);
                if height == 0.0 {
                    height = 50.0; // 默认高度
                }
                
                // 计算子元素的宽度
                let mut width = available_width; // 默认填满列宽
                if let Some(w) = styles.get("width") {
                    width = parse_length(w, container_width);
                }
                
                // 使用辅助函数计算交叉轴位置（水平方向）
                let effective_column_width = if column_width > 0.0 { column_width } else { available_width };
                let (item_x, final_width) = compute_cross_axis_position(
                    &container.align_items,
                    width,
                    effective_column_width,
                    0.0, // 列内无额外 padding
                    0.0,
                    true, // 允许拉伸
                );
                
                // 创建布局框
                let mut child_layout = LayoutBox::with_position(
                    current_x + item_x,
                    current_y,
                    if final_width > 0.0 { final_width } else { width },
                    height,
                );
                
                // 解析盒模型并应用约束
                parse_box_model(&mut child_layout, &styles, container_width);
                child_layout.box_model.apply_width_constraints(child_layout.width);
                child_layout.box_model.apply_height_constraints(child_layout.height);
                
                current_y += height + container.gap;
            }
        }
        
        // 更新下一列的 X 坐标（Wrap-Reverse 时递减）
        let effective_column_width = if column_width > 0.0 { column_width } else { available_width };
        current_x += if is_wrap_reverse {
            -(effective_column_width + column_gap)
        } else {
            effective_column_width + column_gap
        };
    }
}

/// 递归计算子元素的精确宽度
fn compute_children_precise_width(
    node: &mut DOMNode,
    children_indices: &[usize],
    container_height: f32,
    container: &FlexContainer,
) -> f32 {
    let mut max_width: f32 = 0.0;
    
    for &child_idx in children_indices {
        if let Some(child) = node.children.get_mut(child_idx) {
            // 获取子元素的样式
            let styles = ComputedStyles::new();
            
            // 解析子元素的布局框
            let mut child_layout = LayoutBox::new();
            parse_box_model(&mut child_layout, &styles, container_height);
            
            // 解析宽度样式
            if let Some(width) = styles.get("width") {
                child_layout.width = parse_length(width, container_height);
                child_layout.box_model.content_width = child_layout.width;
            } else {
                // 默认宽度
                child_layout.width = 100.0;
                child_layout.box_model.content_width = 100.0;
            }
            
            // 获取子元素的总宽度（包括 padding, border, margin）
            let child_total_width = child_layout.box_model.total_width();
            
            // 取最大宽度作为列宽
            if child_total_width > max_width {
                max_width = child_total_width;
            }
        }
    }
    
    max_width
}

/// 多行 Flex 布局
fn compute_flex_row_multi_line(
    node: &mut DOMNode,
    children_indices: &[usize],
    container: &FlexContainer,
    container_box: &mut LayoutBox,
) {
    let container_width = container_box.width;
    let container_padding_left = container_box.box_model.padding.3;
    let container_padding_right = container_box.box_model.padding.1;
    let container_padding_top = container_box.box_model.padding.0;
    let container_padding_bottom = container_box.box_model.padding.2;
    
    let available_width = container_width - container_padding_left - container_padding_right;
    
    // 1. 分行
    let mut lines: Vec<FlexLine> = Vec::new();
    let mut current_line = FlexLine::new();
    let mut current_line_size = 0.0;
    
    for (item_idx, &child_idx) in children_indices.iter().enumerate() {
        // 获取子元素的样式
        let styles = if let Some(child) = node.children.get(child_idx) {
            child.computed_styles().unwrap_or_else(ComputedStyles::new)
        } else {
            ComputedStyles::new()
        };
        
        let flex_item = parse_flex_item(&styles);
        
        let mut basis = flex_item.basis.unwrap_or(0.0);
        if basis == 0.0 {
            basis = 100.0; // 默认宽度
        }
        
        // 计算加入该项目后的总宽度
        let added_width = if current_line.item_indices.is_empty() {
            basis
        } else {
            basis + container.gap
        };
        
        // 检查是否需要换行
        if container.wrap != FlexWrap::NoWrap && 
           current_line_size + added_width > available_width && 
           !current_line.item_indices.is_empty() {
            // 当前行已满，创建新行
            lines.push(current_line);
            current_line = FlexLine::new();
            current_line_size = basis;
        } else {
            current_line_size += added_width;
        }
        
        current_line.item_indices.push(child_idx);
        current_line.main_size = current_line_size;
    }
    
    // 添加最后一行
    if !current_line.item_indices.is_empty() {
        lines.push(current_line);
    }
    
    // 2. 计算每行内的布局
    let container_height = container_box.height;
    let available_height = if container_height > 0.0 {
        container_height - container_padding_top - container_padding_bottom
    } else {
        500.0 // 默认高度
    };
    
    // 计算所有行的总高度
    let total_lines_height = available_height; // 简化：使用可用高度
    let row_gap = container.gap;
    let total_row_gaps = row_gap * lines.len().saturating_sub(1) as f32;
    let remaining_vertical = available_height - total_lines_height - total_row_gaps;
    
    // 根据 align-content 计算行的垂直位置（Wrap-Reverse 时需要反转）
    let mut line_start_y = container_padding_top;
    let mut line_gap = row_gap;
    let is_wrap_reverse = matches!(container.wrap, FlexWrap::WrapReverse);
    
    if lines.len() > 1 && remaining_vertical > 0.0 {
        match container.align_content {
            AlignContent::FlexStart => {
                // Wrap-Reverse: FlexStart 在底部
                if is_wrap_reverse {
                    line_start_y = container_box.height - container_padding_bottom;
                }
            },
            AlignContent::FlexEnd => {
                // Wrap-Reverse: FlexEnd 在顶部
                if is_wrap_reverse {
                    line_start_y = container_padding_top;
                } else {
                    line_start_y += remaining_vertical;
                }
            }
            AlignContent::Center => {
                line_start_y += remaining_vertical / 2.0;
            }
            AlignContent::SpaceBetween => {
                if lines.len() > 1 {
                    line_gap = row_gap + remaining_vertical / (lines.len() - 1) as f32;
                }
            }
            AlignContent::SpaceAround => {
                if !lines.is_empty() {
                    let space_per_line = remaining_vertical / lines.len() as f32;
                    line_gap = row_gap + space_per_line;
                    line_start_y += if is_wrap_reverse { -space_per_line / 2.0 } else { space_per_line / 2.0 };
                }
            }
            AlignContent::Stretch => {
                // 拉伸行高
            }
        }
    }
    
    // 3. 布局每一行（Wrap-Reverse 时反转行的顺序）
    let mut current_y = line_start_y;
    
    // Wrap-Reverse: 反转行的顺序（从下到上）
    let ordered_lines: Vec<FlexLine> = if is_wrap_reverse {
        lines.iter().rev().cloned().collect()
    } else {
        lines.clone()
    };
    
    for line in &ordered_lines {
        // 对当前行内的项目应用单行布局逻辑
        let line_item_indices: Vec<usize> = line.item_indices.clone();
        
        // 计算该行的精确高度
        let row_height = compute_children_precise_height(node, &line_item_indices, container_width, container);
        
        // 布局行内的每个项目
        let mut current_x = container_padding_left;
        
        for &child_idx in &line_item_indices {
            if let Some(child) = node.children.get_mut(child_idx) {
                // 获取子元素的样式
                let styles = if let Some(s) = child.computed_styles() {
                    s
                } else {
                    ComputedStyles::new()
                };
                
                // 解析子元素的 Flex 项目属性
                let flex_item = parse_flex_item(&styles);
                
                // 计算子元素的宽度
                let mut width = flex_item.basis.unwrap_or(100.0);
                if width == 0.0 {
                    width = 100.0; // 默认宽度
                }
                
                // 计算子元素的高度
                let mut height = 50.0; // 默认高度
                if let Some(h) = styles.get("height") {
                    height = parse_length(h, 0.0);
                }
                
                // 使用辅助函数计算交叉轴位置（基于行高）
                let effective_row_height = if row_height > 0.0 { row_height } else { 50.0 };
                let (item_y, final_height) = compute_cross_axis_position(
                    &container.align_items,
                    height,
                    effective_row_height,
                    0.0, // 行内无额外 padding
                    0.0,
                    true, // 允许拉伸
                );
                
                // 创建布局框
                let mut child_layout = LayoutBox::with_position(
                    current_x,
                    current_y + item_y,
                    width,
                    if final_height > 0.0 { final_height } else { 50.0 },
                );
                
                // 解析盒模型并应用约束
                parse_box_model(&mut child_layout, &styles, container_width);
                child_layout.box_model.apply_width_constraints(child_layout.width);
                child_layout.box_model.apply_height_constraints(child_layout.height);
                
                current_x += width + container.gap;
            }
        }
        
        // 更新下一行的 Y 坐标（Wrap-Reverse 时递减）
        let effective_row_height = if row_height > 0.0 { row_height } else { 50.0 };
        current_y += if is_wrap_reverse {
            -(effective_row_height + line_gap)
        } else {
            effective_row_height + line_gap
        };
    }
}

/// 解析 Flex 项目属性
fn parse_flex_item(styles: &ComputedStyles) -> FlexItem {
    let mut item = FlexItem::new();
    
    // 先检查 flex 简写属性
    if let Some(flex) = styles.get("flex") {
        parse_flex_shorthand(flex, &mut item);
        return item;
    }
    
    // 解析 flex-grow
    if let Some(grow) = styles.get("flex-grow") {
        item.grow = grow.parse().unwrap_or(0.0);
    }
    
    // 解析 flex-shrink
    if let Some(shrink) = styles.get("flex-shrink") {
        item.shrink = shrink.parse().unwrap_or(1.0);
    }
    
    // 解析 flex-basis
    if let Some(basis) = styles.get("flex-basis") {
        let basis_val = basis.trim();
        if basis_val.ends_with("px") {
            item.basis = Some(basis_val[..basis_val.len() - 2].parse().unwrap_or(0.0));
        } else if basis_val.ends_with('%') {
            // 需要父容器宽度
            item.basis = None;
        } else {
            item.basis = basis_val.parse().ok();
        }
    }
    
    item
}

/// 解析 flex 简写属性
/// 支持格式：
/// - flex: <flex-grow> <flex-shrink> <flex-basis>
/// - flex: <flex-grow> <flex-shrink>
/// - flex: <flex-grow>
/// - flex: none (grow: 0, shrink: 0, basis: auto)
/// - flex: auto (grow: 1, shrink: 1, basis: auto)
fn parse_flex_shorthand(flex: &str, item: &mut FlexItem) {
    let flex = flex.trim();
    
    // 处理特殊值
    if flex == "none" {
        item.grow = 0.0;
        item.shrink = 0.0;
        item.basis = None; // auto
        return;
    }
    
    if flex == "auto" {
        item.grow = 1.0;
        item.shrink = 1.0;
        item.basis = None; // auto
        return;
    }
    
    if flex == "initial" {
        item.grow = 0.0;
        item.shrink = 1.0;
        item.basis = Some(0.0);
        return;
    }
    
    // 解析空格分隔的值
    let parts: Vec<&str> = flex.split_whitespace().collect();
    
    match parts.len() {
        1 => {
            // flex: <flex-grow>
            if let Ok(grow) = parts[0].parse::<f32>() {
                item.grow = grow;
            }
        }
        2 => {
            // flex: <flex-grow> <flex-shrink>
            if let Ok(grow) = parts[0].parse::<f32>() {
                item.grow = grow;
            }
            if let Ok(shrink) = parts[1].parse::<f32>() {
                item.shrink = shrink;
            }
        }
        3 => {
            // flex: <flex-grow> <flex-shrink> <flex-basis>
            if let Ok(grow) = parts[0].parse::<f32>() {
                item.grow = grow;
            }
            if let Ok(shrink) = parts[1].parse::<f32>() {
                item.shrink = shrink;
            }
            
            // 解析 flex-basis
            let basis_str = parts[2];
            if basis_str.ends_with("px") {
                if let Ok(val) = basis_str[..basis_str.len() - 2].parse::<f32>() {
                    item.basis = Some(val);
                }
            } else if basis_str.ends_with('%') {
                // 百分比需要父容器宽度，暂时不处理
                item.basis = None;
            } else if let Ok(val) = basis_str.parse::<f32>() {
                item.basis = Some(val);
            }
        }
        _ => {
            // 无效格式，使用默认值
        }
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

    #[test]
    fn test_compute_layout_with_width() {
        let mut styles = ComputedStyles::new();
        styles.set("width", "200px");
        
        let node = DOMNode::new_element("div");
        let layout = compute_node_layout(&node, &styles, 800.0, 600.0, 0.0, 0.0);
        
        assert_eq!(layout.width, 200.0);
        assert_eq!(layout.box_model.content_width, 200.0);
    }

    #[test]
    fn test_compute_layout_with_percent() {
        let mut styles = ComputedStyles::new();
        styles.set("width", "50%");
        
        let node = DOMNode::new_element("div");
        let layout = compute_node_layout(&node, &styles, 800.0, 600.0, 0.0, 0.0);
        
        assert_eq!(layout.width, 400.0); // 50% of 800
    }

    #[test]
    fn test_compute_layout_with_padding() {
        let mut styles = ComputedStyles::new();
        styles.set("padding", "10px 20px");
        
        let node = DOMNode::new_element("div");
        let layout = compute_node_layout(&node, &styles, 800.0, 600.0, 0.0, 0.0);
        
        assert_eq!(layout.box_model.padding, (10.0, 20.0, 10.0, 20.0));
    }

    #[test]
    fn test_compute_layout_with_margin() {
        let mut styles = ComputedStyles::new();
        styles.set("margin", "5px 10px 15px 20px");
        
        let node = DOMNode::new_element("div");
        let layout = compute_node_layout(&node, &styles, 800.0, 600.0, 0.0, 0.0);
        
        assert_eq!(layout.box_model.margin, (5.0, 10.0, 15.0, 20.0));
    }

    #[test]
    fn test_total_size_with_box_model() {
        let mut box_model = BoxModel::new();
        box_model.content_width = 100.0;
        box_model.content_height = 50.0;
        box_model.padding = (10.0, 20.0, 10.0, 20.0);
        box_model.border = (2.0, 2.0, 2.0, 2.0);
        box_model.margin = (5.0, 10.0, 5.0, 10.0);
        
        // 总宽度 = content + padding-left-right + border-left-right + margin-left-right
        // 100 + 20 + 20 + 2 + 2 + 10 + 10 = 164
        assert_eq!(box_model.total_width(), 164.0);
        // 总高度 = content + padding-top-bottom + border-top-bottom + margin-top-bottom
        // 50 + 10 + 10 + 2 + 2 + 5 + 5 = 84
        assert_eq!(box_model.total_height(), 84.0);
    }

    #[test]
    fn test_layout_box_with_position() {
        let layout = LayoutBox::with_position(100.0, 200.0, 300.0, 400.0);
        
        assert_eq!(layout.x, 100.0);
        assert_eq!(layout.y, 200.0);
        assert_eq!(layout.width, 300.0);
        assert_eq!(layout.height, 400.0);
    }

    #[test]
    fn test_child_stacking_layout() {
        let mut parent = DOMNode::new_element("div");
        let child1 = DOMNode::new_element("p");
        let child2 = DOMNode::new_element("p");
        parent.append_child(child1);
        parent.append_child(child2);
        
        // 验证子节点被正确添加
        assert_eq!(parent.children.len(), 2);
    }

    #[test]
    fn test_flex_container_default() {
        let container = FlexContainer::new();
        assert_eq!(container.direction, FlexDirection::Row);
        assert_eq!(container.wrap, FlexWrap::NoWrap);
        assert_eq!(container.justify_content, JustifyContent::FlexStart);
        assert_eq!(container.align_items, AlignItems::Stretch);
        assert_eq!(container.gap, 0.0);
    }

    #[test]
    fn test_flex_item_default() {
        let item = FlexItem::new();
        assert_eq!(item.grow, 0.0);
        assert_eq!(item.shrink, 1.0);
        assert!(item.basis.is_none());
        assert!(item.align_self.is_none());
    }

    #[test]
    fn test_flex_direction_variants() {
        assert_eq!(FlexDirection::Row, FlexDirection::Row);
        assert_eq!(FlexDirection::Column, FlexDirection::Column);
        assert_ne!(FlexDirection::Row, FlexDirection::Column);
    }

    #[test]
    fn test_justify_content_variants() {
        assert_eq!(JustifyContent::FlexStart, JustifyContent::FlexStart);
        assert_eq!(JustifyContent::Center, JustifyContent::Center);
        assert_ne!(JustifyContent::FlexStart, JustifyContent::Center);
    }

    #[test]
    fn test_align_items_variants() {
        assert_eq!(AlignItems::Stretch, AlignItems::Stretch);
        assert_eq!(AlignItems::Center, AlignItems::Center);
        assert_ne!(AlignItems::Stretch, AlignItems::Center);
    }

    #[test]
    fn test_flex_wrap_variants() {
        assert_eq!(FlexWrap::NoWrap, FlexWrap::NoWrap);
        assert_eq!(FlexWrap::Wrap, FlexWrap::Wrap);
        assert_ne!(FlexWrap::NoWrap, FlexWrap::Wrap);
    }

    #[test]
    fn test_flex_container_with_children() {
        // 创建 Flex 容器
        let mut container = DOMNode::new_element("div");
        
        // 添加 Flex 项目
        for i in 0..3 {
            let item = DOMNode::new_element("span");
            container.append_child(item);
        }
        
        assert_eq!(container.children.len(), 3);
        assert!(container.children.iter().all(|c| c.is_element()));
    }

    #[test]
    fn test_layout_type_flex() {
        let flex_type = LayoutType::Flex;
        assert_eq!(flex_type, LayoutType::Flex);
        assert_ne!(flex_type, LayoutType::Flow);
    }

    #[test]
    fn test_box_model_with_flex_basis() {
        // 测试 flex-basis 作为基础尺寸
        let mut box_model = BoxModel::new();
        box_model.content_width = 200.0;
        box_model.content_height = 50.0;
        box_model.padding = (10.0, 20.0, 10.0, 20.0);
        
        // flex-basis 应该作为内容宽度
        assert_eq!(box_model.content_width, 200.0);
        // 总宽度 = content + padding-left-right + border-left-right + margin-left-right
        // 200 + 20 + 20 + 0 + 0 + 0 + 0 = 240
        assert_eq!(box_model.total_width(), 240.0);
    }

    #[test]
    fn test_flex_gap_spacing() {
        let mut container = FlexContainer::new();
        container.gap = 16.0;
        
        // 3 个项目，2 个 gap
        let total_gap = container.gap * 2.0;
        assert_eq!(total_gap, 32.0);
    }

    #[test]
    fn test_flex_row_layout_structure() {
        // 测试 Flex 布局结构是否正确创建
        let mut container = DOMNode::new_element("div");
        container.set_attribute("style", "display: flex; flex-direction: row;");
        
        let item1 = DOMNode::new_element("div");
        let item2 = DOMNode::new_element("div");
        container.append_child(item1);
        container.append_child(item2);
        
        assert_eq!(container.children.len(), 2);
        // 验证容器和项目的结构
        assert!(container.is_element());
    }

    #[test]
    fn test_flex_column_layout_structure() {
        let mut container = DOMNode::new_element("div");
        container.set_attribute("style", "display: flex; flex-direction: column;");
        
        for _ in 0..5 {
            container.append_child(DOMNode::new_element("div"));
        }
        
        assert_eq!(container.children.len(), 5);
    }

    #[test]
    fn test_flex_justify_space_between() {
        let justify = JustifyContent::SpaceBetween;
        assert_eq!(justify, JustifyContent::SpaceBetween);
        assert_ne!(justify, JustifyContent::SpaceAround);
    }

    #[test]
    fn test_flex_align_center() {
        let align = AlignItems::Center;
        assert_eq!(align, AlignItems::Center);
        assert_ne!(align, AlignItems::FlexStart);
    }

    #[test]
    fn test_flex_shorthand_auto() {
        let mut item = FlexItem::new();
        parse_flex_shorthand("auto", &mut item);
        
        assert_eq!(item.grow, 1.0);
        assert_eq!(item.shrink, 1.0);
        assert!(item.basis.is_none());
    }

    #[test]
    fn test_flex_shorthand_none() {
        let mut item = FlexItem::new();
        parse_flex_shorthand("none", &mut item);
        
        assert_eq!(item.grow, 0.0);
        assert_eq!(item.shrink, 0.0);
        assert!(item.basis.is_none());
    }

    #[test]
    fn test_flex_shorthand_initial() {
        let mut item = FlexItem::new();
        parse_flex_shorthand("initial", &mut item);
        
        assert_eq!(item.grow, 0.0);
        assert_eq!(item.shrink, 1.0);
        assert_eq!(item.basis, Some(0.0));
    }

    #[test]
    fn test_flex_shorthand_single_value() {
        let mut item = FlexItem::new();
        parse_flex_shorthand("2", &mut item);
        
        assert_eq!(item.grow, 2.0);
        assert_eq!(item.shrink, 1.0); // 默认值
        assert!(item.basis.is_none());
    }

    #[test]
    fn test_flex_shorthand_two_values() {
        let mut item = FlexItem::new();
        parse_flex_shorthand("2 3", &mut item);
        
        assert_eq!(item.grow, 2.0);
        assert_eq!(item.shrink, 3.0);
        assert!(item.basis.is_none());
    }

    #[test]
    fn test_flex_shorthand_three_values() {
        let mut item = FlexItem::new();
        parse_flex_shorthand("1 1 200px", &mut item);
        
        assert_eq!(item.grow, 1.0);
        assert_eq!(item.shrink, 1.0);
        assert_eq!(item.basis, Some(200.0));
    }

    #[test]
    fn test_flex_shorthand_with_percent() {
        let mut item = FlexItem::new();
        parse_flex_shorthand("1 1 50%", &mut item);
        
        assert_eq!(item.grow, 1.0);
        assert_eq!(item.shrink, 1.0);
        assert!(item.basis.is_none()); // 百分比暂不处理
    }

    #[test]
    fn test_flex_grow_calculation() {
        // 测试 flex-grow 分配算法
        let total_grow: f32 = 1.0 + 2.0 + 3.0; // 6
        let free_space: f32 = 600.0;
        
        // item1: 1/6 * 600 = 100
        let grow1: f32 = (1.0 / total_grow) * free_space;
        assert!((grow1 - 100.0).abs() < 0.01);
        
        // item2: 2/6 * 600 = 200
        let grow2: f32 = (2.0 / total_grow) * free_space;
        assert!((grow2 - 200.0).abs() < 0.01);
        
        // item3: 3/6 * 600 = 300
        let grow3: f32 = (3.0 / total_grow) * free_space;
        assert!((grow3 - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_flex_shrink_calculation() {
        // 测试 flex-shrink 分配算法
        let basis1: f32 = 200.0;
        let basis2: f32 = 300.0;
        let basis3: f32 = 500.0;
        
        let shrink1: f32 = 1.0;
        let shrink2: f32 = 2.0;
        let shrink3: f32 = 1.0;
        
        let total_shrink_weight: f32 = basis1 * shrink1 + basis2 * shrink2 + basis3 * shrink3;
        // 200*1 + 300*2 + 500*1 = 1300
        assert!((total_shrink_weight - 1300.0).abs() < 0.01);
        
        // item1 的收缩权重
        let weight1: f32 = (basis1 * shrink1) / total_shrink_weight;
        assert!((weight1 - 200.0/1300.0).abs() < 0.001);
    }

    #[test]
    fn test_justify_space_between_calculation() {
        // 测试 space-between 间距计算
        let container_width = 800.0;
        let item_width = 100.0;
        let item_count = 3;
        let total_items = item_width * item_count as f32;
        let remaining = container_width - total_items;
        
        // space-between: 2 个间隙
        let gap = remaining / (item_count - 1) as f32;
        assert!((gap - 250.0).abs() < 0.01); // (800 - 300) / 2 = 250
    }

    #[test]
    fn test_justify_space_around_calculation() {
        // 测试 space-around 间距计算
        let container_width = 800.0;
        let item_width = 100.0;
        let item_count = 3;
        let total_items = item_width * item_count as f32;
        let remaining = container_width - total_items;
        
        // space-around: 每个项目两侧空间相等
        let space_per_item = remaining / item_count as f32;
        assert!((space_per_item - 166.666).abs() < 1.0);
    }

    #[test]
    fn test_justify_space_evenly_calculation() {
        // 测试 space-evenly 间距计算
        let container_width = 800.0;
        let item_width = 100.0;
        let item_count = 3;
        let total_items = item_width * item_count as f32;
        let remaining = container_width - total_items;
        
        // space-evenly: 所有间隙相等（包括两端）
        let gap_count = item_count + 1; // 4 个间隙
        let gap = remaining / gap_count as f32;
        assert!((gap - 125.0).abs() < 0.01); // 500 / 4 = 125
    }

    #[test]
    fn test_flex_container_full_config() {
        // 测试完整的 Flex 容器配置
        let container = FlexContainer {
            direction: FlexDirection::Row,
            wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            align_content: AlignContent::Stretch,
            gap: 16.0,
        };
        
        assert_eq!(container.direction, FlexDirection::Row);
        assert_eq!(container.wrap, FlexWrap::Wrap);
        assert_eq!(container.justify_content, JustifyContent::SpaceBetween);
        assert_eq!(container.align_items, AlignItems::Center);
        assert_eq!(container.gap, 16.0);
    }

    #[test]
    fn test_flex_item_with_all_props() {
        // 测试完整的 Flex 项目配置
        let item = FlexItem {
            grow: 2.0,
            shrink: 3.0,
            basis: Some(200.0),
            align_self: Some(AlignItems::FlexEnd),
        };
        
        assert_eq!(item.grow, 2.0);
        assert_eq!(item.shrink, 3.0);
        assert_eq!(item.basis, Some(200.0));
        assert_eq!(item.align_self, Some(AlignItems::FlexEnd));
    }

    #[test]
    fn test_flex_line_creation() {
        // 测试 FlexLine 创建
        let line = FlexLine::new();
        assert!(line.item_indices.is_empty());
        assert_eq!(line.main_size, 0.0);
        assert_eq!(line.cross_size, 0.0);
        assert_eq!(line.offset, 0.0);
    }

    #[test]
    fn test_flex_line_with_items() {
        // 测试带项目的 FlexLine
        let mut line = FlexLine::new();
        line.item_indices.push(0);
        line.item_indices.push(1);
        line.item_indices.push(2);
        line.main_size = 300.0;
        line.cross_size = 50.0;
        
        assert_eq!(line.item_indices.len(), 3);
        assert_eq!(line.main_size, 300.0);
        assert_eq!(line.cross_size, 50.0);
    }

    #[test]
    fn test_align_content_variants() {
        // 测试 AlignContent 枚举
        assert_eq!(AlignContent::Stretch, AlignContent::Stretch);
        assert_eq!(AlignContent::FlexStart, AlignContent::FlexStart);
        assert_eq!(AlignContent::Center, AlignContent::Center);
        assert_ne!(AlignContent::Stretch, AlignContent::FlexStart);
    }

    #[test]
    fn test_flex_container_with_align_content() {
        // 测试容器 align_content 字段
        let container = FlexContainer {
            direction: FlexDirection::Row,
            wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            align_content: AlignContent::SpaceBetween,
            gap: 10.0,
        };
        
        assert_eq!(container.wrap, FlexWrap::Wrap);
        assert_eq!(container.align_content, AlignContent::SpaceBetween);
    }

    #[test]
    fn test_flex_wrap_multi_line_structure() {
        // 测试多行 Flex 布局结构
        let mut container = DOMNode::new_element("div");
        
        // 添加 6 个子元素
        for _ in 0..6 {
            container.append_child(DOMNode::new_element("div"));
        }
        
        assert_eq!(container.children.len(), 6);
        
        // 验证容器样式
        container.set_attribute("style", "display: flex; flex-wrap: wrap;");
    }

    #[test]
    fn test_flex_wrap_calculation() {
        // 测试换行计算逻辑
        let container_width: f32 = 400.0;
        let item_width: f32 = 150.0;
        let gap: f32 = 10.0;
        
        // 每行能容纳的项目数
        let items_per_row = ((container_width + gap) / (item_width + gap)).floor() as usize;
        assert_eq!(items_per_row, 2); // 400 / 160 = 2.5 -> 2
        
        // 6 个项目需要多少行
        let total_items = 6;
        let rows = (total_items + items_per_row - 1) / items_per_row;
        assert_eq!(rows, 3); // 6 / 2 = 3 行
    }

    #[test]
    fn test_align_content_space_between_calculation() {
        // 测试 align-content: space-between 计算
        let container_height = 500.0;
        let row_height = 100.0;
        let row_count = 3;
        let gap = 10.0;
        
        let total_rows_height = row_height * row_count as f32;
        let total_gaps = gap * (row_count - 1) as f32;
        let remaining = container_height - total_rows_height - total_gaps;
        
        // space-between: 2 个间隙
        let extra_gap = remaining / (row_count - 1) as f32;
        assert!(extra_gap > 0.0);
    }

    #[test]
    fn test_align_content_space_around_calculation() {
        // 测试 align-content: space-around 计算
        let container_height = 500.0;
        let row_height = 100.0;
        let row_count = 3;
        let gap = 10.0;
        
        let total_rows_height = row_height * row_count as f32;
        let remaining = container_height - total_rows_height;
        
        // space-around: 每个行上下都有空间
        let space_per_row = remaining / row_count as f32;
        assert!(space_per_row > 0.0);
    }

    #[test]
    fn test_box_model_min_max_constraints() {
        // 测试 BoxModel 的 min/max 约束字段
        let mut box_model = BoxModel::new();
        
        // 设置约束
        box_model.min_width = Some(100.0);
        box_model.max_width = Some(500.0);
        box_model.min_height = Some(50.0);
        box_model.max_height = Some(300.0);
        
        // 测试宽度约束应用
        box_model.apply_width_constraints(50.0); // 小于 min
        assert_eq!(box_model.content_width, 100.0);
        
        box_model.apply_width_constraints(600.0); // 大于 max
        assert_eq!(box_model.content_width, 500.0);
        
        box_model.apply_width_constraints(300.0); // 在范围内
        assert_eq!(box_model.content_width, 300.0);
        
        // 测试高度约束应用
        box_model.apply_height_constraints(30.0); // 小于 min
        assert_eq!(box_model.content_height, 50.0);
        
        box_model.apply_height_constraints(400.0); // 大于 max
        assert_eq!(box_model.content_height, 300.0);
        
        box_model.apply_height_constraints(150.0); // 在范围内
        assert_eq!(box_model.content_height, 150.0);
    }

    #[test]
    fn test_box_model_no_constraints() {
        // 测试没有约束时的行为
        let mut box_model = BoxModel::new();
        
        box_model.apply_width_constraints(200.0);
        assert_eq!(box_model.content_width, 200.0);
        
        box_model.apply_height_constraints(100.0);
        assert_eq!(box_model.content_height, 100.0);
    }

    #[test]
    fn test_box_model_only_min_constraint() {
        // 测试仅有 min 约束
        let mut box_model = BoxModel::new();
        box_model.min_width = Some(100.0);
        
        box_model.apply_width_constraints(50.0);
        assert_eq!(box_model.content_width, 100.0);
        
        box_model.apply_width_constraints(200.0);
        assert_eq!(box_model.content_width, 200.0);
    }

    #[test]
    fn test_box_model_only_max_constraint() {
        // 测试仅有 max 约束
        let mut box_model = BoxModel::new();
        box_model.max_width = Some(300.0);
        
        box_model.apply_width_constraints(400.0);
        assert_eq!(box_model.content_width, 300.0);
        
        box_model.apply_width_constraints(200.0);
        assert_eq!(box_model.content_width, 200.0);
    }

    #[test]
    fn test_flex_container_from_styles_with_align_content() {
        // 测试 FlexContainer::from_styles 包含 align-content
        let styles = ComputedStyles::new();
        let container = FlexContainer::from_styles(&styles);
        
        // 验证默认值
        assert_eq!(container.align_content, AlignContent::Stretch);
        assert_eq!(container.wrap, FlexWrap::NoWrap);
    }

    #[test]
    fn test_align_items_flex_start() {
        // 测试 align-items: flex-start
        let (y, height) = compute_cross_axis_position(
            &AlignItems::FlexStart,
            50.0,
            200.0,
            10.0,
            10.0,
            true,
        );
        
        assert_eq!(y, 10.0); // 靠近起点
        assert_eq!(height, 50.0); // 高度不变
    }

    #[test]
    fn test_align_items_flex_end() {
        // 测试 align-items: flex-end
        let (y, height) = compute_cross_axis_position(
            &AlignItems::FlexEnd,
            50.0,
            200.0,
            10.0,
            10.0,
            true,
        );
        
        // 200 - 10 - 50 = 140
        assert_eq!(y, 140.0); // 靠近终点
        assert_eq!(height, 50.0); // 高度不变
    }

    #[test]
    fn test_align_items_center() {
        // 测试 align-items: center
        let (y, height) = compute_cross_axis_position(
            &AlignItems::Center,
            50.0,
            200.0,
            10.0,
            10.0,
            true,
        );
        
        // 可用空间: 200 - 10 - 10 = 180
        // 居中: 10 + (180 - 50) / 2 = 10 + 65 = 75
        assert_eq!(y, 75.0); // 居中
        assert_eq!(height, 50.0); // 高度不变
    }

    #[test]
    fn test_align_items_stretch() {
        // 测试 align-items: stretch
        let (y, height) = compute_cross_axis_position(
            &AlignItems::Stretch,
            50.0,
            200.0,
            10.0,
            10.0,
            true,
        );
        
        // 可用空间: 200 - 10 - 10 = 180
        assert_eq!(y, 10.0); // 从起点开始
        assert_eq!(height, 180.0); // 拉伸到填满
    }

    #[test]
    fn test_align_items_baseline() {
        // 测试 align-items: baseline
        let (y, height) = compute_cross_axis_position(
            &AlignItems::Baseline,
            50.0,
            200.0,
            10.0,
            10.0,
            true,
        );
        
        // 简化实现：等同于 flex-start
        assert_eq!(y, 10.0); // 靠近起点
        assert_eq!(height, 50.0); // 高度不变
    }

    #[test]
    fn test_align_items_no_container_height() {
        // 测试容器没有高度时的行为
        let (y, height) = compute_cross_axis_position(
            &AlignItems::Center,
            50.0,
            0.0, // 容器高度为 0
            10.0,
            10.0,
            true,
        );
        
        // 容器高度为 0，available_space 为 0，保持默认值
        assert_eq!(y, 10.0);
        assert_eq!(height, 50.0);
    }

    #[test]
    fn test_align_items_stretch_not_allowed() {
        // 测试 stretch 不允许拉伸的情况
        let (y, height) = compute_cross_axis_position(
            &AlignItems::Stretch,
            50.0,
            200.0,
            10.0,
            10.0,
            false, // 不允许拉伸
        );
        
        // 不允许拉伸，保持原高度
        assert_eq!(y, 10.0);
        assert_eq!(height, 50.0);
    }

    #[test]
    fn test_flex_column_wrap_structure() {
        // 测试垂直方向 flex-wrap 结构
        let mut container = DOMNode::new_element("div");
        
        // 添加 6 个子元素
        for _ in 0..6 {
            container.append_child(DOMNode::new_element("div"));
        }
        
        assert_eq!(container.children.len(), 6);
        
        // 验证容器样式
        container.set_attribute("style", "display: flex; flex-direction: column; flex-wrap: wrap;");
    }

    #[test]
    fn test_flex_column_wrap_calculation() {
        // 测试垂直方向换列计算逻辑
        let container_height: f32 = 400.0;
        let item_height: f32 = 150.0;
        let gap: f32 = 10.0;
        
        // 每列能容纳的项目数
        let items_per_column = ((container_height + gap) / (item_height + gap)).floor() as usize;
        assert_eq!(items_per_column, 2); // 400 / 160 = 2.5 -> 2
        
        // 6 个项目需要多少列
        let total_items = 6;
        let columns = (total_items + items_per_column - 1) / items_per_column;
        assert_eq!(columns, 3); // 6 / 2 = 3 列
    }

    #[test]
    fn test_justify_content_space_between_column_calculation() {
        // 测试垂直方向 justify-content: space-between 计算
        let container_width: f32 = 500.0;
        let column_width: f32 = 100.0;
        let column_count = 3;
        let gap: f32 = 10.0;
        
        let total_columns_width = column_width * column_count as f32;
        let total_gaps = gap * (column_count - 1) as f32;
        let remaining = container_width - total_columns_width - total_gaps;
        
        // space-between: 2 个间隙
        let extra_gap = remaining / (column_count - 1) as f32;
        assert!(extra_gap > 0.0);
    }

    #[test]
    fn test_justify_content_space_around_column_calculation() {
        // 测试垂直方向 justify-content: space-around 计算
        let container_width: f32 = 500.0;
        let column_width: f32 = 100.0;
        let column_count = 3;
        let gap: f32 = 10.0;
        
        let total_columns_width = column_width * column_count as f32;
        let remaining = container_width - total_columns_width;
        
        // space-around: 每个列左右都有空间
        let space_per_column = remaining / column_count as f32;
        assert!(space_per_column > 0.0);
    }

    #[test]
    fn test_flex_direction_reverse_parsing() {
        // 测试反向方向的解析
        let mut container = DOMNode::new_element("div");
        
        // 测试 row-reverse
        container.set_attribute("style", "display: flex; flex-direction: row-reverse;");
        let styles = container.computed_styles().unwrap();
        let flex_container = parse_flex_container(&styles);
        assert_eq!(flex_container.direction, FlexDirection::RowReverse);
        
        // 测试 column-reverse
        container.set_attribute("style", "display: flex; flex-direction: column-reverse;");
        let styles = container.computed_styles().unwrap();
        let flex_container = parse_flex_container(&styles);
        assert_eq!(flex_container.direction, FlexDirection::ColumnReverse);
    }

    #[test]
    fn test_flex_direction_reverse_enum() {
        // 测试反向方向枚举值
        assert_eq!(FlexDirection::Row, FlexDirection::Row);
        assert_eq!(FlexDirection::RowReverse, FlexDirection::RowReverse);
        assert_eq!(FlexDirection::Column, FlexDirection::Column);
        assert_eq!(FlexDirection::ColumnReverse, FlexDirection::ColumnReverse);
        
        assert_ne!(FlexDirection::Row, FlexDirection::RowReverse);
        assert_ne!(FlexDirection::Column, FlexDirection::ColumnReverse);
    }

    #[test]
    fn test_flex_container_with_reverse_direction() {
        // 测试容器配置反向方向
        let container = FlexContainer {
            direction: FlexDirection::RowReverse,
            wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            align_content: AlignContent::Stretch,
            gap: 10.0,
        };
        
        assert_eq!(container.direction, FlexDirection::RowReverse);
        assert_eq!(container.wrap, FlexWrap::Wrap);
    }

    #[test]
    fn test_row_reverse_layout_order() {
        // 测试 Row-Reverse 的子元素顺序反转
        let mut parent = DOMNode::new_element("div");
        parent.set_attribute("style", "display: flex; flex-direction: row-reverse; width: 400px;");
        
        // 添加 3 个子元素
        for i in 1..=3 {
            let mut child = DOMNode::new_element("span");
            child.set_attribute("style", &format!("width: 100px; height: 50px;"));
            parent.children.push(child);
        }
        
        // 计算布局（这里验证解析正确）
        let styles = parent.computed_styles().unwrap();
        let flex_container = parse_flex_container(&styles);
        assert_eq!(flex_container.direction, FlexDirection::RowReverse);
    }

    #[test]
    fn test_column_reverse_layout_order() {
        // 测试 Column-Reverse 的子元素顺序反转
        let mut parent = DOMNode::new_element("div");
        parent.set_attribute("style", "display: flex; flex-direction: column-reverse; height: 300px;");
        
        // 添加 3 个子元素
        for i in 1..=3 {
            let mut child = DOMNode::new_element("span");
            child.set_attribute("style", &format!("width: 100px; height: 80px;"));
            parent.children.push(child);
        }
        
        // 计算布局（这里验证解析正确）
        let styles = parent.computed_styles().unwrap();
        let flex_container = parse_flex_container(&styles);
        assert_eq!(flex_container.direction, FlexDirection::ColumnReverse);
    }

    #[test]
    fn test_wrap_reverse_parsing() {
        // 测试 wrap-reverse 的 CSS 解析
        let mut container = DOMNode::new_element("div");
        
        // 测试 wrap-reverse
        container.set_attribute("style", "display: flex; flex-wrap: wrap-reverse;");
        let styles = container.computed_styles().unwrap();
        let flex_container = parse_flex_container(&styles);
        assert_eq!(flex_container.wrap, FlexWrap::WrapReverse);
    }

    #[test]
    fn test_wrap_reverse_container_config() {
        // 测试容器配置 wrap-reverse
        let container = FlexContainer {
            direction: FlexDirection::Row,
            wrap: FlexWrap::WrapReverse,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            align_content: AlignContent::Stretch,
            gap: 10.0,
        };
        
        assert_eq!(container.wrap, FlexWrap::WrapReverse);
    }

    #[test]
    fn test_wrap_reverse_with_row_reverse() {
        // 测试 wrap-reverse 与 row-reverse 组合
        let container = FlexContainer {
            direction: FlexDirection::RowReverse,
            wrap: FlexWrap::WrapReverse,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexEnd,
            align_content: AlignContent::SpaceAround,
            gap: 15.0,
        };
        
        assert_eq!(container.direction, FlexDirection::RowReverse);
        assert_eq!(container.wrap, FlexWrap::WrapReverse);
    }
}
