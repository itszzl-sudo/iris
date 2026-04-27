//! Iris GPU —— WebGPU 硬件渲染管线 + 简化字体渲染
//!
//! 基于标准 WebGPU 规范（wgpu 24.x），统一桌面 / 浏览器渲染接口。
//! 后端自动探测：Vulkan（Windows/Linux）/ Metal（macOS）/ DX12（Windows）/ WebGPU（Wasm）。
//! 当前实现：可编程渲染管线（三角形/矩形）+ fontdue 字体渲染（CPU 光栅化 → GPU 纹理）。

#![warn(missing_docs)]

mod batch_renderer;
mod file_watcher;
mod font_atlas;
mod texture_cache;
mod text_renderer;
mod canvas;

pub use batch_renderer::{BatchRenderer, BatchVertex, DrawCommand};
pub use font_atlas::{FontAtlas, GlyphInfo};
pub use texture_cache::{TextureCache, TextureEntry};
pub use text_renderer::TextRenderer;
pub use canvas::Canvas2DContext;
use bytemuck::{Pod, Zeroable};
pub use file_watcher::{deduplicate_changes, FileChange, FileWatcher, WatcherConfig};
use wgpu::util::DeviceExt;
use winit::window::Window;

/// 顶点数据：位置 (x, y) + 颜色 (r, g, b, a)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    /// 生成 wgpu 顶点缓冲区布局描述。
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
            ],
        }
    }
}

