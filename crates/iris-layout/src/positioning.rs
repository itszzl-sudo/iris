//! CSS 定位系统
//!
//! 实现 CSS 定位属性：
//! - static（默认）
//! - relative（相对定位）
//! - absolute（绝对定位）
//! - fixed（固定定位）
//! - sticky（粘性定位）

use crate::dom::DOMNode;
use crate::style::ComputedStyles;

/// 定位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionType {
    /// 静态定位（默认）
    Static,
    /// 相对定位
    Relative,
    /// 绝对定位
    Absolute,
    /// 固定定位
    Fixed,
    /// 粘性定位
    Sticky,
}

impl PositionType {
    /// 从 CSS 字符串解析
    pub fn from_css(css: &str) -> Self {
        match css.trim() {
            "static" => PositionType::Static,
            "relative" => PositionType::Relative,
            "absolute" => PositionType::Absolute,
            "fixed" => PositionType::Fixed,
            "sticky" => PositionType::Sticky,
            _ => PositionType::Static,
        }
    }

    /// 转换为 CSS 字符串
    pub fn to_css(&self) -> &'static str {
        match self {
            PositionType::Static => "static",
            PositionType::Relative => "relative",
            PositionType::Absolute => "absolute",
            PositionType::Fixed => "fixed",
            PositionType::Sticky => "sticky",
        }
    }

    /// 是否脱离文档流
    pub fn is_out_of_flow(&self) -> bool {
        matches!(self, PositionType::Absolute | PositionType::Fixed)
    }
}

/// 定位偏移值
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OffsetValue {
    /// 未设置（auto）
    Auto,
    /// 像素值
    Pixels(f32),
    /// 百分比（相对于包含块）
    Percentage(f32),
}

impl OffsetValue {
    /// 从 CSS 字符串解析
    pub fn from_css(css: &str) -> Self {
        let css = css.trim();
        if css == "auto" {
            return OffsetValue::Auto;
        }

        if css.ends_with('%') {
            if let Ok(val) = css.trim_end_matches('%').parse::<f32>() {
                return OffsetValue::Percentage(val);
            }
        }

        if let Ok(val) = css.trim_end_matches("px").parse::<f32>() {
            return OffsetValue::Pixels(val);
        }

        OffsetValue::Auto
    }

    /// 计算实际像素值
    pub fn to_pixels(&self, reference: f32) -> f32 {
        match self {
            OffsetValue::Auto => 0.0,
            OffsetValue::Pixels(px) => *px,
            OffsetValue::Percentage(pct) => reference * pct / 100.0,
        }
    }

    /// 转换为 CSS 字符串
    pub fn to_css(&self) -> String {
        match self {
            OffsetValue::Auto => "auto".to_string(),
            OffsetValue::Pixels(px) => format!("{}px", px),
            OffsetValue::Percentage(pct) => format!("{}%", pct),
        }
    }
}

/// 定位配置
#[derive(Debug, Clone)]
pub struct PositionConfig {
    /// 定位类型
    pub position: PositionType,
    /// top 偏移
    pub top: OffsetValue,
    /// right 偏移
    pub right: OffsetValue,
    /// bottom 偏移
    pub bottom: OffsetValue,
    /// left 偏移
    pub left: OffsetValue,
    /// z-index 层级
    pub z_index: Option<i32>,
}

impl PositionConfig {
    /// 从 ComputedStyles 创建
    pub fn from_styles(styles: &ComputedStyles) -> Self {
        let position = styles
            .get("position")
            .map(|s| PositionType::from_css(s))
            .unwrap_or(PositionType::Static);

        let top = styles
            .get("top")
            .map(|s| OffsetValue::from_css(s))
            .unwrap_or(OffsetValue::Auto);

        let right = styles
            .get("right")
            .map(|s| OffsetValue::from_css(s))
            .unwrap_or(OffsetValue::Auto);

        let bottom = styles
            .get("bottom")
            .map(|s| OffsetValue::from_css(s))
            .unwrap_or(OffsetValue::Auto);

        let left = styles
            .get("left")
            .map(|s| OffsetValue::from_css(s))
            .unwrap_or(OffsetValue::Auto);

        let z_index = styles
            .get("z-index")
            .and_then(|s| s.parse::<i32>().ok());

        Self {
            position,
            top,
            right,
            bottom,
            left,
            z_index,
        }
    }

    /// 转换为 CSS 字符串
    pub fn to_css(&self) -> String {
        let mut parts = vec![format!("position: {}", self.position.to_css())];
        
        if self.top != OffsetValue::Auto {
            parts.push(format!("top: {}", self.top.to_css()));
        }
        if self.right != OffsetValue::Auto {
            parts.push(format!("right: {}", self.right.to_css()));
        }
        if self.bottom != OffsetValue::Auto {
            parts.push(format!("bottom: {}", self.bottom.to_css()));
        }
        if self.left != OffsetValue::Auto {
            parts.push(format!("left: {}", self.left.to_css()));
        }
        if let Some(z) = self.z_index {
            parts.push(format!("z-index: {}", z));
        }

        parts.join("; ")
    }
}

