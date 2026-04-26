//! VNode 到 GPU 渲染适配器
//!
//! 将虚拟 DOM 树转换为 GPU 绘制命令，实现高效的渲染管线。
//! 支持纯色背景、渐变背景、边框等 CSS 特性。

use iris_dom::vnode::VNode;
use iris_gpu::{BatchRenderer, DrawCommand};
use tracing::{debug, warn};

/// 渐变停止点
#[derive(Debug, Clone)]
struct GradientStop {
    position: f32,
    color: [f32; 4],
}

/// 渐变类型
#[derive(Debug, Clone)]
enum GradientType {
    Linear {
        horizontal: bool, // true = 水平, false = 垂直
    },
}

/// 背景类型
#[derive(Debug, Clone)]
enum Background {
    Solid([f32; 4]),
    Gradient {
        gradient_type: GradientType,
        stops: Vec<GradientStop>,
    },
}

/// 边框信息
#[derive(Debug, Clone)]
struct BorderInfo {
    width: (f32, f32, f32, f32), // 上, 右, 下, 左
    color: [f32; 4],
}

/// VNode 渲染器
///
/// 负责将虚拟 DOM 树转换为 GPU 绘制命令。
pub struct VNodeRenderer;

impl VNodeRenderer {
    /// 渲染虚拟 DOM 树到 GPU
    ///
    /// 遍历 VNode 树，为每个可见元素生成绘制命令。
    ///
    /// # 参数
    ///
    /// * `vnode` - 虚拟 DOM 根节点
    /// * `renderer` - GPU 批渲染器
    /// * `parent_x` - 父元素 X 偏移
    /// * `parent_y` - 父元素 Y 偏移
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let vnode = VNode::element("div");
    /// let mut batch_renderer = // ... 创建渲染器
    /// VNodeRenderer::render(&vnode, &mut batch_renderer, 0.0, 0.0)?;
    /// batch_renderer.flush()?;
    /// ```
    pub fn render(
        vnode: &VNode,
        renderer: &mut BatchRenderer,
        parent_x: f32,
        parent_y: f32,
    ) -> Result<(), String> {
        Self::render_recursive(vnode, renderer, parent_x, parent_y)
    }

    /// 递归渲染 VNode
    fn render_recursive(
        vnode: &VNode,
        renderer: &mut BatchRenderer,
        parent_x: f32,
        parent_y: f32,
    ) -> Result<(), String> {
        match vnode {
            VNode::Element {
                tag,
                layout,
                styles,
                children,
                ..
            } => {
                // 如果有布局信息，渲染元素
                if let Some(layout_box) = layout {
                    let box_model = &layout_box.box_model;
                    
                    // 计算绝对位置
                    let x = parent_x + layout_box.x;
                    let y = parent_y + layout_box.y;
                    let width = layout_box.width;
                    let height = layout_box.height;

                    // 跳过不可见元素
                    if width <= 0.0 || height <= 0.0 {
                        debug!(tag = tag, "Skipping zero-size element");
                    } else {
                        // 渲染背景（支持纯色和渐变）
                        Self::render_background(styles, x, y, width, height, renderer)?;
                        
                        // 渲染边框
                        Self::render_border(styles, x, y, width, height, renderer)?;
                    }
                }

                // 递归渲染子节点
                for child in children {
                    Self::render_recursive(child, renderer, parent_x, parent_y)?;
                }
            }
            VNode::Text { content } => {
                // TODO: 文本渲染需要字体系统支持
                debug!(text = content, "Text rendering requires font system");
            }
            VNode::Comment { .. } => {
                // 注释节点不渲染
            }
            VNode::Fragment { children } => {
                // Fragment 只是包装，递归渲染子节点
                for child in children {
                    Self::render_recursive(child, renderer, parent_x, parent_y)?;
                }
            }
        }

        Ok(())
    }

    /// 渲染背景（支持纯色和渐变）
    fn render_background(
        styles: &iris_layout::style::ComputedStyles,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        renderer: &mut BatchRenderer,
    ) -> Result<(), String> {
        // 尝试获取 background 或 background-color
        let bg_css = styles.get("background")
            .or_else(|| styles.get("background-color"));

        if let Some(bg_value) = bg_css {
            match Self::parse_background(bg_value) {
                Some(Background::Solid(color)) => {
                    if color[3] > 0.0 {
                        renderer.submit(DrawCommand::Rect {
                            x,
                            y,
                            width,
                            height,
                            color,
                        });
                    }
                }
                Some(Background::Gradient { gradient_type, stops }) => {
                    if stops.len() >= 2 {
                        match gradient_type {
                            GradientType::Linear { horizontal } => {
                                let start_color = stops[0].color;
                                let end_color = stops[stops.len() - 1].color;
                                
                                renderer.submit(DrawCommand::GradientRect {
                                    x,
                                    y,
                                    width,
                                    height,
                                    start_color,
                                    end_color,
                                    horizontal,
                                });
                            }
                        }
                    }
                }
                None => {}
            }
        }

        Ok(())
    }

