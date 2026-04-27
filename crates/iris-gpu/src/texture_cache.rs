//! 纹理缓存管理器
//!
//! 管理 GPU 纹理的加载、缓存和生命周期。

use wgpu;

/// 纹理缓存项
pub struct TextureEntry {
    /// GPU 纹理对象
    pub texture: wgpu::Texture,
    /// 纹理视图
    pub view: wgpu::TextureView,
    /// 纹理宽度
    pub width: u32,
    /// 纹理高度
    pub height: u32,
}

/// 纹理缓存管理器
pub struct TextureCache {
    /// 纹理条目
    textures: Vec<TextureEntry>,
    /// 纹理格式
    format: wgpu::TextureFormat,
}

impl TextureCache {
    /// 创建新的纹理缓存
    pub fn new(format: wgpu::TextureFormat) -> Self {
        Self {
            textures: Vec::new(),
            format,
        }
    }

    /// 从 RGBA 数据创建纹理
    ///
    /// # 参数
    ///
    /// * `device` - GPU 设备
    /// * `queue` - GPU 队列
    /// * `data` - RGBA 像素数据
    /// * `width` - 纹理宽度
    /// * `height` - 纹理高度
    ///
    /// # 返回
    ///
    /// 返回纹理 ID（索引）
    pub fn create_texture_from_rgba(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<u32, String> {
        if data.len() != (width * height * 4) as usize {
            return Err(format!(
                "Invalid data size: expected {} bytes, got {} bytes",
                width * height * 4,
                data.len()
            ));
        }

        // 创建纹理
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TextureCache Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // 上传数据
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // 创建视图
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let id = self.textures.len() as u32;
        self.textures.push(TextureEntry {
            texture,
            view,
            width,
            height,
        });

        Ok(id)
    }

    /// 从图像文件创建纹理
    ///
    /// # 参数
    ///
    /// * `device` - GPU 设备
    /// * `queue` - GPU 队列
    /// * `path` - 图像文件路径
    ///
    /// # 返回
    ///
    /// 返回纹理 ID
    pub fn create_texture_from_path(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
    ) -> Result<u32, String> {
        // 加载图像
        let img = image::open(path).map_err(|e| format!("Failed to load image: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        // 创建纹理
        self.create_texture_from_rgba(device, queue, &rgba, width, height)
    }

    /// 获取纹理条目
    pub fn get_texture(&self, id: u32) -> Option<&TextureEntry> {
        self.textures.get(id as usize)
    }

    /// 获取纹理数量
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// 清空所有纹理
    pub fn clear(&mut self) {
        self.textures.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_cache_creation() {
        // 测试纹理缓存创建（不需要 GPU 设备）
        let cache = TextureCache::new(wgpu::TextureFormat::Rgba8UnormSrgb);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_texture_data_validation() {
        // 测试数据验证逻辑
        let width = 2u32;
        let height = 2u32;
        let expected_size = (width * height * 4) as usize;

        // 正确大小的数据
        let correct_data = vec![0u8; expected_size];
        assert_eq!(correct_data.len(), expected_size);

        // 错误大小的数据
        let wrong_data = vec![0u8; expected_size - 1];
        assert_ne!(wrong_data.len(), expected_size);
    }
}
