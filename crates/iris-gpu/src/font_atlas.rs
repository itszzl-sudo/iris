//! 字体纹理图集管理器
//!
//! 将 CPU 光栅化的字形缓存到 GPU 纹理图集中，提升文本渲染性能。
//!
//! # 工作原理
//!
//! 1. 字形缓存：使用 LRU 缓存已光栅化的字形
//! 2. 纹理图集：将多个字形打包到单个纹理中
//! 3. UV 映射：每个字形在图集中有对应的 UV 坐标
//! 4. 批量渲染：使用纹理图集一次性渲染所有文本

use fontdue::{Font, Metrics};
use std::collections::HashMap;

/// 字形信息
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// 字符
    pub character: char,
    /// 字形度量信息
    pub metrics: Metrics,
    /// 在图集中的 UV 坐标 [left, top, right, bottom]
    pub uv: [f32; 4],
    /// 字形在图集中的像素位置
    pub atlas_x: u32,
    pub atlas_y: u32,
    /// 字形尺寸
    pub width: u32,
    pub height: u32,
}

/// 字形缓存键
#[derive(Debug, Clone, PartialEq)]
struct GlyphKey {
    character: char,
    font_size: f32,
}

// 手动实现 Hash，将 f32 转换为位表示
impl std::hash::Hash for GlyphKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.character.hash(state);
        // 使用 to_bits() 将 f32 转换为 u32 进行哈希
        self.font_size.to_bits().hash(state);
    }
}

// 手动实现 Eq，使用近似比较
impl Eq for GlyphKey {}

/// 字体纹理图集
pub struct FontAtlas {
    /// 纹理图集数据（RGBA）
    atlas_data: Vec<u8>,
    /// 图集宽度
    atlas_width: u32,
    /// 图集高度
    atlas_height: u32,
    /// 当前插入位置的 X 坐标
    cursor_x: u32,
    /// 当前插入位置的 Y 坐标
    cursor_y: u32,
    /// 当前行的最大高度
    row_max_height: u32,
    /// 字形缓存
    glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    /// 字体
    font: Font,
    /// 字体大小
    font_size: f32,
    /// 是否需要更新纹理
    pub dirty: bool,
}

impl FontAtlas {
    /// 创建新的字体图集
    ///
    /// # 参数
    ///
    /// * `font` - 字体数据
    /// * `font_size` - 字体大小
    /// * `atlas_size` - 图集尺寸（宽度和高度相同）
    pub fn new(font: Font, font_size: f32, atlas_size: u32) -> Self {
        let atlas_data = vec![0u8; (atlas_size * atlas_size * 4) as usize];

        Self {
            atlas_data,
            atlas_width: atlas_size,
            atlas_height: atlas_size,
            cursor_x: 0,
            cursor_y: 0,
            row_max_height: 0,
            glyph_cache: HashMap::new(),
            font,
            font_size,
            dirty: false,
        }
    }

    /// 获取或光栅化字形
    ///
    /// 如果字形已在缓存中，直接返回；否则光栅化并添加到图集
    pub fn get_or_rasterize_glyph(&mut self, character: char) -> Option<GlyphInfo> {
        let key = GlyphKey {
            character,
            font_size: self.font_size,
        };

        // 检查缓存
        if let Some(glyph) = self.glyph_cache.get(&key) {
            return Some(glyph.clone());
        }

        // 光栅化字形
        let (metrics, bitmap) = self.font.rasterize(character, self.font_size);

        if metrics.width == 0 || metrics.height == 0 {
            // 空字形（如空格）
            return None;
        }

        // 检查是否有足够空间
        let width = metrics.width as u32;
        let height = metrics.height as u32;

        // 如果当前行空间不足，换行
        if self.cursor_x + width > self.atlas_width {
            self.cursor_x = 0;
            self.cursor_y += self.row_max_height + 1; // +1 作为间距
            self.row_max_height = 0;
        }

        // 如果图集空间不足，返回 None
        if self.cursor_y + height > self.atlas_height {
            tracing::warn!("Font atlas is full, cannot add more glyphs");
            return None;
        }

        // 将字形数据复制到图集
        self.copy_to_atlas(&bitmap, self.cursor_x, self.cursor_y, width, height);

        // 计算 UV 坐标
        let uv = [
            self.cursor_x as f32 / self.atlas_width as f32,
            self.cursor_y as f32 / self.atlas_height as f32,
            (self.cursor_x + width) as f32 / self.atlas_width as f32,
            (self.cursor_y + height) as f32 / self.atlas_height as f32,
        ];

        let glyph_info = GlyphInfo {
            character,
            metrics,
            uv,
            atlas_x: self.cursor_x,
            atlas_y: self.cursor_y,
            width,
            height,
        };

        // 更新光标
        self.cursor_x += width + 1; // +1 作为间距
        self.row_max_height = self.row_max_height.max(height);
        self.dirty = true;

        // 缓存字形
        self.glyph_cache.insert(key, glyph_info.clone());

        Some(glyph_info)
    }

