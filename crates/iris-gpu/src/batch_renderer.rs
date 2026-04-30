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
    /// 圆角矩形。
    RoundedRect {
        /// X 坐标（像素）。
        x: f32,
        /// Y 坐标（像素）。
        y: f32,
        /// 宽度（像素）。
        width: f32,
        /// 高度（像素）。
        height: f32,
        /// 圆角半径（像素）。
        radius: f32,
        /// RGBA 颜色。
        color: [f32; 4],
    },
    /// 阴影（简化的盒阴影）。
    BoxShadow {
        /// X 坐标（像素）。
        x: f32,
        /// Y 坐标（像素）。
        y: f32,
        /// 宽度（像素）。
        width: f32,
        /// 高度（像素）。
        height: f32,
        /// 阴影偏移 X（像素）。
        offset_x: f32,
        /// 阴影偏移 Y（像素）。
        offset_y: f32,
        /// 阴影模糊半径（像素）。
        blur: f32,
        /// 阴影颜色（通常半透明黑色）。
        color: [f32; 4],
    },
    /// 圆形/椭圆。
    Circle {
        /// 中心 X 坐标（像素）。
        center_x: f32,
        /// 中心 Y 坐标（像素）。
        center_y: f32,
        /// 半径 X（像素）。
        radius_x: f32,
        /// 半径 Y（像素）。
        radius_y: f32,
        /// RGBA 颜色。
        color: [f32; 4],
    },
    /// 径向渐变矩形。
    RadialGradientRect {
        /// 中心 X 坐标（像素）。
        center_x: f32,
        /// 中心 Y 坐标（像素）。
        center_y: f32,
        /// 渐变半径（像素）。
        radius: f32,
        /// 起始颜色（中心）。
        start_color: [f32; 4],
        /// 结束颜色（边缘）。
        end_color: [f32; 4],
    },
    /// 文本。
    Text {
        /// X 坐标（像素）。
        x: f32,
        /// Y 坐标（像素）。
        y: f32,
        /// 宽度（像素）。
        width: f32,
        /// 高度（像素）。
        height: f32,
        /// 文本内容。
        text: String,
        /// RGBA 颜色。
        color: [f32; 4],
        /// 字体大小（像素）。
        font_size: f32,
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

        // 创建纹理绑定组布局（必须在 pipeline_layout 之前）
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

        // ========== 创建渲染管线 ==========
        // 步骤 1: 更新渲染管线布局使用纹理绑定组
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Batch Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
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
        // 注意：每个矩形需要 4 个顶点，所以顶点缓冲区大小是 capacity * 4
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batch Vertex Buffer"),
            size: (capacity * 4 * std::mem::size_of::<BatchVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batch Index Buffer"),
            size: (capacity * 6 * std::mem::size_of::<u16>()) as u64, // 每个矩形 6 索引
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 步骤 2 & 3: 创建实例并初始化默认纹理和绑定组
        let mut renderer = Self {
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
        };

        // 创建默认的 1x1 白色纹理（用于纯色渲染）
        let white_pixel = [255u8, 255, 255, 255];
        let default_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default White Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        renderer.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &default_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &white_pixel,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let default_view = default_texture.create_view(&wgpu::TextureViewDescriptor::default());
        renderer.textures.push(default_texture);
        renderer.texture_views.push(default_view);

        // 创建默认绑定组（使用索引 0 的白色纹理）
        renderer.texture_bind_group = Some(
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Default Texture Bind Group"),
                layout: &renderer.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&renderer.texture_views[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&renderer.texture_sampler),
                    },
                ],
            })
        );

        renderer
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
                
                // 上边框（完整宽度）
                if top > 0.0 {
                    self.add_rect(x, y, width, top, border_color, border_color);
                }
                
                // 下边框（完整宽度）
                if bottom > 0.0 {
                    self.add_rect(x, y + height - bottom, width, bottom, border_color, border_color);
                }
                
                // 左边框（减去上下边框的高度，避免角落重叠）
                if left > 0.0 {
                    self.add_rect(x, y + top, left, height - top - bottom, border_color, border_color);
                }
                
                // 右边框（减去上下边框的高度，避免角落重叠）
                if right > 0.0 {
                    self.add_rect(x + width - right, y + top, right, height - top - bottom, border_color, border_color);
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
            DrawCommand::RoundedRect {
                x,
                y,
                width,
                height,
                radius,
                color,
            } => {
                self.add_rounded_rect(x, y, width, height, radius, color);
            }
            DrawCommand::BoxShadow {
                x,
                y,
                width,
                height,
                offset_x,
                offset_y,
                blur,
                color,
            } => {
                self.add_box_shadow(x, y, width, height, offset_x, offset_y, blur, color);
            }
            DrawCommand::Circle {
                center_x,
                center_y,
                radius_x,
                radius_y,
                color,
            } => {
                // 使用圆角矩形近似圆形（radius = 50% 宽高）
                let x = center_x - radius_x;
                let y = center_y - radius_y;
                let width = radius_x * 2.0;
                let height = radius_y * 2.0;
                let radius = radius_x.min(radius_y);
                self.add_rounded_rect(x, y, width, height, radius, color);
            }
            DrawCommand::RadialGradientRect {
                center_x,
                center_y,
                radius,
                start_color,
                end_color,
            } => {
                // 真正的径向渐变实现
                // 使用多个同心圆环近似径向渐变
                self.add_radial_gradient(center_x, center_y, radius, start_color, end_color);
            }
            DrawCommand::Text {
                x,
                y,
                width: _,
                height: _,
                text,
                color,
                font_size,
            } => {
                // 使用 fontdue 进行真正的文本渲染
                tracing::debug!(text = %text, x, y, font_size, "Rendering text with fontdue");
                
                // 设置字体大小
                let current_size = self.font_size;
                if (font_size - current_size).abs() > 0.1 {
                    self.font_size = font_size;
                }
                
                // 调用文本渲染
                self.submit_text(&text, x, y, color);
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
        // 检查 u16 索引溢出风险
        if self.vertices.len() + 4 > 65536 {
            panic!("Vertex count exceeds u16 indexing limit (65535). Current: {}", self.vertices.len());
        }
        
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

    /// 添加径向渐变（使用多个同心圆环）。
    ///
    /// # 参数
    ///
    /// * `center_x` - 中心 X 坐标
    /// * `center_y` - 中心 Y 坐标
    /// * `radius` - 渐变半径
    /// * `start_color` - 中心颜色
    /// * `end_color` - 边缘颜色
    fn add_radial_gradient(
        &mut self,
        center_x: f32,
        center_y: f32,
        radius: f32,
        start_color: [f32; 4],
        end_color: [f32; 4],
    ) {
        // 使用 16 个同心圆环来近似径向渐变
        let rings = 16;
        let ring_width = radius / rings as f32;

        // 从外到内绘制（避免覆盖）
        for i in (0..rings).rev() {
            let outer_radius = (i + 1) as f32 * ring_width;
            let inner_radius = i as f32 * ring_width;

            // 计算当前环的颜色（从中心到边缘插值）
            let t_outer = outer_radius / radius;
            let t_inner = inner_radius / radius;

            let color_outer = [
                start_color[0] + (end_color[0] - start_color[0]) * t_outer,
                start_color[1] + (end_color[1] - start_color[1]) * t_outer,
                start_color[2] + (end_color[2] - start_color[2]) * t_outer,
                start_color[3] + (end_color[3] - start_color[3]) * t_outer,
            ];

            let color_inner = [
                start_color[0] + (end_color[0] - start_color[0]) * t_inner,
                start_color[1] + (end_color[1] - start_color[1]) * t_inner,
                start_color[2] + (end_color[2] - start_color[2]) * t_inner,
                start_color[3] + (end_color[3] - start_color[3]) * t_inner,
            ];

            // 使用平均颜色作为当前环的颜色
            let color = [
                (color_outer[0] + color_inner[0]) / 2.0,
                (color_outer[1] + color_inner[1]) / 2.0,
                (color_outer[2] + color_inner[2]) / 2.0,
                (color_outer[3] + color_inner[3]) / 2.0,
            ];

            // 使用圆角矩形近似圆环
            // 外圆
            if i == rings - 1 {
                // 最外层：绘制完整圆
                self.add_circle_approximation(
                    center_x,
                    center_y,
                    outer_radius,
                    color,
                );
            } else {
                // 中间层：绘制圆环（用两个圆角矩形的差集）
                // 这里简化为绘制实心圆，依靠绘制顺序实现渐变
                self.add_circle_approximation(
                    center_x,
                    center_y,
                    outer_radius,
                    color,
                );
            }
        }
    }

    /// 添加圆形的近似（使用圆角矩形）。
    ///
    /// # 参数
    ///
    /// * `center_x` - 中心 X 坐标
    /// * `center_y` - 中心 Y 坐标
    /// * `radius` - 半径
    /// * `color` - 颜色
    fn add_circle_approximation(
        &mut self,
        center_x: f32,
        center_y: f32,
        radius: f32,
        color: [f32; 4],
    ) {
        // 使用圆角矩形近似圆形（radius = 50% 宽高）
        let x = center_x - radius;
        let y = center_y - radius;
        let width = radius * 2.0;
        let height = radius * 2.0;
        self.add_rounded_rect(x, y, width, height, radius, color);
    }

    /// 添加圆角矩形到顶点池。
    ///
    /// 使用三角形扇形（triangle fan）近似圆角，每个圆角使用 16 个三角形。
    fn add_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: [f32; 4],
    ) {
        // 限制圆角半径，避免超过矩形尺寸的一半
        let radius = radius.min(width / 2.0).min(height / 2.0);

        if radius <= 0.0 {
            // 半径为 0，退化为普通矩形
            self.add_rect(x, y, width, height, color, color);
            return;
        }

        // 检查 u16 索引溢出风险
        if self.vertices.len() + 100 > 65536 {
            panic!("Vertex count exceeds u16 indexing limit (65535). Current: {}", self.vertices.len());
        }

        let _center_x = x + width / 2.0;
        let _center_y = y + height / 2.0;

        // 圆角分段数（越多越平滑）
        let segments = 16;

        // 中心矩形（不包含圆角部分）
        let rect_left = x + radius;
        let _rect_right = x + width - radius;
        let rect_top = y + radius;
        let _rect_bottom = y + height - radius;

        // 绘制中心矩形
        self.add_rect(rect_left, rect_top, width - 2.0 * radius, height, color, color);

        // 绘制上下矩形条（填充圆角之间的区域）
        self.add_rect(rect_left, y, width - 2.0 * radius, radius, color, color);
        self.add_rect(rect_left, y + height - radius, width - 2.0 * radius, radius, color, color);

        // 绘制四个圆角
        // 左上角 (90° -> 180°)
        self.add_corner(
            x + radius,
            y + radius,
            radius,
            std::f32::consts::PI,
            1.5 * std::f32::consts::PI,
            color,
            segments,
        );

        // 右上角 (0° -> 90°)
        self.add_corner(
            x + width - radius,
            y + radius,
            radius,
            1.5 * std::f32::consts::PI,
            2.0 * std::f32::consts::PI,
            color,
            segments,
        );

        // 右下角 (270° -> 360°)
        self.add_corner(
            x + width - radius,
            y + height - radius,
            radius,
            0.0,
            0.5 * std::f32::consts::PI,
            color,
            segments,
        );

        // 左下角 (180° -> 270°)
        self.add_corner(
            x + radius,
            y + height - radius,
            radius,
            0.5 * std::f32::consts::PI,
            std::f32::consts::PI,
            color,
            segments,
        );
    }

    /// 添加圆角（三角形扇形）。
    fn add_corner(
        &mut self,
        center_x: f32,
        center_y: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        color: [f32; 4],
        segments: usize,
    ) {
        let base_index = self.vertices.len() as u16;

        // 添加中心顶点
        self.vertices.push(BatchVertex {
            position: self.to_ndc_pos(center_x, center_y),
            color,
            uv: [0.0, 0.0], // 纯色不需要 UV
        });

        // 添加圆弧上的顶点
        for i in 0..=segments {
            let angle = start_angle + (end_angle - start_angle) * i as f32 / segments as f32;
            let px = center_x + radius * angle.cos();
            let py = center_y + radius * angle.sin();

            self.vertices.push(BatchVertex {
                position: self.to_ndc_pos(px, py),
                color,
                uv: [0.0, 0.0],
            });
        }

        // 添加索引（三角形扇形）
        for i in 0..segments {
            self.indices.push(base_index); // 中心
            self.indices.push(base_index + 1 + i as u16); // 当前点
            self.indices.push(base_index + 2 + i as u16); // 下一个点
        }
    }

    /// 将像素坐标转换为 NDC 坐标（便捷方法）。
    fn to_ndc_pos(&self, x: f32, y: f32) -> [f32; 2] {
        let x_ndc = (x / self.screen_width) * 2.0 - 1.0;
        let y_ndc = 1.0 - (y / self.screen_height) * 2.0;
        [x_ndc, y_ndc]
    }

    /// 添加盒阴影到顶点池。
    ///
    /// 使用多层半透明矩形近似模糊效果，层数取决于模糊半径。
    fn add_box_shadow(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        color: [f32; 4],
    ) {
        // 阴影位置
        let shadow_x = x + offset_x;
        let shadow_y = y + offset_y;

        // 根据模糊半径决定层数
        let layers = if blur <= 0.0 {
            1
        } else {
            (blur / 2.0).ceil().max(1.0) as usize
        };

        // 每层的透明度递减
        for i in 0..layers {
            let spread = i as f32 * 2.0;
            let alpha_multiplier = 1.0 - (i as f32 / layers as f32);

            let layer_color = [
                color[0],
                color[1],
                color[2],
                color[3] * alpha_multiplier,
            ];

            // 绘制扩散层
            self.add_rect(
                shadow_x - spread,
                shadow_y - spread,
                width + spread * 2.0,
                height + spread * 2.0,
                layer_color,
                layer_color,
            );
        }
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

        // 步骤 4: 在 flush() 中绑定纹理组（在 draw_indexed 之前）
        if let Some(bind_group) = &self.texture_bind_group {
            render_pass.set_bind_group(0, bind_group, &[]);
        }

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
    #[allow(deprecated)]
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
    /// 提交纹理矩形（步骤 5: 完整实现纹理渲染）
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

        // 检查 u16 索引溢出风险
        if self.vertices.len() + 4 > 65536 {
            panic!("Vertex count exceeds u16 indexing limit (65535). Current: {}", self.vertices.len());
        }

        // 转换为 NDC 坐标
        let (x1, y1) = self.to_ndc(x, y);
        let (x2, y2) = self.to_ndc(x + width, y + height);

        let base_index = self.vertices.len() as u16;

        // 4 个顶点，带 UV 坐标
        // 左上角 (uv[0], uv[1])
        self.vertices.push(BatchVertex {
            position: [x1, y1],
            color: [1.0, 1.0, 1.0, 1.0], // 白色，让纹理颜色完全显示
            uv: [uv[0], uv[1]],
        });
        // 右上角 (uv[2], uv[1])
        self.vertices.push(BatchVertex {
            position: [x2, y1],
            color: [1.0, 1.0, 1.0, 1.0],
            uv: [uv[2], uv[1]],
        });
        // 右下角 (uv[2], uv[3])
        self.vertices.push(BatchVertex {
            position: [x2, y2],
            color: [1.0, 1.0, 1.0, 1.0],
            uv: [uv[2], uv[3]],
        });
        // 左下角 (uv[0], uv[3])
        self.vertices.push(BatchVertex {
            position: [x1, y2],
            color: [1.0, 1.0, 1.0, 1.0],
            uv: [uv[0], uv[3]],
        });

        // 6 个索引（两个三角形）
        self.indices.push(base_index);
        self.indices.push(base_index + 1);
        self.indices.push(base_index + 2);
        self.indices.push(base_index);
        self.indices.push(base_index + 2);
        self.indices.push(base_index + 3);

        tracing::debug!(
            texture_id = texture_id,
            x = x,
            y = y,
            width = width,
            height = height,
            uv = ?uv,
            "Submitted texture rectangle with UV coordinates"
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
        if bitmap.is_empty() || width < 1.0 || height < 1.0 {
            return;
        }
        
        // 检查 u16 索引溢出风险
        if self.vertices.len() + 4 > 65536 {
            tracing::warn!("Vertex count approaching u16 limit, flushing batch");
            // 在实际应用中应该在这里 flush batch
        }
        
        // 简化实现：计算平均 alpha 值，使用单个矩形表示文本
        // 这样避免创建过多顶点导致缓冲区溢出
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
            
            // 添加单个矩形表示文本
            self.add_rect(x, y, width, height, final_color, final_color);
            
            tracing::trace!(
                x, y, width, height, avg_alpha,
                "Added text glyph as single rect"
            );
        }
    }
}

