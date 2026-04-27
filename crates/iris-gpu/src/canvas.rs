//! Canvas 2D API 实现
//!
//! 提供 HTML5 Canvas 2D 上下文的部分实现，
//! 将 Canvas 绘图命令转换为 GPU 可执行的绘制命令。

use crate::batch_renderer::DrawCommand;

/// Canvas 2D 渲染上下文
#[derive(Debug, Clone)]
pub struct Canvas2DContext {
    /// 画布宽度
    pub width: u32,
    /// 画布高度
    pub height: u32,
    /// 当前填充颜色
    pub fill_style: String,
    /// 当前描边颜色
    pub stroke_style: String,
    /// 线条宽度
    pub line_width: f32,
    /// 透明度
    pub global_alpha: f32,
    /// 绘制命令队列
    pub commands: Vec<DrawCommand>,
    /// 变换矩阵（简化）
    pub transform: TransformMatrix,
    /// 裁剪区域（简化）
    pub clip_region: Option<ClipRegion>,
}

/// 变换矩阵
#[derive(Debug, Clone)]
pub struct TransformMatrix {
    pub a: f32, // 水平缩放
    pub b: f32, // 水平倾斜
    pub c: f32, // 垂直倾斜
    pub d: f32, // 垂直缩放
    pub e: f32, // 水平平移
    pub f: f32, // 垂直平移
}

