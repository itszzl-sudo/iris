//! CSS Float 布局系统
//!
//! 实现 CSS Float 和 Clear 属性，支持：
//! - Float: left / right / none
//! - Clear: left / right / both / none
//! - 浮动元素的流式布局
//! - 清除浮动后的布局恢复
//! - 包含块高度计算（清除浮动）

use crate::style::ComputedStyles;
use crate::positioning::{FloatType, ClearType};

/// 浮动元素信息
#[derive(Debug, Clone)]
pub struct FloatedElement {
    /// 元素 ID
    pub element_id: u64,
    /// 浮动方向
    pub float_type: FloatType,
    /// 元素宽度
    pub width: f32,
    /// 元素高度
    pub height: f32,
    /// x 坐标（相对于容器）
    pub x: f32,
    /// y 坐标（相对于容器）
    pub y: f32,
}

/// 浮动行（用于管理同一行的浮动元素）
#[derive(Debug, Clone)]
pub struct FloatLine {
    /// 行中所有浮动元素
    pub elements: Vec<FloatedElement>,
    /// 行的最大高度
    pub max_height: f32,
    /// 行的 y 坐标
    pub y: f32,
    /// 已使用的左侧空间
    pub used_left: f32,
    /// 已使用的右侧空间
    pub used_right: f32,
}

impl FloatLine {
    /// 创建新的浮动行
    pub fn new(y: f32) -> Self {
        Self {
            elements: Vec::new(),
            max_height: 0.0,
            y,
            used_left: 0.0,
            used_right: 0.0,
        }
    }

    /// 检查是否可以添加元素
    pub fn can_fit(&self, width: f32, container_width: f32, float_type: FloatType) -> bool {
        match float_type {
            FloatType::Left => self.used_left + width <= container_width,
            FloatType::Right => self.used_right + width <= container_width,
            _ => false,
        }
    }

    /// 添加浮动元素
    pub fn add_element(&mut self, element: FloatedElement, _container_width: f32) {
        match element.float_type {
            FloatType::Left => {
                self.used_left += element.width;
            }
            FloatType::Right => {
                self.used_right += element.width;
            }
            _ => {}
        }
        
        self.max_height = self.max_height.max(element.height);
        self.elements.push(element);
    }
}

/// Float 布局上下文
#[derive(Debug, Clone)]
pub struct FloatContext {
    /// 左侧浮动行列表
    pub left_lines: Vec<FloatLine>,
    /// 右侧浮动行列表
    pub right_lines: Vec<FloatLine>,
    /// 容器宽度
    pub container_width: f32,
    /// 当前 y 坐标
    pub current_y: f32,
}

impl FloatContext {
    /// 创建新的浮动上下文
    pub fn new(container_width: f32) -> Self {
        Self {
            left_lines: vec![FloatLine::new(0.0)],
            right_lines: vec![FloatLine::new(0.0)],
            container_width,
            current_y: 0.0,
        }
    }

    /// 获取当前浮动高度
    pub fn get_float_height(&self) -> f32 {
        let left_max: f32 = self.left_lines
            .iter()
            .map(|line| line.y + line.max_height)
            .fold(0.0, f32::max);
        
        let right_max: f32 = self.right_lines
            .iter()
            .map(|line| line.y + line.max_height)
            .fold(0.0, f32::max);
        
        left_max.max(right_max)
    }

    /// 计算清除浮动后的 y 坐标
    pub fn get_cleared_y(&self, clear_type: ClearType) -> f32 {
        match clear_type {
            ClearType::Left => {
                self.left_lines
                    .iter()
                    .map(|line| line.y + line.max_height)
                    .fold(0.0, f32::max)
            }
            ClearType::Right => {
                self.right_lines
                    .iter()
                    .map(|line| line.y + line.max_height)
                    .fold(0.0, f32::max)
            }
            ClearType::Both => {
                self.get_float_height()
            }
            ClearType::None => self.current_y,
        }
    }

    /// 添加浮动元素
    pub fn add_floated_element(&mut self, element: FloatedElement) {
        match element.float_type {
            FloatType::Left => {
                // 找到合适的行
                let line = self.left_lines
                    .iter_mut()
                    .find(|line| line.can_fit(element.width, self.container_width, FloatType::Left));
                
                if let Some(line) = line {
                    line.add_element(element, self.container_width);
                } else {
                    // 创建新行
                    let mut new_line = FloatLine::new(self.get_float_height());
                    new_line.add_element(element, self.container_width);
                    self.left_lines.push(new_line);
                }
            }
            FloatType::Right => {
                let line = self.right_lines
                    .iter_mut()
                    .find(|line| line.can_fit(element.width, self.container_width, FloatType::Right));
                
                if let Some(line) = line {
                    line.add_element(element, self.container_width);
                } else {
                    let mut new_line = FloatLine::new(self.get_float_height());
                    new_line.add_element(element, self.container_width);
                    self.right_lines.push(new_line);
                }
            }
            _ => {}
        }
    }
}