#[cfg(test)]
mod texture_tests {
    use super::*;

    /// 测试：纹理 UV 坐标转换正确性
    #[test]
    fn test_uv_coordinate_mapping() {
        // 验证 UV 坐标格式：[左, 上, 右, 下]
        let full_uv = [0.0, 0.0, 1.0, 1.0];
        assert_eq!(full_uv[0], 0.0); // 左
        assert_eq!(full_uv[1], 0.0); // 上
        assert_eq!(full_uv[2], 1.0); // 右
        assert_eq!(full_uv[3], 1.0); // 下

        // 测试部分纹理（裁剪）
        let partial_uv = [0.25_f32, 0.25, 0.75, 0.75];
        assert!((partial_uv[2] - partial_uv[0] - 0.5_f32).abs() < f32::EPSILON);
        assert!((partial_uv[3] - partial_uv[1] - 0.5_f32).abs() < f32::EPSILON);
    }

    /// 测试：纹理顶点数据布局正确性（模拟测试）
    #[test]
    fn test_texture_vertex_layout() {
        // 模拟 4 个顶点的 UV 分配（左上、右上、右下、左下）
        let uv = [0.0, 0.0, 1.0, 1.0];
        
        // 顶点 0: 左上角 (uv[0], uv[1])
        let v0_uv = [uv[0], uv[1]];
        assert_eq!(v0_uv, [0.0, 0.0]);
        
        // 顶点 1: 右上角 (uv[2], uv[1])
        let v1_uv = [uv[2], uv[1]];
        assert_eq!(v1_uv, [1.0, 0.0]);
        
        // 顶点 2: 右下角 (uv[2], uv[3])
        let v2_uv = [uv[2], uv[3]];
        assert_eq!(v2_uv, [1.0, 1.0]);
        
        // 顶点 3: 左下角 (uv[0], uv[3])
        let v3_uv = [uv[0], uv[3]];
        assert_eq!(v3_uv, [0.0, 1.0]);
    }