/// WGSL 着色器代码。
/// 顶点着色器：传递位置与颜色到片段着色器。
/// 片段着色器：输出插值后的颜色。
const SHADER_SOURCE: &str = r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(position, 0.0, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

/// Iris GPU 渲染器。
///
/// 封装 wgpu 实例、表面、设备、队列、渲染管线、顶点/索引缓冲区，
/// 提供极简的每帧渲染接口。
pub struct Renderer {
    // SAFETY: `surface` 引用了 `window` 的底层 OS 句柄。
    // `surface` 字段必须排在 `window` 之前，确保 drop 时先释放 surface。
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,

    // 几何体渲染（旧版，保留用于测试）
    #[allow(dead_code)]
    render_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    vertex_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    index_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    triangle_vertices: u32,
    #[allow(dead_code)]
    rect_indices: u32,

    // 2D 批渲染系统（新版）
    batch_renderer: BatchRenderer,

    // 纹理缓存
    texture_cache: TextureCache,

    // 文件热更新监听器（可选）
    file_watcher: Option<FileWatcher>,
}

impl Renderer {
    /// 从 winit 窗口异步初始化 GPU 渲染器。
    ///
    /// 接收窗口所有权，确保 surface 引用的句柄在整个 Renderer 生命周期内有效。
    /// 上层通常通过 [`iris_core::Context::block_on`] 在主线程同步调用。
    pub async fn new(window: Window) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // 临时借用创建 surface，随后通过 unsafe 延长生命周期为 'static。
        // 这是安全的，因为 Renderer 拥有 window 的所有权，且 surface 先被 drop。
        let surface = instance.create_surface(&window)?;
        let surface =
            unsafe { std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find appropriate GPU adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("Iris GPU Device"),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await?;

        let size = window.inner_size();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        // ========== 创建着色器模块 ==========
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Iris Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        // ========== 创建渲染管线 ==========
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Iris Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Iris Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
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

        // ========== 创建顶点缓冲区（三角形 + 矩形）==========
        let vertices: &[Vertex] = &[
            // 三角形顶点（RGB 渐变，居中显示）
            // 0: 顶部 - 红色
            Vertex {
                position: [0.0, 0.5],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            // 1: 左下 - 绿色
            Vertex {
                position: [-0.5, -0.1],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            // 2: 右下 - 蓝色
            Vertex {
                position: [0.5, -0.1],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            // 矩形顶点（左上到右下渐变，右半部分显示）
            // 3: 左上 - 青色
            Vertex {
                position: [0.2, 0.0],
                color: [0.0, 1.0, 1.0, 1.0],
            },
            // 4: 右上 - 品红
            Vertex {
                position: [0.9, 0.0],
                color: [1.0, 0.0, 1.0, 1.0],
            },
            // 5: 左下 - 黄色
            Vertex {
                position: [0.2, -0.6],
                color: [1.0, 1.0, 0.0, 1.0],
            },
            // 6: 右下 - 白色
            Vertex {
                position: [0.9, -0.6],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // ========== 创建索引缓冲区（矩形使用 2 个三角形）==========
        // 矩形索引：[3,4,5] + [5,4,6] = 两个三角形拼成矩形
        let indices: &[u16] = &[3, 4, 5, 5, 4, 6];

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let triangle_vertices = 3; // 三角形使用顶点 0-2
        let rect_indices = 6; // 矩形使用 6 个索引（2 个三角形）

        // 初始化 2D 批渲染系统（容量 1024 个矩形）
        let batch_renderer = BatchRenderer::new(
            &device,
            &queue,
            surface_format,
            size.width as f32,
            size.height as f32,
            1024,
        );

        // 初始化纹理缓存
        let texture_cache = TextureCache::new(surface_format);

        println!("✅ Iris GPU renderer initialized (batch renderer + texture cache ready)");

        Ok(Self {
            surface,
            device,
            queue,
            surface_config,
            size,
            window,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            triangle_vertices,
            rect_indices,
            batch_renderer,
            texture_cache,
            file_watcher: None, // 默认不启动，需要时手动设置
        })
    }

    /// 启动文件热更新监听器。
    ///
    /// # 参数
    ///
    /// * `config` - 监听器配置
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use iris_gpu::{Renderer, WatcherConfig};
    ///
    /// // let mut renderer = Renderer::new(window).await.unwrap();
    /// // renderer.start_file_watcher(
    /// //     WatcherConfig::new("./src")
    /// //         .extensions(vec!["vue".to_string(), "js".to_string(), "css".to_string()])
    /// // );
    /// ```
    pub fn start_file_watcher(&mut self, config: WatcherConfig) {
        match FileWatcher::new(config) {
            Ok(watcher) => {
                self.file_watcher = Some(watcher);
                println!("✅ File watcher started");
            }
            Err(e) => {
                eprintln!("⚠️ Failed to start file watcher: {}", e);
            }
        }
    }

    /// 检查并处理文件变更事件（非阻塞）。
    ///
    /// 应在每帧渲染时调用，或在事件循环中定期调用。
    /// 自动去重：同一文件的多次变更只保留最后一次。
    pub fn poll_file_changes(&mut self) -> Vec<FileChange> {
        let mut changes = Vec::new();

        if let Some(watcher) = &mut self.file_watcher {
            // 非阻塞接收所有待处理的事件
            while let Some(change) = watcher.try_recv() {
                changes.push(change);
            }
        }

        // 去重：同一文件的多次变更只保留最后一次
        if !changes.is_empty() {
            let deduplicated = file_watcher::deduplicate_changes(changes);

            // 打印去重后的事件
            for change in &deduplicated {
                println!(
                    "🔥 File change detected: {:?} ({})",
                    change.path(),
                    change.extension().unwrap_or("unknown")
                );
            }

            deduplicated
        } else {
            Vec::new()
        }
    }

    /// 获取窗口引用。
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// 窗口大小变化时重新配置表面。
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    /// 渲染一帧。
    ///
    /// 执行流程：
    /// 1. 清屏为 Iris 品牌底色（Retina Dark #0D0D12）
    /// 2. 使用批渲染系统绘制多个矩形（单次 draw call）
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Iris Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Iris Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.051,
                            g: 0.051,
                            b: 0.071,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // ========== 使用批渲染系统绘制 ==========
            // 示例 1: 紫色背景矩形（Iris Violet）
            self.batch_renderer.submit(DrawCommand::Rect {
                x: 100.0,
                y: 100.0,
                width: 300.0,
                height: 200.0,
                color: [0.4196, 0.3059, 0.9020, 1.0], // #6B4EE6
            });

            // 示例 2: 水平渐变矩形（青色 → 品红）
            self.batch_renderer.submit(DrawCommand::GradientRect {
                x: 500.0,
                y: 100.0,
                width: 300.0,
                height: 200.0,
                start_color: [0.0, 0.831, 0.667, 1.0], // #00D4AA
                end_color: [1.0, 0.0, 1.0, 1.0],
                horizontal: true,
            });

            // 示例 3: 垂直渐变矩形（光谱金 → Iris Violet）
            self.batch_renderer.submit(DrawCommand::GradientRect {
                x: 900.0,
                y: 100.0,
                width: 300.0,
                height: 200.0,
                start_color: [1.0, 0.843, 0.0, 1.0], // #FFD700
                end_color: [0.4196, 0.3059, 0.9020, 1.0],
                horizontal: false,
            });

            // 示例 4: 半透明矩形（Alpha 混合测试）
            self.batch_renderer.submit(DrawCommand::Rect {
                x: 200.0,
                y: 350.0,
                width: 400.0,
                height: 150.0,
                color: [0.0, 0.831, 0.667, 0.5], // 50% 透明度
            });

            // 示例 5: 小型 UI 元素模拟（多个按钮）
            for i in 0..5 {
                self.batch_renderer.submit(DrawCommand::Rect {
                    x: 100.0 + i as f32 * 150.0,
                    y: 550.0,
                    width: 120.0,
                    height: 50.0,
                    color: [
                        0.4196 + i as f32 * 0.1,
                        0.3059,
                        0.9020 - i as f32 * 0.1,
                        1.0,
                    ],
                });
            }

            // 示例 6: 圆形（使用 Circle 命令）
            self.batch_renderer.submit(DrawCommand::Circle {
                center_x: 1000.0,
                center_y: 300.0,
                radius_x: 50.0,
                radius_y: 50.0,
                color: [1.0, 0.843, 0.0, 0.8], // 金色半透明
            });

            // 示例 7: 径向渐变（使用 RadialGradientRect）
            self.batch_renderer.submit(DrawCommand::RadialGradientRect {
                center_x: 1150.0,
                center_y: 300.0,
                radius: 60.0,
                start_color: [0.0, 0.831, 0.667, 1.0], // 中心：青色
                end_color: [0.4196, 0.3059, 0.9020, 1.0], // 边缘：紫色
            });

            // 刷新批渲染（单次 draw call 提交所有矩形）
            let count = self.batch_renderer.draw_count();
            self.batch_renderer.flush(&mut render_pass);
            println!("🎨 Batch rendered {} rectangles", count);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// 提交绘制命令到批渲染器
    ///
    /// # 参数
    ///
    /// * `command` - 绘制命令
    pub fn submit_command(&mut self, command: DrawCommand) {
        self.batch_renderer.submit(command);
    }

    /// 批量提交绘制命令
    ///
    /// # 参数
    ///
    /// * `commands` - 绘制命令列表
    pub fn submit_commands(&mut self, commands: Vec<DrawCommand>) {
        for command in commands {
            self.batch_renderer.submit(command);
        }
    }

    /// 获取当前渲染尺寸。
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    /// 从 RGBA 数据加载纹理。
    ///
    /// # 参数
    ///
    /// * `data` - RGBA 像素数据
    /// * `width` - 纹理宽度
    /// * `height` - 纹理高度
    ///
    /// # 返回
    ///
    /// 返回纹理 ID
    pub fn load_texture_from_rgba(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<u32, String> {
        self.texture_cache
            .create_texture_from_rgba(&self.device, &self.queue, data, width, height)
    }

    /// 从文件路径加载纹理。
    ///
    /// # 参数
    ///
    /// * `path` - 图像文件路径
    ///
    /// # 返回
    ///
    /// 返回纹理 ID
    pub fn load_texture_from_path(&mut self, path: &str) -> Result<u32, String> {
        self.texture_cache
            .create_texture_from_path(&self.device, &self.queue, path)
    }

    /// 获取纹理缓存引用。
    pub fn texture_cache(&self) -> &TextureCache {
        &self.texture_cache
    }
}

/// 兼容旧 API 的初始化函数。
pub fn init() {
    println!("iris-gpu initialized");
}