/// Float 配置
#[derive(Debug, Clone)]
pub struct FloatConfig {
    /// 浮动类型
    pub float: FloatType,
    /// 清除类型
    pub clear: ClearType,
}

impl FloatConfig {
    /// 从 ComputedStyles 创建
    pub fn from_styles(styles: &ComputedStyles) -> Self {
        let float = styles
            .get("float")
            .map(|s| FloatType::from_css(s))
            .unwrap_or(FloatType::None);

        let clear = styles
            .get("clear")
            .map(|s| ClearType::from_css(s))
            .unwrap_or(ClearType::None);

        Self { float, clear }
    }

    /// 是否有浮动
    pub fn is_floated(&self) -> bool {
        self.float != FloatType::None
    }

    /// 是否需要清除浮动
    pub fn needs_clear(&self) -> bool {
        self.clear != ClearType::None
    }
}

/// Float 布局结果
#[derive(Debug, Clone)]
pub struct FloatLayout {
    /// 浮动元素列表
    pub floated_elements: Vec<FloatedElement>,
    /// 内容区域的可用宽度
    pub available_width: f32,
    /// 浮动区域的总高度
    pub float_height: f32,
}

/// 计算 Float 布局
/// 
/// # 参数
/// 
/// - `float_configs`: 所有子元素的浮动配置（按文档顺序）
/// - `element_sizes`: 对应的元素尺寸 [(width, height), ...]
/// - `container_width`: 容器宽度
/// 
/// # 返回
/// 
/// 返回 Float 布局结果
pub fn compute_float_layout(
    float_configs: &[(FloatConfig, u64)],
    element_sizes: &[(f32, f32)],
    container_width: f32,
) -> FloatLayout {
    let mut context = FloatContext::new(container_width);
    let mut floated_elements = Vec::new();
    let mut available_width = container_width;

    for (i, (config, element_id)) in float_configs.iter().enumerate() {
        if config.is_floated() && i < element_sizes.len() {
            let (width, height) = element_sizes[i];
            
            // 如果需要清除浮动，更新当前 y
            let y = if config.needs_clear() {
                context.get_cleared_y(config.clear)
            } else {
                context.current_y
            };

            // 计算 x 坐标
            let x = match config.float {
                FloatType::Left => {
                    // 找到当前行的左侧位置
                    if let Some(line) = context.left_lines.last() {
                        line.used_left
                    } else {
                        0.0
                    }
                }
                FloatType::Right => {
                    // 找到当前行的右侧位置
                    if let Some(line) = context.right_lines.last() {
                        container_width - line.used_right - width
                    } else {
                        container_width - width
                    }
                }
                _ => 0.0,
            };

            let element = FloatedElement {
                element_id: *element_id,
                float_type: config.float,
                width,
                height,
                x,
                y,
            };

            context.add_floated_element(element.clone());
            floated_elements.push(element);

            // 更新可用宽度（简化实现）
            let left_floats: f32 = context.left_lines
                .iter()
                .flat_map(|line| &line.elements)
                .map(|e| e.width)
                .sum();
            
            let right_floats: f32 = context.right_lines
                .iter()
                .flat_map(|line| &line.elements)
                .map(|e| e.width)
                .sum();

            available_width = (container_width - left_floats - right_floats).max(0.0);
        }
    }

    let float_height = context.get_float_height();

    FloatLayout {
        floated_elements,
        available_width,
        float_height,
    }
}

/// 清除浮动的辅助函数
/// 
/// # 用途
/// 
/// 用于容器需要包含所有浮动元素时计算高度
/// 
/// # 参数
/// 
/// - `float_height`: 浮动区域的总高度
/// - `content_height`: 内容区域的总高度
/// 
/// # 返回
/// 
/// 返回容器的实际高度（取最大值）
pub fn clear_floats(float_height: f32, content_height: f32) -> f32 {
    float_height.max(content_height)
}