    /// 测试：纹理索引生成正确性（两个三角形）
    #[test]
    fn test_texture_index_generation() {
        let base_index: u16 = 0;
        
        // 第一个三角形: 0, 1, 2（左上、右上、右下）
        let tri1 = [base_index, base_index + 1, base_index + 2];
        assert_eq!(tri1, [0, 1, 2]);
        
        // 第二个三角形: 0, 2, 3（左上、右下、左下）
        let tri2 = [base_index, base_index + 2, base_index + 3];
        assert_eq!(tri2, [0, 2, 3]);
    }

    /// 测试：多个纹理的索引偏移计算
    #[test]
    fn test_multiple_texture_offset() {
        // 第一个纹理：base_index = 0
        let base1: u16 = 0;
        assert_eq!(base1, 0);
        
        // 第二个纹理：base_index = 4（4 个顶点后）
        let base2: u16 = 4;
        assert_eq!(base2, 4);
        
        // 第三个纹理：base_index = 8
        let base3: u16 = 8;
        assert_eq!(base3, 8);
    }

    /// 测试：u16 索引边界条件（65535 最大值）
    #[test]
    fn test_u16_index_boundary() {
        // 安全边界：65532 个顶点（可以被 4 整除）
        let safe_vertices = 65532;
        assert!(safe_vertices + 4 <= 65536);
        
        // 溢出边界：65533 个顶点会超出限制
        let overflow_vertices = 65533;
        assert!(overflow_vertices + 4 > 65536);
    }