    /// 解析 CSS 背景值
    fn parse_background(css: &str) -> Option<Background> {
        let css = css.trim();
        
        // 检查是否是渐变
        if css.starts_with("linear-gradient") {
            return Self::parse_linear_gradient(css);
        }
        
        // 否则尝试解析为纯色
        Self::parse_css_color(css).map(Background::Solid)
    }

    /// 解析线性渐变
    fn parse_linear_gradient(css: &str) -> Option<Background> {
        // 提取括号内容: linear-gradient(to right, red, blue)
        let start = css.find('(')? + 1;
        let end = css.rfind(')')?;
        let content = &css[start..end];
        
        // 分割参数
        let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
        if parts.len() < 2 {
            return None;
        }
        
        // 解析方向（第一个参数）
        let (horizontal, color_start) = Self::parse_gradient_direction(parts[0]);
        
        // 解析颜色
        let mut stops = Vec::new();
        let color_parts = &parts[color_start..];
        
        for (i, color_css) in color_parts.iter().enumerate() {
            if let Some(color) = Self::parse_css_color(color_css) {
                let position = if color_parts.len() <= 1 {
                    0.0
                } else {
                    i as f32 / (color_parts.len() - 1) as f32
                };
                stops.push(GradientStop { position, color });
            }
        }
        
        if stops.len() < 2 {
            return None;
        }
        
        Some(Background::Gradient {
            gradient_type: GradientType::Linear { horizontal },
            stops,
        })
    }

    /// 解析渐变方向
    fn parse_gradient_direction(dir: &str) -> (bool, usize) {
        match dir.trim() {
            "to right" | "to left" => (true, 1),   // 水平渐变
            "to bottom" | "to top" => (false, 1),  // 垂直渐变
            _ => (false, 0), // 默认垂直，第一个参数是颜色
        }
    }

    /// 渲染边框
    fn render_border(
        styles: &iris_layout::style::ComputedStyles,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        renderer: &mut BatchRenderer,
    ) -> Result<(), String> {
        if let Some(border) = Self::parse_border(styles) {
            renderer.submit(DrawCommand::Border {
                x,
                y,
                width,
                height,
                border_width: border.width,
                border_color: border.color,
            });
        }
        
        Ok(())
    }

    /// 解析 CSS border 属性
    fn parse_border(styles: &iris_layout::style::ComputedStyles) -> Option<BorderInfo> {
        // 获取 border-width
        let width_str = styles.get("border-width")?;
        let width = Self::parse_border_width(width_str);
        
        // 检查是否有任何边框宽度 > 0
        if width.0 == 0.0 && width.1 == 0.0 && width.2 == 0.0 && width.3 == 0.0 {
            return None;
        }
        
        // 获取 border-color
        let color_str = styles.get("border-color")
            .or_else(|| styles.get("border")); //  fallback to border shorthand
        
        let color = if let Some(color_css) = color_str {
            Self::parse_css_color(color_css).unwrap_or([0.0, 0.0, 0.0, 1.0]) // 默认黑色
        } else {
            [0.0, 0.0, 0.0, 1.0] // 默认黑色
        };
        
        Some(BorderInfo { width, color })
    }

    /// 解析边框宽度 (支持 "1px 2px 1px 2px" 或 "2px")
    fn parse_border_width(css: &str) -> (f32, f32, f32, f32) {
        let parts: Vec<&str> = css.split_whitespace().collect();
        
        match parts.len() {
            1 => {
                // 1 个值: 所有边相同
                let val = Self::parse_css_unit(parts[0]);
                (val, val, val, val)
            }
            2 => {
                // 2 个值: 上下, 左右
                let top_bottom = Self::parse_css_unit(parts[0]);
                let left_right = Self::parse_css_unit(parts[1]);
                (top_bottom, left_right, top_bottom, left_right)
            }
            3 => {
                // 3 个值: 上, 左右, 下
                let top = Self::parse_css_unit(parts[0]);
                let left_right = Self::parse_css_unit(parts[1]);
                let bottom = Self::parse_css_unit(parts[2]);
                (top, left_right, bottom, left_right)
            }
            4 => {
                // 4 个值: 上, 右, 下, 左
                let top = Self::parse_css_unit(parts[0]);
                let right = Self::parse_css_unit(parts[1]);
                let bottom = Self::parse_css_unit(parts[2]);
                let left = Self::parse_css_unit(parts[3]);
                (top, right, bottom, left)
            }
            _ => (0.0, 0.0, 0.0, 0.0),
        }
    }