/// 计算包含浮动元素的容器宽度
/// 
/// # 参数
/// 
/// - `container_width`: 容器总宽度
/// - `left_floats_width`: 左侧浮动元素总宽度
/// - `right_floats_width`: 右侧浮动元素总宽度
/// 
/// # 返回
/// 
/// 返回内容区域的可用宽度
pub fn calculate_available_width(
    container_width: f32,
    left_floats_width: f32,
    right_floats_width: f32,
) -> f32 {
    (container_width - left_floats_width - right_floats_width).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_config_from_styles() {
        let mut styles = ComputedStyles::new();
        styles.set("float", "left");
        styles.set("clear", "both");

        let config = FloatConfig::from_styles(&styles);
        assert_eq!(config.float, FloatType::Left);
        assert_eq!(config.clear, ClearType::Both);
        assert!(config.is_floated());
        assert!(config.needs_clear());
    }

    #[test]
    fn test_float_line_fit() {
        let mut line = FloatLine::new(0.0);
        
        assert!(line.can_fit(100.0, 800.0, FloatType::Left));
        assert!(!line.can_fit(900.0, 800.0, FloatType::Left));
        
        line.used_left = 500.0;
        assert!(line.can_fit(300.0, 800.0, FloatType::Left));
        assert!(!line.can_fit(400.0, 800.0, FloatType::Left));
    }

    #[test]
    fn test_float_line_add_element() {
        let mut line = FloatLine::new(0.0);
        
        let element = FloatedElement {
            element_id: 1,
            float_type: FloatType::Left,
            width: 100.0,
            height: 50.0,
            x: 0.0,
            y: 0.0,
        };
        
        line.add_element(element, 800.0);
        
        assert_eq!(line.elements.len(), 1);
        assert!((line.used_left - 100.0).abs() < 0.01);
        assert!((line.max_height - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_float_context_add_elements() {
        let mut context = FloatContext::new(800.0);
        
        let elem1 = FloatedElement {
            element_id: 1,
            float_type: FloatType::Left,
            width: 200.0,
            height: 100.0,
            x: 0.0,
            y: 0.0,
        };
        
        let elem2 = FloatedElement {
            element_id: 2,
            float_type: FloatType::Left,
            width: 300.0,
            height: 80.0,
            x: 200.0,
            y: 0.0,
        };
        
        context.add_floated_element(elem1);
        context.add_floated_element(elem2);
        
        assert_eq!(context.left_lines.len(), 1);
        assert!((context.left_lines[0].used_left - 500.0).abs() < 0.01);
        assert!((context.left_lines[0].max_height - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_float_context_new_line() {
        let mut context = FloatContext::new(400.0);
        
        // 添加一个占满宽度的元素
        let elem1 = FloatedElement {
            element_id: 1,
            float_type: FloatType::Left,
            width: 400.0,
            height: 100.0,
            x: 0.0,
            y: 0.0,
        };
        context.add_floated_element(elem1);
        
        // 添加第二个元素，应该创建新行
        let elem2 = FloatedElement {
            element_id: 2,
            float_type: FloatType::Left,
            width: 200.0,
            height: 80.0,
            x: 0.0,
            y: 100.0,
        };
        context.add_floated_element(elem2);
        
        assert_eq!(context.left_lines.len(), 2);
        assert!((context.left_lines[1].y - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_clear_floats() {
        // 浮动高度大于内容高度
        let height = clear_floats(200.0, 100.0);
        assert!((height - 200.0).abs() < 0.01);
        
        // 内容高度大于浮动高度
        let height = clear_floats(100.0, 200.0);
        assert!((height - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_available_width() {
        let width = calculate_available_width(800.0, 200.0, 100.0);
        assert!((width - 500.0).abs() < 0.01);
        
        // 浮动宽度超过容器宽度
        let width = calculate_available_width(400.0, 300.0, 200.0);
        assert!((width - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_float_layout_simple() {
        let configs = vec![
            (
                FloatConfig {
                    float: FloatType::Left,
                    clear: ClearType::None,
                },
                1u64,
            ),
            (
                FloatConfig {
                    float: FloatType::Left,
                    clear: ClearType::None,
                },
                2u64,
            ),
        ];
        
        let sizes = vec![
            (100.0, 50.0),
            (150.0, 80.0),
        ];
        
        let layout = compute_float_layout(&configs, &sizes, 800.0);
        
        assert_eq!(layout.floated_elements.len(), 2);
        assert!(layout.available_width < 800.0);
        assert!(layout.float_height > 0.0);
    }

    #[test]
    fn test_float_context_cleared_y() {
        let mut context = FloatContext::new(800.0);
        
        // 添加一些浮动元素
        let elem1 = FloatedElement {
            element_id: 1,
            float_type: FloatType::Left,
            width: 200.0,
            height: 100.0,
            x: 0.0,
            y: 0.0,
        };
        context.add_floated_element(elem1);
        
        // 清除左侧浮动
        let cleared_y = context.get_cleared_y(ClearType::Left);
        assert!((cleared_y - 100.0).abs() < 0.01);
        
        // 清除两侧浮动
        let cleared_y = context.get_cleared_y(ClearType::Both);
        assert!((cleared_y - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_float_context_both_sides() {
        let mut context = FloatContext::new(800.0);
        
        // 左侧浮动
        let left_elem = FloatedElement {
            element_id: 1,
            float_type: FloatType::Left,
            width: 200.0,
            height: 100.0,
            x: 0.0,
            y: 0.0,
        };
        context.add_floated_element(left_elem);
        
        // 右侧浮动
        let right_elem = FloatedElement {
            element_id: 2,
            float_type: FloatType::Right,
            width: 300.0,
            height: 80.0,
            x: 500.0,
            y: 0.0,
        };
        context.add_floated_element(right_elem);
        
        assert_eq!(context.left_lines.len(), 1);
        assert_eq!(context.right_lines.len(), 1);
        assert!((context.left_lines[0].used_left - 200.0).abs() < 0.01);
        assert!((context.right_lines[0].used_right - 300.0).abs() < 0.01);
    }
}