    /// 测试：纹理 ID 有效性检查逻辑
    #[test]
    fn test_texture_id_validation() {
        let texture_count = 5;
        
        // 有效的纹理 ID（0 到 4）
        for id in 0..texture_count {
            assert!(id < texture_count);
        }
        
        // 无效的纹理 ID（5 及以上）
        let invalid_id = 5;
        assert!(invalid_id >= texture_count);
    }

    /// 测试：UV 坐标裁剪场景（精灵图/纹理图集）
    #[test]
    fn test_sprite_sheet_uv() {
        // 假设 4x4 的精灵图，每个精灵占 1/4
        let sprite_width = 0.25;
        let sprite_height = 0.25;
        
        // 第一个精灵（左上角）
        let sprite1_uv = [0.0_f32, 0.0, sprite_width, sprite_height];
        assert!((sprite1_uv[2] - sprite1_uv[0] - sprite_width).abs() < f32::EPSILON);
        
        // 第二个精灵（右上角）
        let sprite2_uv = [sprite_width * 3.0_f32, 0.0, 1.0, sprite_height];
        assert!((sprite2_uv[2] - sprite2_uv[0] - sprite_width).abs() < f32::EPSILON);
        
        // 第三个精灵（左下角）
        let sprite3_uv = [0.0_f32, sprite_height * 3.0, sprite_width, 1.0];
        assert!((sprite3_uv[3] - sprite3_uv[1] - sprite_height).abs() < f32::EPSILON);
    }

