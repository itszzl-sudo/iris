//! 完整窗口示例：渲染第一个 Vue 组件
//!
//! 这个示例展示了如何：
//! 1. 创建 winit 窗口
//! 2. 初始化 GPU 渲染器
//! 3. 加载 Vue SFC 组件
//! 4. 实现事件循环
//! 5. 渲染到屏幕
//!
//! # 使用方法
//!
//! ```bash
//! cargo run --example gpu_render_window
//! ```
//!
//! # 预期效果
//!
//! 打开一个 800x600 的窗口，显示：
//! - 标题 "Hello Iris Engine!"
//! - 副标题 "GPU Rendering with Vue SFC"
//! - 背景色为渐变蓝色

use iris_engine::orchestrator::RuntimeOrchestrator;
use iris_gpu::Renderer;
use iris_layout::vdom::{VElement, VNode, VTree};
use tracing::{info, warn};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

/// 主应用程序状态
struct App {
    window: Option<Window>,
    orchestrator: RuntimeOrchestrator,
    renderer: Option<Renderer>,
    size: winit::dpi::PhysicalSize<u32>,
    suspended: bool,
    renderer_initialized: bool,
}

impl App {
    fn new() -> Self {
        info!("Creating application state...");
        
        // 1. 创建并初始化编排器
        let mut orchestrator = RuntimeOrchestrator::new();
        orchestrator.initialize().expect("Failed to initialize orchestrator");
        info!("RuntimeOrchestrator initialized");

        // 2. 创建示例 VTree（模拟 Vue SFC 编译结果）
        let vtree = create_sample_vtree();
        orchestrator.set_vtree(vtree);
        info!("Sample VTree created");

        // 3. 计算布局
        orchestrator.set_viewport_size(800.0, 600.0);
        orchestrator.compute_layout().expect("Failed to compute layout");
        info!("Layout computed");

        Self {
            window: None,
            orchestrator,
            renderer: None,
            size: winit::dpi::PhysicalSize::new(800, 600),
            suspended: false,
            renderer_initialized: false,
        }
    }

    /// 初始化 GPU 渲染器（同步版本，使用 pollster）
    fn init_renderer_sync(&mut self) {
        if self.renderer_initialized {
            return;
        }

        // 取出 window 用于创建渲染器
        if let Some(window) = self.window.take() {
            info!("Initializing GPU renderer (sync)...");
            
            // 使用 pollster 阻塞地等待异步初始化
            match pollster::block_on(Renderer::new(window)) {
                Ok(renderer) => {
                    self.renderer = Some(renderer);
                    self.renderer_initialized = true;
                    info!("✅ GPU renderer initialized successfully");
                    
                    // 标记需要渲染
                    self.orchestrator.mark_dirty();
                }
                Err(e) => {
                    warn!("❌ Failed to initialize GPU renderer: {}", e);
                    self.renderer_initialized = true; // 标记为已尝试，避免重复尝试
                }
            }
        }
    }

    /// 渲染一帧
    fn render(&mut self) {
        if self.suspended {
            return;
        }

        // 使用 GPU 渲染器渲染
        if self.renderer.is_some() {
            let rendered = self.orchestrator.render_frame_gpu();
            
            if rendered {
                info!("Frame rendered with GPU");
            }
        } else {
            // 没有 GPU 渲染器时，仍然生成命令用于调试
            let commands = self.orchestrator.generate_render_commands();
            if !commands.is_empty() {
                info!("Generated {} render commands (no GPU renderer)", commands.len());
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("Application resumed");
        self.suspended = false;

        // 创建窗口
        if self.window.is_none() {
            let window = event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Iris Engine - GPU Rendering with Vue SFC")
                        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600)),
                )
                .expect("Failed to create window");

            info!("Window created: 800x600");
            self.window = Some(window);
            self.size = winit::dpi::PhysicalSize::new(800, 600);

            // 同步初始化 GPU 渲染器
            self.init_renderer_sync();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        info!("Application suspended");
        self.suspended = true;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Close requested, exiting...");
                event_loop.exit();
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                info!("Escape pressed, exiting...");
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                info!("Window resized to {}x{}", new_size.width, new_size.height);
                self.size = new_size;

                // 更新编排器的视口尺寸
                self.orchestrator.set_viewport_size(
                    new_size.width as f32,
                    new_size.height as f32,
                );

                // 重新计算布局
                if let Err(e) = self.orchestrator.compute_layout() {
                    warn!("Failed to recompute layout: {}", e);
                }

                // 更新渲染器大小
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(new_size);
                }
            }

            WindowEvent::RedrawRequested => {
                // 渲染一帧
                self.render();

                // 请求下一帧（如果需要）
                if self.orchestrator.is_dirty() {
                    if let Some(ref window) = self.window {
                        window.request_redraw();
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    info!("Mouse button {:?} pressed", button);
                    // TODO: 处理鼠标点击事件
                    // self.orchestrator.handle_mouse_click(...);
                }
            }

            _ => {}
        }
    }
}

/// 创建示例 VTree
///
/// 这模拟了 Vue SFC 编译后的结果：
/// ```vue
/// <template>
///   <div id="app">
///     <h1>Hello Iris Engine!</h1>
///     <p>GPU Rendering with Vue SFC</p>
///   </div>
/// </template>
///
/// <style>
/// #app {
///   background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
///   display: flex;
///   flex-direction: column;
///   justify-content: center;
///   align-items: center;
///   padding: 40px;
/// }
/// h1 {
///   color: white;
///   font-size: 48px;
///   margin-bottom: 20px;
/// }
/// p {
///   color: rgba(255, 255, 255, 0.8);
///   font-size: 24px;
/// }
/// </style>
/// ```
fn create_sample_vtree() -> VTree {
    VTree {
        root: VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![
                ("id".to_string(), "app".to_string()),
                ("style".to_string(), "background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); display: flex; flex-direction: column; justify-content: center; align-items: center; padding: 40px;".to_string()),
            ].into_iter().collect(),
            children: vec![
                VNode::Element(VElement {
                    tag: "h1".to_string(),
                    attrs: vec![
                        ("style".to_string(), "color: white; font-size: 48px; margin-bottom: 20px; text-align: center;".to_string()),
                    ].into_iter().collect(),
                    children: vec![VNode::Text("Hello Iris Engine!".to_string())],
                    key: None,
                }),
                VNode::Element(VElement {
                    tag: "p".to_string(),
                    attrs: vec![
                        ("style".to_string(), "color: rgba(255, 255, 255, 0.8); font-size: 24px; text-align: center;".to_string()),
                    ].into_iter().collect(),
                    children: vec![VNode::Text("GPU Rendering with Vue SFC".to_string())],
                    key: None,
                }),
            ],
            key: None,
        }),
    }
}

fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Iris Engine - GPU Render Window Example");
    info!("===========================================");

    // 创建事件循环
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    // 创建应用程序状态
    let mut app = App::new();

    // 运行事件循环
    // 注意：这里需要使用 pollster 来处理异步初始化
    // 但由于 winit 的事件循环是同步的，我们在 resumed 事件中初始化
    
    // 运行事件循环
    event_loop.run_app(&mut app).expect("Failed to run event loop");

    info!("👋 Application exited successfully");
}