/// 绝对定位布局结果
#[derive(Debug, Clone)]
pub struct AbsoluteLayout {
    /// x 坐标（相对于包含块）
    pub x: f32,
    /// y 坐标（相对于包含块）
    pub y: f32,
    /// z-index 层级
    pub z_index: i32,
}

/// 计算绝对定位
/// 
/// # 参数
/// 
/// - `node`: 当前节点
/// - `containing_block_width`: 包含块宽度
/// - `containing_block_height`: 包含块高度
/// - `config`: 定位配置
/// 
/// # 返回
/// 
/// 返回绝对定位的坐标
pub fn compute_absolute_position(
    containing_block_width: f32,
    containing_block_height: f32,
    config: &PositionConfig,
) -> AbsoluteLayout {
    let x = match config.left {
        OffsetValue::Pixels(px) => px,
        OffsetValue::Percentage(pct) => containing_block_width * pct / 100.0,
        OffsetValue::Auto => {
            // 如果 left 是 auto 且 right 不是 auto，则从 right 计算
            match config.right {
                OffsetValue::Pixels(px) => containing_block_width - px,
                OffsetValue::Percentage(pct) => containing_block_width * (100.0 - pct) / 100.0,
                OffsetValue::Auto => 0.0, // 默认
            }
        }
    };

    let y = match config.top {
        OffsetValue::Pixels(px) => px,
        OffsetValue::Percentage(pct) => containing_block_height * pct / 100.0,
        OffsetValue::Auto => {
            // 如果 top 是 auto 且 bottom 不是 auto，则从 bottom 计算
            match config.bottom {
                OffsetValue::Pixels(px) => containing_block_height - px,
                OffsetValue::Percentage(pct) => containing_block_height * (100.0 - pct) / 100.0,
                OffsetValue::Auto => 0.0, // 默认
            }
        }
    };

    AbsoluteLayout {
        x,
        y,
        z_index: config.z_index.unwrap_or(0),
    }
}

/// 浮动方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatType {
    /// 不浮动（默认）
    None,
    /// 左浮动
    Left,
    /// 右浮动
    Right,
}

impl FloatType {
    /// 从 CSS 字符串解析
    pub fn from_css(css: &str) -> Self {
        match css.trim() {
            "left" => FloatType::Left,
            "right" => FloatType::Right,
            _ => FloatType::None,
        }
    }

    /// 转换为 CSS 字符串
    pub fn to_css(&self) -> &'static str {
        match self {
            FloatType::None => "none",
            FloatType::Left => "left",
            FloatType::Right => "right",
        }
    }
}

/// 清除浮动
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearType {
    /// 不清除（默认）
    None,
    /// 清除左浮动
    Left,
    /// 清除右浮动
    Right,
    /// 清除所有浮动
    Both,
}

impl ClearType {
    /// 从 CSS 字符串解析
    pub fn from_css(css: &str) -> Self {
        match css.trim() {
            "left" => ClearType::Left,
            "right" => ClearType::Right,
            "both" => ClearType::Both,
            _ => ClearType::None,
        }
    }

    /// 转换为 CSS 字符串
    pub fn to_css(&self) -> &'static str {
        match self {
            ClearType::None => "none",
            ClearType::Left => "left",
            ClearType::Right => "right",
            ClearType::Both => "both",
        }
    }
}

/// 粘性定位状态
#[derive(Debug, Clone)]
pub struct StickyState {
    /// 是否处于粘性状态
    pub is_sticky: bool,
    /// 粘性偏移量
    pub offset: f32,
}