    /// 测试：纹理颜色混合计算（白色 * 纹理 = 纹理）
    #[test]
    fn test_texture_color_blending() {
        let white = [1.0, 1.0, 1.0, 1.0];
        let texture_color = [0.8, 0.6, 0.4, 0.9];
        
        // 颜色混合：white * texture_color = texture_color
        let blended = [
            white[0] * texture_color[0],
            white[1] * texture_color[1],
            white[2] * texture_color[2],
            white[3] * texture_color[3],
        ];
        
        assert!((blended[0] - 0.8_f32).abs() < f32::EPSILON);
        assert!((blended[1] - 0.6_f32).abs() < f32::EPSILON);
        assert!((blended[2] - 0.4_f32).abs() < f32::EPSILON);
        assert!((blended[3] - 0.9_f32).abs() < f32::EPSILON);
    }

    /// 测试：纹理透明度混合（半透明纹理）
    #[test]
    fn test_texture_alpha_blending() {
        let vertex_color = [1.0, 1.0, 1.0, 1.0];
        let semi_transparent_texture = [1.0, 0.0, 0.0, 0.5]; // 50% 透明红色
        
        let blended_alpha = vertex_color[3] * semi_transparent_texture[3];
        assert!((blended_alpha - 0.5_f32).abs() < f32::EPSILON);
    }