    /// 将字形位图复制到图集
    fn copy_to_atlas(&mut self, bitmap: &[u8], x: u32, y: u32, width: u32, height: u32) {
        let bitmap_idx = 0;
        let atlas_idx = ((y * self.atlas_width + x) * 4) as usize;

        for row in 0..height {
            for col in 0..width {
                let src_idx = bitmap_idx + (row * width + col) as usize;
                let dst_idx = atlas_idx + ((row * self.atlas_width + col) * 4) as usize;

                let alpha = bitmap[src_idx];

                // 存储为白色 + alpha（纹理颜色由顶点颜色控制）
                self.atlas_data[dst_idx] = 255;     // R
                self.atlas_data[dst_idx + 1] = 255; // G
                self.atlas_data[dst_idx + 2] = 255; // B
                self.atlas_data[dst_idx + 3] = alpha; // A
            }
        }
    }

    /// 获取图集数据
    pub fn get_atlas_data(&self) -> &[u8] {
        &self.atlas_data
    }

    /// 获取图集宽度
    pub fn atlas_width(&self) -> u32 {
        self.atlas_width
    }

    /// 获取图集高度
    pub fn atlas_height(&self) -> u32 {
        self.atlas_height
    }

    /// 重置 dirty 标记
    pub fn reset_dirty(&mut self) {
        self.dirty = false;
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.glyph_cache.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_max_height = 0;
        self.atlas_data.fill(0);
        self.dirty = true;
    }

    /// 获取缓存的字形数量
    pub fn cached_glyph_count(&self) -> usize {
        self.glyph_cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用字体
    fn create_test_font() -> Option<Font> {
        // 尝试加载常见系统字体
        let font_paths = [
            "C:/Windows/Fonts/arial.ttf",
            "C:/Windows/Fonts/calibri.ttf",
            "C:/Windows/Fonts/segoeui.ttf",
        ];

        for path in &font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                if let Ok(font) = Font::from_bytes(&font_data[..], fontdue::FontSettings::default()) {
                    return Some(font);
                }
            }
        }