/// 计算粘性定位状态
/// 
/// # 参数
/// 
/// - `scroll_y`: 当前滚动位置
/// - `element_top`: 元素顶部位置
/// - `element_height`: 元素高度
/// - `container_height`: 容器高度
/// - `sticky_offset`: 粘性偏移（top 值）
/// 
/// # 返回
/// 
/// 返回粘性定位状态
pub fn compute_sticky_state(
    scroll_y: f32,
    element_top: f32,
    element_height: f32,
    container_height: f32,
    sticky_offset: f32,
) -> StickyState {
    // 元素应该在滚动到 sticky_offset 时粘住
    let should_stick = scroll_y >= (element_top - sticky_offset);
    
    // 检查是否还在容器内
    let element_bottom = element_top + element_height;
    let container_bottom = scroll_y + container_height;
    let still_in_container = element_bottom > scroll_y && element_top < container_bottom;

    StickyState {
        is_sticky: should_stick && still_in_container,
        offset: if should_stick && still_in_container {
            sticky_offset
        } else {
            0.0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_type_parsing() {
        assert_eq!(PositionType::from_css("static"), PositionType::Static);
        assert_eq!(PositionType::from_css("relative"), PositionType::Relative);
        assert_eq!(PositionType::from_css("absolute"), PositionType::Absolute);
        assert_eq!(PositionType::from_css("fixed"), PositionType::Fixed);
        assert_eq!(PositionType::from_css("sticky"), PositionType::Sticky);
    }

    #[test]
    fn test_offset_value_parsing() {
        assert_eq!(OffsetValue::from_css("auto"), OffsetValue::Auto);
        assert!(matches!(OffsetValue::from_css("10px"), OffsetValue::Pixels(10.0)));
        assert!(matches!(OffsetValue::from_css("50%"), OffsetValue::Percentage(50.0)));
    }

    #[test]
    fn test_absolute_position_pixels() {
        let config = PositionConfig {
            position: PositionType::Absolute,
            top: OffsetValue::Pixels(100.0),
            left: OffsetValue::Pixels(50.0),
            right: OffsetValue::Auto,
            bottom: OffsetValue::Auto,
            z_index: Some(10),
        };

        let result = compute_absolute_position(800.0, 600.0, &config);
        assert!((result.x - 50.0).abs() < 0.01);
        assert!((result.y - 100.0).abs() < 0.01);
        assert_eq!(result.z_index, 10);
    }

    #[test]
    fn test_absolute_position_percentage() {
        let config = PositionConfig {
            position: PositionType::Absolute,
            top: OffsetValue::Percentage(50.0),
            left: OffsetValue::Percentage(25.0),
            right: OffsetValue::Auto,
            bottom: OffsetValue::Auto,
            z_index: None,
        };

        let result = compute_absolute_position(800.0, 600.0, &config);
        assert!((result.x - 200.0).abs() < 0.01); // 800 * 25%
        assert!((result.y - 300.0).abs() < 0.01); // 600 * 50%
        assert_eq!(result.z_index, 0);
    }

    #[test]
    fn test_absolute_position_right_bottom() {
        let config = PositionConfig {
            position: PositionType::Absolute,
            top: OffsetValue::Auto,
            left: OffsetValue::Auto,
            right: OffsetValue::Pixels(20.0),
            bottom: OffsetValue::Pixels(30.0),
            z_index: None,
        };

        let result = compute_absolute_position(800.0, 600.0, &config);
        assert!((result.x - 780.0).abs() < 0.01); // 800 - 20
        assert!((result.y - 570.0).abs() < 0.01); // 600 - 30
    }

    #[test]
    fn test_float_type_parsing() {
        assert_eq!(FloatType::from_css("left"), FloatType::Left);
        assert_eq!(FloatType::from_css("right"), FloatType::Right);
        assert_eq!(FloatType::from_css("none"), FloatType::None);
    }

    #[test]
    fn test_clear_type_parsing() {
        assert_eq!(ClearType::from_css("left"), ClearType::Left);
        assert_eq!(ClearType::from_css("right"), ClearType::Right);
        assert_eq!(ClearType::from_css("both"), ClearType::Both);
        assert_eq!(ClearType::from_css("none"), ClearType::None);
    }

    #[test]
    fn test_sticky_state() {
        // 元素应该粘住
        let state = compute_sticky_state(
            200.0,  // scroll_y
            150.0,  // element_top
            100.0,  // element_height
            800.0,  // container_height
            50.0,   // sticky_offset
        );
        assert!(state.is_sticky);
        assert!((state.offset - 50.0).abs() < 0.01);

        // 元素不应该粘住（滚动位置不够）
        let state2 = compute_sticky_state(
            50.0,   // scroll_y
            150.0,  // element_top
            100.0,  // element_height
            800.0,  // container_height
            50.0,   // sticky_offset
        );
        assert!(!state2.is_sticky);
    }

    #[test]
    fn test_position_config_from_styles() {
        let mut styles = ComputedStyles::new();
        styles.set("position", "absolute");
        styles.set("top", "100px");
        styles.set("left", "50px");
        styles.set("z-index", "10");

        let config = PositionConfig::from_styles(&styles);
        assert_eq!(config.position, PositionType::Absolute);
        assert!(matches!(config.top, OffsetValue::Pixels(100.0)));
        assert!(matches!(config.left, OffsetValue::Pixels(50.0)));
        assert_eq!(config.z_index, Some(10));
    }

    #[test]
    fn test_is_out_of_flow() {
        assert!(!PositionType::Static.is_out_of_flow());
        assert!(!PositionType::Relative.is_out_of_flow());
        assert!(PositionType::Absolute.is_out_of_flow());
        assert!(PositionType::Fixed.is_out_of_flow());
        assert!(!PositionType::Sticky.is_out_of_flow());
    }
}