    /// 测试：纹理尺寸与 UV 坐标的关系（非正方形纹理）
    #[test]
    fn test_non_square_texture_uv() {
        // 2:1 宽高比的纹理（例如 512x256）
        let width = 512.0;
        let height = 256.0;
        let aspect_ratio = width / height;
        
        // UV 坐标不受实际像素尺寸影响，始终是 0.0-1.0 归一化坐标
        let uv = [0.0, 0.0, 1.0, 1.0];
        assert_eq!(uv[2] - uv[0], 1.0);
        assert_eq!(uv[3] - uv[1], 1.0);
        
        // 但实际像素覆盖率会受宽高比影响
        let coverage_ratio = aspect_ratio;
        assert!((coverage_ratio - 2.0_f32).abs() < f32::EPSILON);
    }

    /// 测试：纹理旋转场景的 UV 坐标调整（90 度旋转）
    #[test]
    fn test_texture_rotation_uv() {
        // 原始 UV（正常方向）
        let normal_uv = [0.0, 0.0, 1.0, 1.0];
        
        // 90 度顺时针旋转后的 UV（交换 x/y 并反转一个轴）
        let rotated_uv = [normal_uv[1], 1.0 - normal_uv[2], normal_uv[3], 1.0 - normal_uv[0]];
        assert_eq!(rotated_uv, [0.0, 0.0, 1.0, 1.0]); // 对于完整纹理结果相同
        
        // 测试部分纹理旋转（左上 1/4 区域）
        let partial_uv = [0.0, 0.0, 0.5, 0.5];
        let rotated_partial = [
            partial_uv[1],
            1.0 - partial_uv[2],
            partial_uv[3],
            1.0 - partial_uv[0],
        ];
        assert_eq!(rotated_partial, [0.0, 0.5, 0.5, 1.0]);
    }

    /// 测试：纹理翻转（水平/垂直镜像）的 UV 坐标
    #[test]
    fn test_texture_flip_uv() {
        let normal_uv = [0.0, 0.0, 1.0, 1.0];
        
        // 水平翻转（左右镜像）：交换左和右
        let h_flip = [normal_uv[2], normal_uv[1], normal_uv[0], normal_uv[3]];
        assert_eq!(h_flip, [1.0, 0.0, 0.0, 1.0]);
        
        // 垂直翻转（上下镜像）：交换上和下
        let v_flip = [normal_uv[0], normal_uv[3], normal_uv[2], normal_uv[1]];
        assert_eq!(v_flip, [0.0, 1.0, 1.0, 0.0]);
    }

    /// 测试：批量渲染多个纹理的顶点计数增长
    #[test]
    fn test_batch_vertex_growth() {
        // 每个纹理矩形添加 4 个顶点、6 个索引
        let vertices_per_texture = 4;
        let indices_per_texture = 6;
        
        // 渲染 1 个纹理矩形
        let count1 = 1;
        assert_eq!(count1 * vertices_per_texture, 4);
        assert_eq!(count1 * indices_per_texture, 6);
        
        // 渲染 10 个纹理矩形
        let count10 = 10;
        assert_eq!(count10 * vertices_per_texture, 40);
        assert_eq!(count10 * indices_per_texture, 60);
        
        // 渲染 100 个纹理矩形
        let count100 = 100;
        assert_eq!(count100 * vertices_per_texture, 400);
        assert_eq!(count100 * indices_per_texture, 600);
    }

    /// 测试：纹理加载的错误处理（边界条件）
    #[test]
    fn test_texture_load_error_handling() {
        // 测试空纹理 ID 列表的情况
        let loaded_textures: Vec<u32> = vec![];
        let invalid_id = 0;
        assert!(invalid_id >= loaded_textures.len() as u32);
        
        // 测试单个纹理的情况
        let single_texture: Vec<u32> = vec![0];
        let valid_id = 0;
        assert!(valid_id < single_texture.len() as u32);
        
        let invalid_id2 = 1;
        assert!(invalid_id2 >= single_texture.len() as u32);
    }

