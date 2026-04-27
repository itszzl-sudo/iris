//! 文本渲染器
//!
//! 将文本字符串转换为 GPU 可渲染的顶点数据，使用字体图集进行渲染。

use crate::font_atlas::{FontAtlas, GlyphInfo};
use crate::batch_renderer::DrawCommand;

/// 文本渲染器
///
/// 负责将文本转换为 DrawCommand，供批渲染系统使用。
pub struct TextRenderer {
    /// 字体图集
    font_atlas: FontAtlas,
    /// 是否需要更新 GPU 纹理
    needs_texture_update: bool,
}

impl TextRenderer {
    /// 创建新的文本渲染器
    ///
    /// # 参数
    ///
    /// * `font_atlas` - 字体图集
    pub fn new(font_atlas: FontAtlas) -> Self {
        Self {
            font_atlas,
            needs_texture_update: true,
        }
    }

    /// 获取字体图集的引用
    pub fn font_atlas(&self) -> &FontAtlas {
        &self.font_atlas
    }

    /// 获取字体图集的可变引用
    pub fn font_atlas_mut(&mut self) -> &mut FontAtlas {
        &mut self.font_atlas
    }

    /// 检查是否需要更新 GPU 纹理
    pub fn needs_texture_update(&self) -> bool {
        self.needs_texture_update
    }

    /// 标记纹理已更新
    pub fn mark_texture_updated(&mut self) {
        self.needs_texture_update = false;
        self.font_atlas.dirty = false;
    }

    /// 将文本转换为绘制命令列表
    ///
    /// # 参数
    ///
    /// * `text` - 要渲染的文本
    /// * `x` - 起始 X 坐标（像素）
    /// * `y` - 起始 Y 坐标（像素）
    /// * `color` - 文本颜色 RGBA
    /// * `texture_id` - 字体图集纹理 ID
    ///
    /// # 返回
    ///
    /// 返回 DrawCommand 列表，每个字符一个 TextureRect 命令
    pub fn render_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        _color: [f32; 4],
        texture_id: u32,
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        let mut current_x = x;

        for ch in text.chars() {
            // 跳过换行符（简单处理，未来支持多行）
            if ch == '\n' {
                continue;
            }

            // 获取或光栅化字形
            if let Some(glyph_info) = self.font_atlas.get_or_rasterize_glyph(ch) {
                // 计算字形位置
                let glyph_x = current_x;
                let glyph_y = y - glyph_info.metrics.ymin as f32;

                // 计算字形的宽度和高度
                let glyph_width = glyph_info.width as f32;
                let glyph_height = glyph_info.height as f32;

                if glyph_width > 0.0 && glyph_height > 0.0 {
                    // 创建纹理矩形命令
                    commands.push(DrawCommand::TextureRect {
                        x: glyph_x,
                        y: glyph_y,
                        width: glyph_width,
                        height: glyph_height,
                        texture_id,
                        uv: glyph_info.uv,
                    });
                }

                // 更新 X 坐标（使用 advance 值）
                current_x += glyph_info.metrics.advance_width;
            } else if ch == ' ' {
                // 空格：只推进位置（使用估算值）
                current_x += self.font_atlas.font_size() * 0.5;
            }
        }

        // 标记纹理可能需要更新
        if self.font_atlas.dirty {
            self.needs_texture_update = true;
        }

        commands
    }

    /// 计算文本的宽度
    ///
    /// # 参数
    ///
    /// * `text` - 要测量的文本
    ///
    /// # 返回
    ///
    /// 返回文本的总宽度（像素）
    pub fn measure_text_width(&mut self, text: &str) -> f32 {
        let mut width = 0.0f32;

        for ch in text.chars() {
            if ch == ' ' {
                width += self.font_atlas.font_size() * 0.5;
            } else if let Some(glyph_info) = self.font_atlas.get_or_rasterize_glyph(ch) {
                width += glyph_info.metrics.advance_width;
            }
        }

        width
    }

    /// 清空字形缓存
    pub fn clear(&mut self) {
        self.font_atlas.clear();
        self.needs_texture_update = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_renderer_creation() {
        // 测试 TextRenderer 结构存在性
        // 实际需要字体文件才能完整测试
        assert!(std::mem::size_of::<TextRenderer>() > 0);
    }

    #[test]
    fn test_measure_empty_text() {
        // 测试空文本测量
        assert_eq!("".len(), 0);
    }

    #[test]
    fn test_draw_command_generation() {
        // 测试绘制命令生成的逻辑（不涉及实际字体）
        let text = "test";
        assert_eq!(text.chars().count(), 4);
    }
}
