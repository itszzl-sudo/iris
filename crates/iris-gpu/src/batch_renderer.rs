//! Iris GPU 2D 批渲染系统
//!
//! 合并多次 2D 绘制调用为单次 GPU draw call，支持：
//! - 纯色矩形
//! - 线性渐变（水平/垂直）
//! - 边框渲染
//! - Alpha 混合
//! - 文本渲染（fontdue 光栅化）

use bytemuck::{Pod, Zeroable};
use fontdue::Font;

/// 批渲染顶点：位置 + 颜色 + UV 坐标
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BatchVertex {
    /// 屏幕空间坐标（像素）
    pub position: [f32; 2],
    /// RGBA 颜色
    pub color: [f32; 4],
    /// 纹理 UV 坐标（预留）
    pub uv: [f32; 2],
}

impl BatchVertex {
    /// 生成 wgpu 顶点缓冲区布局描述。
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BatchVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() + std::mem::size_of::<[f32; 4]>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// 批渲染绘制命令。
#[derive(Clone, Debug)]
pub enum DrawCommand {
    /// 纯色矩形。
    Rect {
        /// X 坐标（像素）。
        x: f32,
        /// Y 坐标（像素）。
        y: f32,
        /// 宽度（像素）。
        width: f32,
        /// 高度（像素）。
        height: f32,
        /// RGBA 颜色。
        color: [f32; 4],
    },
    /// 线性渐变矩形。
    GradientRect {
        /// X 坐标（像素）。
        x: f32,
        /// Y 坐标（像素）。
        y: f32,
        /// 宽度（像素）。
        width: f32,
        /// 高度（像素）。
        height: f32,
        /// 起始颜色。
        start_color: [f32; 4],
        /// 结束颜色。
        end_color: [f32; 4],
        /// true = 水平渐变，false = 垂直渐变。
        horizontal: bool,
    },
    /// 边框。
    Border {
        /// X 坐标（像素）。
        x: f32,
        /// Y 坐标（像素）。
        y: f32,
        /// 宽度（像素）。
        width: f32,
        /// 高度（像素）。
        height: f32,
        /// 边框宽度 (上, 右, 下, 左)。
        border_width: (f32, f32, f32, f32),
        /// 边框颜色。
        border_color: [f32; 4],
    },
    /// 纹理矩形。
    TextureRect {
        /// X 坐标（像素）。
        x: f32,
        /// Y 坐标（像素）。
        y: f32,
        /// 宽度（像素）。
        width: f32,
        /// 高度（像素）。
        height: f32,
        /// 纹理 ID。
        texture_id: u32,
        /// UV 坐标 (u1, v1, u2, v2)。
        uv: [f32; 4],
    },
}

/// 2D 批渲染器。
///
/// 维护顶点池和索引池，在 `flush()` 时一次性提交到 GPU。
pub struct BatchRenderer {
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertices: Vec<BatchVertex>,
    indices: Vec<u16>,
    capacity: usize,
    screen_width: f32,
    screen_height: f32,
    
    // 字体渲染
    font: Option<Font>,
    font_size: f32,
    