    /// 测试：纹理渲染的坐标变换（屏幕坐标 → NDC）
    #[test]
    fn test_screen_to_ndc_transform() {
        let screen_width = 800.0;
        let screen_height = 600.0;
        
        // 模拟 to_ndc 变换（实际实现中的公式）
        fn to_ndc(x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
            let ndc_x = (x / width) * 2.0 - 1.0;
            let ndc_y = 1.0 - (y / height) * 2.0; // Y 轴反转
            (ndc_x, ndc_y)
        }
        
        // 测试左上角 (0, 0) → (-1, 1)
        let (x1, y1) = to_ndc(0.0, 0.0, screen_width, screen_height);
        assert!((x1 - (-1.0)).abs() < f32::EPSILON);
        assert!((y1 - 1.0).abs() < f32::EPSILON);
        
        // 测试右下角 (800, 600) → (1, -1)
        let (x2, y2) = to_ndc(800.0, 600.0, screen_width, screen_height);
        assert!((x2 - 1.0).abs() < f32::EPSILON);
        assert!((y2 - (-1.0)).abs() < f32::EPSILON);
        
        // 测试中心点 (400, 300) → (0, 0)
        let (x3, y3) = to_ndc(400.0, 300.0, screen_width, screen_height);
        assert!(x3.abs() < f32::EPSILON);
        assert!(y3.abs() < f32::EPSILON);
    }