/// 裁剪区域
#[derive(Debug, Clone)]
pub struct ClipRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Canvas2DContext {
    /// 创建新的 Canvas 2D 上下文
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            fill_style: "#000000".to_string(),
            stroke_style: "#000000".to_string(),
            line_width: 1.0,
            global_alpha: 1.0,
            commands: Vec::new(),
            transform: TransformMatrix {
                a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0,
            },
            clip_region: None,
        }
    }

    /// 填充矩形
    pub fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let color = self.parse_color(&self.fill_style);
        self.commands.push(DrawCommand::Rect {
            x,
            y,
            width,
            height,
            color,
        });
    }

    /// 描边矩形
    pub fn stroke_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let color = self.parse_color(&self.stroke_style);
        // 使用 4 个细矩形模拟边框
        let lw = self.line_width;
        
        // 上边框
        self.commands.push(DrawCommand::Rect {
            x,
            y,
            width,
            height: lw,
            color,
        });
        
        // 下边框
        self.commands.push(DrawCommand::Rect {
            x,
            y: y + height - lw,
            width,
            height: lw,
            color,
        });
        
        // 左边框
        self.commands.push(DrawCommand::Rect {
            x,
            y,
            width: lw,
            height,
            color,
        });
        
        // 右边框
        self.commands.push(DrawCommand::Rect {
            x: x + width - lw,
            y,
            width: lw,
            height,
            color,
        });
    }

    /// 清除矩形区域
    pub fn clear_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.commands.push(DrawCommand::Rect {
            x,
            y,
            width,
            height,
            color: [0.0, 0.0, 0.0, 0.0], // 透明
        });
    }

    /// 填充整个画布
    pub fn fill(&mut self) {
        self.fill_rect(0.0, 0.0, self.width as f32, self.height as f32);
    }

    /// 描边路径（简化为描边矩形）
    pub fn stroke(&mut self) {
        // 简化实现：实际应该描边当前路径
    }

    /// 开始新路径
    pub fn begin_path(&mut self) {
        // 简化实现
    }

    /// 移动到指定点
    pub fn move_to(&mut self, _x: f32, _y: f32) {
        // 简化实现
    }

    /// 画线到指定点
    pub fn line_to(&mut self, _x: f32, _y: f32) {
        // 简化实现
    }

    /// 画圆弧
    pub fn arc(&mut self, _x: f32, _y: f32, _radius: f32, _start_angle: f32, _end_angle: f32) {
        // 简化实现
    }

    /// 关闭路径
    pub fn close_path(&mut self) {
        // 简化实现
    }

    /// 填充圆形
    pub fn fill_circle(&mut self, x: f32, y: f32, radius: f32) {
        let color = self.parse_color(&self.fill_style);
        self.commands.push(DrawCommand::Circle {
            center_x: x,
            center_y: y,
            radius_x: radius,
            radius_y: radius,
            color,
        });
    }

    /// 设置填充颜色
    pub fn set_fill_style(&mut self, style: &str) {
        self.fill_style = style.to_string();
    }

    /// 设置描边颜色
    pub fn set_stroke_style(&mut self, style: &str) {
        self.stroke_style = style.to_string();
    }

    /// 设置线条宽度
    pub fn set_line_width(&mut self, width: f32) {
        self.line_width = width;
    }

    /// 设置全局透明度
    pub fn set_global_alpha(&mut self, alpha: f32) {
        self.global_alpha = alpha.clamp(0.0, 1.0);
    }

    /// 保存当前状态
    pub fn save(&mut self) {
        // 简化实现：实际应该保存状态栈
    }

    /// 恢复之前保存的状态
    pub fn restore(&mut self) {
        // 简化实现
    }

    /// 平移变换
    pub fn translate(&mut self, x: f32, y: f32) {
        self.transform.e += x;
        self.transform.f += y;
    }

    /// 旋转变换
    pub fn rotate(&mut self, _angle: f32) {
        // 简化实现
    }

    /// 缩放变换
    pub fn scale(&mut self, x: f32, y: f32) {
        self.transform.a *= x;
        self.transform.d *= y;
    }

    /// 获取绘制命令
    pub fn get_commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// 清空绘制命令
    pub fn clear_commands(&mut self) {
        self.commands.clear();
    }

    /// 解析颜色字符串
    fn parse_color(&self, color: &str) -> [f32; 4] {
        if color.starts_with('#') {
            self.parse_hex_color(color)
        } else if color.starts_with("rgb") {
            self.parse_rgb_color(color)
        } else if color.starts_with("rgba") {
            self.parse_rgba_color(color)
        } else {
            // 预设颜色名称
            self.parse_named_color(color)
        }
    }

    /// 解析十六进制颜色
    fn parse_hex_color(&self, hex: &str) -> [f32; 4] {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return [
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    self.global_alpha,
                ];
            }
        }
        [0.0, 0.0, 0.0, self.global_alpha]
    }

    /// 解析 RGB 颜色
    fn parse_rgb_color(&self, rgb: &str) -> [f32; 4] {
        // 简化解析：rgb(r, g, b)
        let parts: Vec<&str> = rgb
            .trim_start_matches("rgb(")
            .trim_end_matches(')')
            .split(',')
            .map(|s| s.trim())
            .collect();
        
        if parts.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].parse::<f32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
            ) {
                return [
                    r / 255.0,
                    g / 255.0,
                    b / 255.0,
                    self.global_alpha,
                ];
            }
        }
        [0.0, 0.0, 0.0, self.global_alpha]
    }

    /// 解析 RGBA 颜色
    fn parse_rgba_color(&self, rgba: &str) -> [f32; 4] {
        let parts: Vec<&str> = rgba
            .trim_start_matches("rgba(")
            .trim_end_matches(')')
            .split(',')
            .map(|s| s.trim())
            .collect();
        
        if parts.len() == 4 {
            if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
                parts[0].parse::<f32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
                parts[3].parse::<f32>(),
            ) {
                return [
                    r / 255.0,
                    g / 255.0,
                    b / 255.0,
                    a * self.global_alpha,
                ];
            }
        }
        [0.0, 0.0, 0.0, self.global_alpha]
    }

    /// 解析命名颜色
    fn parse_named_color(&self, name: &str) -> [f32; 4] {
        match name.to_lowercase().as_str() {
            "black" => [0.0, 0.0, 0.0, self.global_alpha],
            "white" => [1.0, 1.0, 1.0, self.global_alpha],
            "red" => [1.0, 0.0, 0.0, self.global_alpha],
            "green" => [0.0, 1.0, 0.0, self.global_alpha],
            "blue" => [0.0, 0.0, 1.0, self.global_alpha],
            "yellow" => [1.0, 1.0, 0.0, self.global_alpha],
            "cyan" => [0.0, 1.0, 1.0, self.global_alpha],
            "magenta" => [1.0, 0.0, 1.0, self.global_alpha],
            _ => [0.0, 0.0, 0.0, self.global_alpha],
        }
    }
}

