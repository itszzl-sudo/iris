//! Iris GPU 2D 批渲染系统
//!
//! 合并多次 2D 绘制调用为单次 GPU draw call，支持：
//! - 纯色矩形
//! - 线性渐变（水平/垂直）
//! - Alpha 混合
//! - 纹理贴图（预留）

use bytemuck::{Pod, Zeroable};

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
}