    // 纹理管理
    textures: Vec<wgpu::Texture>,
    texture_views: Vec<wgpu::TextureView>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group: Option<wgpu::BindGroup>,
    texture_sampler: wgpu::Sampler,
}

impl BatchRenderer {
    /// 创建新的批渲染器。
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        screen_width: f32,
        screen_height: f32,
        capacity: usize,
    ) -> Self {
        // ========== 创建批渲染 Shader ==========
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Batch Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("batch_shader.wgsl").into()),
        });

        // ========== 创建渲染管线 ==========
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Batch Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Batch Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[BatchVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // 创建顶点/索引缓冲区（使用最大容量）
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batch Vertex Buffer"),
            size: (capacity * std::mem::size_of::<BatchVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batch Index Buffer"),
            size: (capacity * 6 * std::mem::size_of::<u16>()) as u64, // 每个矩形 6 索引
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 创建采样器（用于纹理过滤）
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // 创建纹理绑定组布局
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        Self {
            queue: queue.clone(),
            render_pipeline,
            vertex_buffer,
            index_buffer,
            vertices: Vec::with_capacity(capacity * 4),
            indices: Vec::with_capacity(capacity * 6),
            capacity,
            screen_width,
            screen_height,
            font: None,
            font_size: 16.0,
            textures: Vec::new(),
            texture_views: Vec::new(),
            texture_bind_group_layout,
            texture_bind_group: None,
            texture_sampler,
        }
    }

    /// 提交绘制命令（不立即渲染，累积到顶点池）。
    ///
    /// # Panics
    ///
    /// 当顶点池或索引池容量不足时会 panic。
    pub fn submit(&mut self, command: DrawCommand) {
        // 容量检查：每个矩形需要 4 顶点 + 6 索引
        if self.vertices.len() + 4 > self.vertices.capacity()
            || self.indices.len() + 6 > self.indices.capacity()
        {
            panic!(
                "BatchRenderer capacity exceeded: {} rects (max {})",
                self.draw_count(),
                self.capacity
            );
        }

        match command {
            DrawCommand::Rect {
                x,
                y,
                width,
                height,
                color,
            } => {
                self.add_rect(x, y, width, height, color, color);
            }
            DrawCommand::GradientRect {
                x,
                y,
                width,
                height,
                start_color,
                end_color,
                horizontal,
            } => {
                if horizontal {
                    // 水平渐变：左上/左下 = start, 右上/右下 = end
                    self.add_rect(x, y, width, height, start_color, end_color);
                } else {
                    // 垂直渐变：左上/右上 = start, 左下/右下 = end
                    self.add_rect_vertical(x, y, width, height, start_color, end_color);
                }
            }
            DrawCommand::Border {
                x,
                y,
                width,
                height,
                border_width,
                border_color,
            } => {
                let (top, right, bottom, left) = border_width;
                
                // 上边框
                if top > 0.0 {
                    self.add_rect(x, y, width, top, border_color, border_color);
                }
                
                // 下边框
                if bottom > 0.0 {
                    self.add_rect(x, y + height - bottom, width, bottom, border_color, border_color);
                }
                
                // 左边框
                if left > 0.0 {
                    self.add_rect(x, y, left, height, border_color, border_color);
                }
                
                // 右边框
                if right > 0.0 {
                    self.add_rect(x + width - right, y, right, height, border_color, border_color);
                }
            }
            DrawCommand::TextureRect {
                x,
                y,
                width,
                height,
                texture_id,
                uv,
            } => {
                self.submit_texture_rect(x, y, width, height, texture_id, uv);
            }
        }
    }

    /// 将像素坐标转换为归一化设备坐标 (NDC)。
    #[inline]
    fn to_ndc(&self, x: f32, y: f32) -> (f32, f32) {
        (
            (x / self.screen_width) * 2.0 - 1.0,
            1.0 - (y / self.screen_height) * 2.0,
        )
    }

    /// 添加矩形到顶点池（内部实现）。
    fn add_rect_internal(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color_tl: [f32; 4], // 左上
        color_tr: [f32; 4], // 右上
        color_bl: [f32; 4], // 左下
        color_br: [f32; 4], // 右下
    ) {
        let (x1, y1) = self.to_ndc(x, y);
        let (x2, y2) = self.to_ndc(x + width, y + height);

        let base_index = self.vertices.len() as u16;

        // 4 个顶点（左上、右上、左下、右下）
        self.vertices.extend_from_slice(&[
            BatchVertex {
                position: [x1, y1],
                color: color_tl,
                uv: [0.0, 0.0],
            },
            BatchVertex {
                position: [x2, y1],
                color: color_tr,
                uv: [1.0, 0.0],
            },
            BatchVertex {
                position: [x1, y2],
                color: color_bl,
                uv: [0.0, 1.0],
            },
            BatchVertex {
                position: [x2, y2],
                color: color_br,
                uv: [1.0, 1.0],
            },
        ]);

        // 2 个三角形 = 6 索引（顺时针）
        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index + 2,
            base_index + 1,
            base_index + 3,
        ]);
    }

    /// 添加矩形到顶点池（水平渐变）。
    fn add_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color_left: [f32; 4],
        color_right: [f32; 4],
    ) {
        self.add_rect_internal(
            x,
            y,
            width,
            height,
            color_left,
            color_right, // 上边：左→右
            color_left,
            color_right, // 下边：左→右
        );
    }

    /// 添加矩形到顶点池（垂直渐变）。
    fn add_rect_vertical(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color_top: [f32; 4],
        color_bottom: [f32; 4],
    ) {
        self.add_rect_internal(
            x,
            y,
            width,
            height,
            color_top,
            color_top, // 上边：相同颜色
            color_bottom,
            color_bottom, // 下边：相同颜色
        );
    }

    /// 将所有累积的绘制命令提交到 GPU 并渲染。
    pub fn flush(&mut self, render_pass: &mut wgpu::RenderPass<'_>) {
        if self.vertices.is_empty() {
            return;
        }

        // 上传顶点数据到 GPU
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        self.queue
            .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));

        // 执行绘制
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);

        // 清空顶点池，准备下一帧
        self.vertices.clear();
        self.indices.clear();
    }

    /// 获取当前累积的绘制命令数量。
    #[must_use]
    pub fn draw_count(&self) -> usize {
        self.indices.len() / 6
    }

    /// 设置字体。
    pub fn set_font(&mut self, font: Font, size: f32) {
        self.font = Some(font);
        self.font_size = size;
    }

    /// 从字节数据加载纹理。
    pub fn load_texture_from_bytes(
        &mut self,
        device: &wgpu::Device,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<u32, String> {
        // 创建纹理
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Loaded Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // 上传数据到 GPU
        self.queue.write_texture(
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

        // 创建纹理视图
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let texture_id = self.textures.len() as u32;
        self.textures.push(texture);
        self.texture_views.push(view);

        Ok(texture_id)
    }

    /// 添加纹理到批渲染器（简化实现，当前使用纯色占位符）。
    pub fn submit_texture_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        texture_id: u32,
        uv: [f32; 4],
    ) {
        if texture_id >= self.textures.len() as u32 {
            tracing::warn!("Texture ID {} not found, using placeholder", texture_id);
            // 使用粉色占位符表示纹理缺失
            self.add_rect(x, y, width, height, [1.0, 0.0, 1.0, 0.5], [1.0, 0.0, 1.0, 0.5]);
            return;
        }

        // 当前实现：使用半透明占位符
        // 完整实现需要更新 shader 支持纹理采样和 bind group
        let placeholder_color = [0.5, 0.5, 1.0, 0.3]; // 半透明蓝色
        self.add_rect(x, y, width, height, placeholder_color, placeholder_color);
        
        tracing::debug!(
            texture_id = texture_id,
            x = x,
            y = y,
            width = width,
            height = height,
            "Submitted texture rectangle (placeholder mode)"
        );
    }

    /// 渲染文本。
    pub fn submit_text(&mut self, text: &str, x: f32, y: f32, color: [f32; 4]) {
        if self.font.is_none() {
            tracing::warn!("No font set, skipping text rendering");
            return;
        }

        let font_size = self.font_size;
        
        // 收集所有字形数据
        struct GlyphData {
            x: f32,
            y: f32,
            width: f32,
            height: f32,
            bitmap: Vec<u8>,
        }
        
        let mut glyphs = Vec::new();
        let mut cursor_x = x;
        let cursor_y = y;

        // 遍历每个字符并光栅化
        if let Some(ref font) = self.font {
            for ch in text.chars() {
                let (metrics, bitmap) = font.rasterize(ch, font_size);

                if metrics.width > 0 && metrics.height > 0 {
                    glyphs.push(GlyphData {
                        x: cursor_x + metrics.xmin as f32,
                        y: cursor_y + metrics.ymin as f32,
                        width: metrics.width as f32,
                        height: metrics.height as f32,
                        bitmap: bitmap.to_vec(),
                    });
                }

                cursor_x += metrics.advance_width;
            }
        }
        
        // 批量添加字形
        for glyph in glyphs {
            self.add_text_glyph(
                glyph.x,
                glyph.y,
                glyph.width,
                glyph.height,
                &glyph.bitmap,
                color,
            );
        }
    }

    /// 添加文本字形到顶点池。
    fn add_text_glyph(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        bitmap: &[u8],
        color: [f32; 4],
    ) {
        // 简化实现：使用平均 alpha 值创建纯色矩形
        // 完整实现需要创建纹理并使用纹理着色器
        let total_alpha: u32 = bitmap.iter().map(|&b| b as u32).sum();
        let avg_alpha = if bitmap.is_empty() {
            0.0
        } else {
            total_alpha as f32 / (bitmap.len() as f32 * 255.0)
        };

        if avg_alpha > 0.01 {
            let final_color = [
                color[0],
                color[1],
                color[2],
                color[3] * avg_alpha,
            ];

            self.add_rect(x, y, width, height, final_color, final_color);
        }
    }
}