impl Default for Canvas2DContext {
    fn default() -> Self {
        Self::new(300, 150) // HTML5 Canvas 默认尺寸
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_canvas() {
        let canvas = Canvas2DContext::new(800, 600);
        assert_eq!(canvas.width, 800);
        assert_eq!(canvas.height, 600);
        assert_eq!(canvas.fill_style, "#000000");
    }

    #[test]
    fn test_fill_rect() {
        let mut canvas = Canvas2DContext::new(800, 600);
        canvas.fill_rect(10.0, 20.0, 100.0, 50.0);
        
        assert_eq!(canvas.commands.len(), 1);
        if let DrawCommand::Rect { x, y, width, height, color } = &canvas.commands[0] {
            assert_eq!(*x, 10.0);
            assert_eq!(*y, 20.0);
            assert_eq!(*width, 100.0);
            assert_eq!(*height, 50.0);
            assert_eq!(*color, [0.0, 0.0, 0.0, 1.0]);
        } else {
            panic!("Expected Rect command");
        }
    }

    #[test]
    fn test_fill_circle() {
        let mut canvas = Canvas2DContext::new(800, 600);
        canvas.set_fill_style("red");
        canvas.fill_circle(100.0, 100.0, 50.0);
        
        assert_eq!(canvas.commands.len(), 1);
        if let DrawCommand::Circle { center_x, center_y, radius_x, radius_y, color } = &canvas.commands[0] {
            assert_eq!(*center_x, 100.0);
            assert_eq!(*center_y, 100.0);
            assert_eq!(*radius_x, 50.0);
            assert_eq!(*radius_y, 50.0);
            assert_eq!(*color, [1.0, 0.0, 0.0, 1.0]);
        } else {
            panic!("Expected Circle command");
        }
    }

    #[test]
    fn test_parse_hex_color() {
        let canvas = Canvas2DContext::new(800, 600);
        let color = canvas.parse_hex_color("#ff0000");
        assert_eq!(color, [1.0, 0.0, 0.0, 1.0]);
        
        let color = canvas.parse_hex_color("#00ff00");
        assert_eq!(color, [0.0, 1.0, 0.0, 1.0]);
        
        let color = canvas.parse_hex_color("#0000ff");
        assert_eq!(color, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_parse_rgb_color() {
        let canvas = Canvas2DContext::new(800, 600);
        let color = canvas.parse_rgb_color("rgb(255, 128, 0)");
        assert!((color[0] - 1.0).abs() < 0.001);
        assert!((color[1] - 0.502).abs() < 0.001);
        assert!((color[2] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_rgba_color() {
        let canvas = Canvas2DContext::new(800, 600);
        let color = canvas.parse_rgba_color("rgba(255, 0, 0, 0.5)");
        assert_eq!(color, [1.0, 0.0, 0.0, 0.5]);
    }

    #[test]
    fn test_named_colors() {
        let canvas = Canvas2DContext::new(800, 600);
        
        assert_eq!(canvas.parse_named_color("red"), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(canvas.parse_named_color("green"), [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(canvas.parse_named_color("blue"), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(canvas.parse_named_color("white"), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(canvas.parse_named_color("black"), [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_transform() {
        let mut canvas = Canvas2DContext::new(800, 600);
        
        canvas.translate(100.0, 50.0);
        assert_eq!(canvas.transform.e, 100.0);
        assert_eq!(canvas.transform.f, 50.0);
        
        canvas.scale(2.0, 3.0);
        assert_eq!(canvas.transform.a, 2.0);
        assert_eq!(canvas.transform.d, 3.0);
    }

    #[test]
    fn test_global_alpha() {
        let mut canvas = Canvas2DContext::new(800, 600);
        
        canvas.set_global_alpha(0.5);
        assert_eq!(canvas.global_alpha, 0.5);
        
        // 测试边界值
        canvas.set_global_alpha(1.5);
        assert_eq!(canvas.global_alpha, 1.0);
        
        canvas.set_global_alpha(-0.5);
        assert_eq!(canvas.global_alpha, 0.0);
    }

    #[test]
    fn test_stroke_rect() {
        let mut canvas = Canvas2DContext::new(800, 600);
        canvas.set_stroke_style("blue");
        canvas.set_line_width(2.0);
        canvas.stroke_rect(10.0, 10.0, 100.0, 50.0);
        
        // 应该生成 4 个矩形（上下左右边框）
        assert_eq!(canvas.commands.len(), 4);
    }

    #[test]
    fn test_clear_rect() {
        let mut canvas = Canvas2DContext::new(800, 600);
        canvas.clear_rect(0.0, 0.0, 800.0, 600.0);
        
        assert_eq!(canvas.commands.len(), 1);
        if let DrawCommand::Rect { color, .. } = &canvas.commands[0] {
            assert_eq!(*color, [0.0, 0.0, 0.0, 0.0]); // 透明
        }
    }

    #[test]
    fn test_commands_clear() {
        let mut canvas = Canvas2DContext::new(800, 600);
        canvas.fill_rect(0.0, 0.0, 100.0, 100.0);
        assert_eq!(canvas.commands.len(), 1);
        
        canvas.clear_commands();
        assert_eq!(canvas.commands.len(), 0);
    }
}
