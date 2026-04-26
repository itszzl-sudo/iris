//! VNode 到 GPU 渲染适配器
//!
//! 将虚拟 DOM 树转换为 GPU 绘制命令，实现高效的渲染管线。

use iris_dom::vnode::VNode;
use iris_gpu::{BatchRenderer, DrawCommand};
use tracing::{debug, warn};

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
                        // 获取背景颜色
                        let bg_color = Self::parse_background_color(styles);

                        // 只在有背景色时绘制
                        if bg_color[3] > 0.0 {
                            let command = DrawCommand::Rect {
                                x,
                                y,
                                width,
                                height,
                                color: bg_color,
                            };
                            
                            renderer.submit(command);
                        }
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

    /// 解析背景颜色
    fn parse_background_color(styles: &iris_layout::style::ComputedStyles) -> [f32; 4] {
        // 尝试解析 background-color 属性
        if let Some(color_str) = styles.get("background-color") {
            Self::parse_css_color(color_str)
        } else {
            [0.0, 0.0, 0.0, 0.0] // 透明
        }
    }

    /// 解析 CSS 颜色字符串
    fn parse_css_color(color: &str) -> [f32; 4] {
        // 简化实现：支持 rgba(r, g, b, a) 格式
        if color.starts_with("rgba(") {
            let parts: Vec<&str> = color[5..color.len() - 1].split(',').collect();
            if parts.len() == 4 {
                let r: f32 = parts[0].trim().parse().unwrap_or(0.0) / 255.0;
                let g: f32 = parts[1].trim().parse().unwrap_or(0.0) / 255.0;
                let b: f32 = parts[2].trim().parse().unwrap_or(0.0) / 255.0;
                let a: f32 = parts[3].trim().parse().unwrap_or(1.0);
                return [r, g, b, a];
            }
        }
        
        // 默认透明
        [0.0, 0.0, 0.0, 0.0]
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
        assert!((color[0] - 1.0).abs() < 0.01); // 255/255 = 1.0
        assert!((color[1] - 0.502).abs() < 0.01); // 128/255 ≈ 0.502
        assert!((color[2] - 0.251).abs() < 0.01); // 64/255 ≈ 0.251
        assert!((color[3] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_css_color_invalid() {
        let color = VNodeRenderer::parse_css_color("invalid");
        assert_eq!(color, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_parse_css_color_partial() {
        let color = VNodeRenderer::parse_css_color("rgba(255, 0, 0");
        assert_eq!(color, [0.0, 0.0, 0.0, 0.0]); // 不完整，返回透明
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
}