    /// 解析 CSS 单位值 (如 "2px" -> 2.0)
    fn parse_css_unit(css: &str) -> f32 {
        // 移除 "px" 后缀并解析为 f32
        let num_str = css.trim_end_matches("px").trim();
        num_str.parse::<f32>().unwrap_or(0.0)
    }
    fn parse_background_color(styles: &iris_layout::style::ComputedStyles) -> [f32; 4] {
        // 尝试解析 background-color 属性
        if let Some(color_str) = styles.get("background-color") {
            Self::parse_css_color(color_str).unwrap_or([0.0, 0.0, 0.0, 0.0])
        } else {
            [0.0, 0.0, 0.0, 0.0] // 透明
        }
    }

    /// 解析 CSS 颜色字符串
    fn parse_css_color(color: &str) -> Option<[f32; 4]> {
        // 简化实现：支持 rgba(r, g, b, a) 格式
        if color.starts_with("rgba(") {
            let parts: Vec<&str> = color[5..color.len() - 1].split(',').collect();
            if parts.len() == 4 {
                let r: f32 = parts[0].trim().parse().unwrap_or(0.0) / 255.0;
                let g: f32 = parts[1].trim().parse().unwrap_or(0.0) / 255.0;
                let b: f32 = parts[2].trim().parse().unwrap_or(0.0) / 255.0;
                let a: f32 = parts[3].trim().parse().unwrap_or(1.0);
                return Some([r, g, b, a]);
            }
        }
        
        // 支持颜色名
        match color.to_lowercase().as_str() {
            "red" => Some([1.0, 0.0, 0.0, 1.0]),
            "blue" => Some([0.0, 0.0, 1.0, 1.0]),
            "green" => Some([0.0, 0.502, 0.0, 1.0]),
            "yellow" => Some([1.0, 1.0, 0.0, 1.0]),
            "white" => Some([1.0, 1.0, 1.0, 1.0]),
            "black" => Some([0.0, 0.0, 0.0, 1.0]),
            "transparent" => Some([0.0, 0.0, 0.0, 0.0]),
            _ => None,
        }
    }

    /// 获取元素的可见性
    fn is_visible(styles: &iris_layout::style::ComputedStyles) -> bool {
        // 检查 display 属性
        if let Some(display) = styles.get("display") {
            if display == "none" {
                return false;
            }
        }
        
        // 检查 visibility 属性
        if let Some(visibility) = styles.get("visibility") {
            if visibility == "hidden" {
                return false;
            }
        }

        true
    }
}

/// 渲染统计信息
#[derive(Debug, Default)]
pub struct RenderStats {
    /// 绘制的元素数量
    pub elements_drawn: usize,
    /// 跳过的元素数量
    pub elements_skipped: usize,
    /// 文本节点数量
    pub text_nodes: usize,
    /// 总节点数
    pub total_nodes: usize,
}

impl RenderStats {
    /// 从 VNode 树收集统计信息
    pub fn collect(vnode: &VNode) -> Self {
        let mut stats = Self::default();
        Self::collect_recursive(vnode, &mut stats);
        stats
    }