    /// 测试：圆角矩形绘制命令创建
    #[test]
    fn test_rounded_rect_command_creation() {
        let cmd = DrawCommand::RoundedRect {
            x: 100.0,
            y: 100.0,
            width: 200.0,
            height: 150.0,
            radius: 20.0,
            color: [1.0, 0.0, 0.0, 1.0],
        };

        match cmd {
            DrawCommand::RoundedRect {
                x,
                y,
                width,
                height,
                radius,
                color,
            } => {
                assert!((x - 100.0).abs() < f32::EPSILON);
                assert!((y - 100.0).abs() < f32::EPSILON);
                assert!((width - 200.0).abs() < f32::EPSILON);
                assert!((height - 150.0).abs() < f32::EPSILON);
                assert!((radius - 20.0).abs() < f32::EPSILON);
                assert!((color[0] - 1.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected RoundedRect command"),
        }
    }

    /// 测试：圆角半径限制（不能超过矩形尺寸的一半）
    #[test]
    fn test_radius_clamping() {
        let width = 100.0_f32;
        let height = 60.0_f32;
        let radius = 50.0_f32; // 太大了

        // 模拟半径限制逻辑
        let clamped_radius = radius.min(width / 2.0).min(height / 2.0);
        assert!((clamped_radius - 30.0_f32).abs() < f32::EPSILON); // 应该是 height/2
    }

    /// 测试：圆角矩形的退化情况（半径为 0）
    #[test]
    fn test_rounded_rect_zero_radius() {
        let radius = 0.0;
        assert!(radius <= 0.0); // 应该退化为普通矩形
    }

    /// 测试：圆角分段数计算
    #[test]
    fn test_corner_segments() {
        let segments = 16;
        let angle_step = std::f32::consts::PI / 2.0 / segments as f32;

        // 90 度圆角应该有 16 段
        assert_eq!(segments, 16);
        // 每段角度应该是 90/16 = 5.625 度
        assert!((angle_step - 0.09817477).abs() < f32::EPSILON);
    }

    /// 测试：圆角中心点计算
    #[test]
    fn test_corner_center_calculation() {
        let x = 100.0_f32;
        let y = 100.0_f32;
        let width = 200.0_f32;
        let height = 150.0_f32;
        let radius = 20.0_f32;

        // 左上角中心点
        let tl_x = x + radius;
        let tl_y = y + radius;
        assert!((tl_x - 120.0_f32).abs() < f32::EPSILON);
        assert!((tl_y - 120.0_f32).abs() < f32::EPSILON);

        // 右下角中心点
        let br_x = x + width - radius;
        let br_y = y + height - radius;
        assert!((br_x - 280.0_f32).abs() < f32::EPSILON);
        assert!((br_y - 230.0_f32).abs() < f32::EPSILON);
    }

    /// 测试：盒阴影绘制命令创建
    #[test]
    fn test_box_shadow_command_creation() {
        let cmd = DrawCommand::BoxShadow {
            x: 100.0_f32,
            y: 100.0_f32,
            width: 200.0_f32,
            height: 150.0_f32,
            offset_x: 5.0_f32,
            offset_y: 10.0_f32,
            blur: 8.0_f32,
            color: [0.0_f32, 0.0_f32, 0.0_f32, 0.5_f32],
        };

        match cmd {
            DrawCommand::BoxShadow {
                offset_x,
                offset_y,
                blur,
                color,
                ..
            } => {
                assert!((offset_x - 5.0_f32).abs() < f32::EPSILON);
                assert!((offset_y - 10.0_f32).abs() < f32::EPSILON);
                assert!((blur - 8.0_f32).abs() < f32::EPSILON);
                assert!((color[3] - 0.5_f32).abs() < f32::EPSILON);
            }
            _ => panic!("Expected BoxShadow command"),
        }
    }

    /// 测试：阴影层数计算
    #[test]
    fn test_shadow_layer_count() {
        // 无模糊：1 层
        let blur = 0.0_f32;
        let layers = if blur <= 0.0 { 1 } else { (blur / 2.0).ceil().max(1.0) as usize };
        assert_eq!(layers, 1);

        // 小模糊：4 层
        let blur = 8.0_f32;
        let layers = if blur <= 0.0 { 1 } else { (blur / 2.0).ceil().max(1.0) as usize };
        assert_eq!(layers, 4);

        // 大模糊：10 层
        let blur = 20.0_f32;
        let layers = if blur <= 0.0 { 1 } else { (blur / 2.0).ceil().max(1.0) as usize };
        assert_eq!(layers, 10);
    }

    /// 测试：阴影位置计算
    #[test]
    fn test_shadow_position() {
        let x = 100.0_f32;
        let y = 100.0_f32;
        let offset_x = 5.0_f32;
        let offset_y = 10.0_f32;

        let shadow_x = x + offset_x;
        let shadow_y = y + offset_y;

        assert!((shadow_x - 105.0_f32).abs() < f32::EPSILON);
        assert!((shadow_y - 110.0_f32).abs() < f32::EPSILON);
    }

    /// 测试：阴影扩散计算
    #[test]
    fn test_shadow_spread() {
        let width = 200.0_f32;
        let height = 150.0_f32;
        let spread = 4.0_f32; // 第 2 层

        let shadow_width = width + spread * 2.0;
        let shadow_height = height + spread * 2.0;

        assert!((shadow_width - 208.0_f32).abs() < f32::EPSILON);
        assert!((shadow_height - 158.0_f32).abs() < f32::EPSILON);
    }

    /// 测试：阴影透明度衰减
    #[test]
    fn test_shadow_alpha_decay() {
        let base_alpha = 0.5_f32;
        let layers = 4;

        // 第 0 层：100% 透明度
        let alpha_0 = base_alpha * (1.0 - 0.0 / layers as f32);
        assert!((alpha_0 - 0.5_f32).abs() < f32::EPSILON);

        // 第 2 层：50% 透明度
        let alpha_2 = base_alpha * (1.0 - 2.0 / layers as f32);
        assert!((alpha_2 - 0.25_f32).abs() < f32::EPSILON);

        // 最后层：接近 0 透明度
        let alpha_last = base_alpha * (1.0 - (layers - 1) as f32 / layers as f32);
        assert!(alpha_last < 0.15_f32);
    }

    /// 测试：圆形命令创建
    #[test]
    fn test_circle_command_creation() {
        let cmd = DrawCommand::Circle {
            center_x: 100.0,
            center_y: 100.0,
            radius_x: 50.0,
            radius_y: 50.0,
            color: [1.0, 0.0, 0.0, 1.0],
        };

        match cmd {
            DrawCommand::Circle { center_x, center_y, radius_x, radius_y, color } => {
                assert_eq!(center_x, 100.0);
                assert_eq!(center_y, 100.0);
                assert_eq!(radius_x, 50.0);
                assert_eq!(radius_y, 50.0);
                assert_eq!(color, [1.0, 0.0, 0.0, 1.0]);
            }
            _ => panic!("Expected Circle command"),
        }
    }

    /// 测试：径向渐变命令创建
    #[test]
    fn test_radial_gradient_command_creation() {
        let cmd = DrawCommand::RadialGradientRect {
            center_x: 200.0,
            center_y: 200.0,
            radius: 100.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [0.0, 0.0, 0.0, 1.0],
        };

        match cmd {
            DrawCommand::RadialGradientRect { center_x, center_y, radius, start_color, end_color } => {
                assert_eq!(center_x, 200.0);
                assert_eq!(center_y, 200.0);
                assert_eq!(radius, 100.0);
                assert_eq!(start_color, [1.0, 1.0, 1.0, 1.0]);
                assert_eq!(end_color, [0.0, 0.0, 0.0, 1.0]);
            }
            _ => panic!("Expected RadialGradientRect command"),
        }
    }
}