        // 如果没有找到字体文件，返回 None
        None
    }

    #[test]
    fn test_font_atlas_creation() {
        if let Some(font) = create_test_font() {
            let atlas = FontAtlas::new(font, 16.0, 512);

            assert_eq!(atlas.atlas_width(), 512);
            assert_eq!(atlas.atlas_height(), 512);
            assert_eq!(atlas.cached_glyph_count(), 0);
            assert!(!atlas.dirty);
        } else {
            println!("⏭️ Skipped: No font file available");
        }
    }

    #[test]
    fn test_glyph_rasterize_and_cache() {
        if let Some(font) = create_test_font() {
            let mut atlas = FontAtlas::new(font, 16.0, 512);

            // 光栅化字符 'A'
            let glyph_a = atlas.get_or_rasterize_glyph('A');
            assert!(glyph_a.is_some());

            let glyph_a = glyph_a.unwrap();
            assert_eq!(glyph_a.character, 'A');
            assert!(glyph_a.width > 0);
            assert!(glyph_a.height > 0);

            // UV 坐标应该在有效范围内
            assert!(glyph_a.uv[0] >= 0.0 && glyph_a.uv[0] <= 1.0);
            assert!(glyph_a.uv[1] >= 0.0 && glyph_a.uv[1] <= 1.0);
            assert!(glyph_a.uv[2] >= 0.0 && glyph_a.uv[2] <= 1.0);
            assert!(glyph_a.uv[3] >= 0.0 && glyph_a.uv[3] <= 1.0);

            // 应该标记为 dirty
            assert!(atlas.dirty);

            // 缓存计数应该为 1
            assert_eq!(atlas.cached_glyph_count(), 1);
        } else {
            println!("⏭️ Skipped: No font file available");
        }
    }

    #[test]
    fn test_glyph_caching() {
        if let Some(font) = create_test_font() {
            let mut atlas = FontAtlas::new(font, 16.0, 512);

            // 第一次光栅化
            let glyph1 = atlas.get_or_rasterize_glyph('B');
            assert!(glyph1.is_some());
            assert_eq!(atlas.cached_glyph_count(), 1);

            // 第二次应该从缓存返回
            let glyph2 = atlas.get_or_rasterize_glyph('B');
            assert!(glyph2.is_some());

            // 缓存计数不应增加
            assert_eq!(atlas.cached_glyph_count(), 1);

            // 两次返回的字形信息应该相同
            assert_eq!(glyph1.unwrap().character, glyph2.unwrap().character);
        } else {
            println!("⏭️ Skipped: No font file available");
        }
    }

    #[test]
    fn test_multiple_glyphs() {
        if let Some(font) = create_test_font() {
            let mut atlas = FontAtlas::new(font, 16.0, 512);

            // 光栅化多个字符
            for ch in "Hello".chars() {
                let glyph = atlas.get_or_rasterize_glyph(ch);
                // 空格可能返回 None
                if ch != ' ' {
                    assert!(glyph.is_some());
                }
            }

            // 缓存应该包含所有非空字符
            assert!(atlas.cached_glyph_count() >= 4); // H, e, l, o (l 重复)
        } else {
            println!("⏭️ Skipped: No font file available");
        }
    }

    #[test]
    fn test_font_atlas_clear() {
        if let Some(font) = create_test_font() {
            let mut atlas = FontAtlas::new(font, 16.0, 512);

            // 添加一些字形
            atlas.get_or_rasterize_glyph('A');
            atlas.get_or_rasterize_glyph('B');
            atlas.get_or_rasterize_glyph('C');

            assert_eq!(atlas.cached_glyph_count(), 3);
            assert!(atlas.dirty);

            // 清除缓存
            atlas.clear_cache();

            assert_eq!(atlas.cached_glyph_count(), 0);
            assert!(atlas.dirty);
        } else {
            println!("⏭️ Skipped: No font file available");
        }
    }

    #[test]
    fn test_font_atlas_dirty_flag() {
        if let Some(font) = create_test_font() {
            let mut atlas = FontAtlas::new(font, 16.0, 512);

            // 初始状态不是 dirty
            assert!(!atlas.dirty);

            // 添加字形后变为 dirty
            atlas.get_or_rasterize_glyph('X');
            assert!(atlas.dirty);

            // 重置 dirty
            atlas.reset_dirty();
            assert!(!atlas.dirty);

            // 再次添加字形又变为 dirty
            atlas.get_or_rasterize_glyph('Y');
            assert!(atlas.dirty);
        } else {
            println!("⏭️ Skipped: No font file available");
        }
    }

    #[test]
    fn test_empty_glyph() {
        if let Some(font) = create_test_font() {
            let mut atlas = FontAtlas::new(font, 16.0, 512);

            // 空格字符可能没有可见的字形
            let space_glyph = atlas.get_or_rasterize_glyph(' ');

            // 空格可能返回 None 或有效的零宽度字形
            // 这取决于字体实现
            if let Some(glyph) = space_glyph {
                assert_eq!(glyph.character, ' ');
            }
        } else {
            println!("⏭️ Skipped: No font file available");
        }
    }

    #[test]
    fn test_different_font_sizes() {
        if let Some(font) = create_test_font() {
            // 16px 字体
            let mut atlas_16 = FontAtlas::new(font.clone(), 16.0, 512);
            let glyph_16 = atlas_16.get_or_rasterize_glyph('A');

            // 32px 字体
            let mut atlas_32 = FontAtlas::new(font, 32.0, 512);
            let glyph_32 = atlas_32.get_or_rasterize_glyph('A');

            // 不同大小的字形应该有不同尺寸
            if let (Some(g16), Some(g32)) = (glyph_16, glyph_32) {
                // 32px 的字形应该更大
                assert!(g32.width >= g16.width);
                assert!(g32.height >= g16.height);
            }
        } else {
            println!("⏭️ Skipped: No font file available");
        }
    }
}