    fn collect_recursive(vnode: &VNode, stats: &mut RenderStats) {
        stats.total_nodes += 1;

        match vnode {
            VNode::Element {
                layout, children, ..
            } => {
                if layout.is_some() {
                    stats.elements_drawn += 1;
                } else {
                    stats.elements_skipped += 1;
                }
                
                for child in children {
                    Self::collect_recursive(child, stats);
                }
            }
            VNode::Text { .. } => {
                stats.text_nodes += 1;
            }
            VNode::Comment { .. } => {
                // 注释节点不计入
            }
            VNode::Fragment { children } => {
                // Fragment 本身不计入 total_nodes
                stats.total_nodes -= 1;
                for child in children {
                    Self::collect_recursive(child, stats);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iris_dom::vnode::VNode;
    use iris_layout::layout::LayoutBox;
    use iris_layout::style::ComputedStyles;

    #[test]
    fn test_collect_stats() {
        let mut vnode = VNode::element("div");
        vnode.append_child(VNode::text("Hello"));
        vnode.append_child(VNode::element("span"));

        let stats = RenderStats::collect(&vnode);
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.text_nodes, 1);
    }

    #[test]
    fn test_fragment_rendering() {
        let fragment = VNode::fragment(vec![
            VNode::element("div"),
            VNode::element("span"),
        ]);

        let stats = RenderStats::collect(&fragment);
        // Fragment 本身不计入，但子节点计入
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.elements_drawn, 0); // 没有布局信息
        assert_eq!(stats.elements_skipped, 2);
    }

    #[test]
    fn test_parse_css_color_rgba() {
        let color = VNodeRenderer::parse_css_color("rgba(255, 128, 64, 0.5)");
        assert!(color.is_some());
        let color = color.unwrap();
        assert!((color[0] - 1.0).abs() < 0.01); // 255/255 = 1.0
        assert!((color[1] - 0.502).abs() < 0.01); // 128/255 ≈ 0.502
        assert!((color[2] - 0.251).abs() < 0.01); // 64/255 ≈ 0.251
        assert!((color[3] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_css_color_invalid() {
        let color = VNodeRenderer::parse_css_color("invalid");
        assert!(color.is_none());
    }

    #[test]
    fn test_parse_css_color_partial() {
        let color = VNodeRenderer::parse_css_color("rgba(255, 0, 0");
        assert!(color.is_none()); // 不完整，返回 None
    }

    #[test]
    fn test_element_with_layout() {
        let mut vnode = VNode::element("div");
        
        // 设置样式和布局信息（模拟）
        if let VNode::Element { ref mut styles, ref mut layout, .. } = vnode {
            styles.set("background-color", "rgba(255, 0, 0, 1)");
            *layout = Some(LayoutBox::with_position(0.0, 0.0, 100.0, 50.0));
        }

        let stats = RenderStats::collect(&vnode);
        assert_eq!(stats.elements_drawn, 1);
        assert_eq!(stats.elements_skipped, 0);
    }

    #[test]
    fn test_zero_size_element() {
        let mut vnode = VNode::element("div");
        
        if let VNode::Element { ref mut layout, .. } = vnode {
            *layout = Some(LayoutBox::with_position(0.0, 0.0, 0.0, 0.0));
        }

        let stats = RenderStats::collect(&vnode);
        assert_eq!(stats.elements_drawn, 1); // 有布局信息就算 drawn
    }

    #[test]
    fn test_nested_elements() {
        let mut parent = VNode::element("div");
        if let VNode::Element { ref mut layout, .. } = parent {
            *layout = Some(LayoutBox::with_position(0.0, 0.0, 200.0, 100.0));
        }

        let mut child = VNode::element("span");
        if let VNode::Element { ref mut layout, .. } = child {
            *layout = Some(LayoutBox::with_position(10.0, 10.0, 50.0, 30.0));
        }

        parent.append_child(child);

        let stats = RenderStats::collect(&parent);
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.elements_drawn, 2);
    }

    #[test]
    fn test_comment_not_counted() {
        let mut vnode = VNode::element("div");
        vnode.append_child(VNode::comment("This is a comment"));

        let stats = RenderStats::collect(&vnode);
        // div + comment (注释计入 total_nodes 但不计入其他统计)
        assert_eq!(stats.total_nodes, 2);
    }

    #[test]
    fn test_mixed_content() {
        let mut div = VNode::element("div");
        div.append_child(VNode::text("Hello"));
        div.append_child(VNode::element("span"));
        div.append_child(VNode::comment("comment"));

        let stats = RenderStats::collect(&div);
        // div + text + span + comment = 4
        assert_eq!(stats.total_nodes, 4);
        assert_eq!(stats.text_nodes, 1);
        assert_eq!(stats.elements_skipped, 2); // div 和 span 都没有布局
    }

    #[test]
    fn test_deep_nesting() {
        fn create_nested(depth: u32) -> VNode {
            if depth == 0 {
                VNode::element("leaf")
            } else {
                let mut parent = VNode::element("parent");
                parent.append_child(create_nested(depth - 1));
                parent
            }
        }

        let vnode = create_nested(5);
        let stats = RenderStats::collect(&vnode);
        assert_eq!(stats.total_nodes, 6); // 5 parents + 1 leaf
        assert_eq!(stats.elements_skipped, 6);
    }

    #[test]
    fn test_is_visible_display_none() {
        let mut styles = ComputedStyles::new();
        styles.set("display", "none");
        assert!(!VNodeRenderer::is_visible(&styles));
    }

    #[test]
    fn test_is_visible_hidden() {
        let mut styles = ComputedStyles::new();
        styles.set("visibility", "hidden");
        assert!(!VNodeRenderer::is_visible(&styles));
    }

    #[test]
    fn test_is_visible_normal() {
        let styles = ComputedStyles::new();
        assert!(VNodeRenderer::is_visible(&styles));
    }

    #[test]
    fn test_parse_linear_gradient_horizontal() {
        let bg = VNodeRenderer::parse_background("linear-gradient(to right, red, blue)");
        assert!(bg.is_some());
        match bg.unwrap() {
            Background::Gradient { gradient_type, stops } => {
                match gradient_type {
                    GradientType::Linear { horizontal } => {
                        assert!(horizontal); // to right = 水平
                    }
                }
                assert_eq!(stops.len(), 2);
            }
            _ => panic!("Expected gradient"),
        }
    }

    #[test]
    fn test_parse_linear_gradient_vertical() {
        let bg = VNodeRenderer::parse_background("linear-gradient(to bottom, red, blue)");
        assert!(bg.is_some());
        match bg.unwrap() {
            Background::Gradient { gradient_type, stops } => {
                match gradient_type {
                    GradientType::Linear { horizontal } => {
                        assert!(!horizontal); // to bottom = 垂直
                    }
                }
                assert_eq!(stops.len(), 2);
            }
            _ => panic!("Expected gradient"),
        }
    }

    #[test]
    fn test_parse_solid_color() {
        let bg = VNodeRenderer::parse_background("rgba(255, 0, 0, 1)");
        assert!(bg.is_some());
        match bg.unwrap() {
            Background::Solid(color) => {
                assert!((color[0] - 1.0).abs() < 0.01);
            }
            _ => panic!("Expected solid color"),
        }
    }

    #[test]
    fn test_parse_invalid_gradient() {
        let bg = VNodeRenderer::parse_background("linear-gradient(red)");
        assert!(bg.is_none()); // 至少需要2个颜色
    }

    #[test]
    fn test_parse_border_single() {
        let mut styles = ComputedStyles::new();
        styles.set("border-width", "2px");
        styles.set("border-color", "red");
        
        let border = VNodeRenderer::parse_border(&styles);
        assert!(border.is_some());
        let border = border.unwrap();
        assert_eq!(border.width, (2.0, 2.0, 2.0, 2.0));
        assert!((border.color[0] - 1.0).abs() < 0.01); // red
    }

    #[test]
    fn test_parse_border_four_values() {
        let mut styles = ComputedStyles::new();
        styles.set("border-width", "1px 2px 3px 4px");
        styles.set("border-color", "blue");
        
        let border = VNodeRenderer::parse_border(&styles);
        assert!(border.is_some());
        let border = border.unwrap();
        assert_eq!(border.width, (1.0, 2.0, 3.0, 4.0));
        assert!((border.color[2] - 1.0).abs() < 0.01); // blue
    }

    #[test]
    fn test_parse_border_two_values() {
        let mut styles = ComputedStyles::new();
        styles.set("border-width", "5px 10px");
        
        let border = VNodeRenderer::parse_border(&styles);
        assert!(border.is_some());
        let border = border.unwrap();
        assert_eq!(border.width, (5.0, 10.0, 5.0, 10.0));
    }

    #[test]
    fn test_parse_border_three_values() {
        let mut styles = ComputedStyles::new();
        styles.set("border-width", "1px 2px 3px");
        
        let border = VNodeRenderer::parse_border(&styles);
        assert!(border.is_some());
        let border = border.unwrap();
        assert_eq!(border.width, (1.0, 2.0, 3.0, 2.0));
    }

    #[test]
    fn test_parse_border_no_width() {
        let styles = ComputedStyles::new();
        let border = VNodeRenderer::parse_border(&styles);
        assert!(border.is_none());
    }

    #[test]
    fn test_parse_border_zero_width() {
        let mut styles = ComputedStyles::new();
        styles.set("border-width", "0px");
        
        let border = VNodeRenderer::parse_border(&styles);
        assert!(border.is_none());
    }
}
