//! 径向渐变窗口示例：在 Vue SFC 中使用径向渐变
//!
//! 这个示例展示了如何：
//! 1. 创建 winit 窗口
//! 2. 初始化 GPU 渲染器
//! 3. 加载径向渐变演示 Vue SFC
//! 4. 渲染各种径向渐变效果
//!
//! # 使用方法
//!
//! ```bash
//! cargo run --example radial_gradient_window
//! ```
//!
//! # 预期效果
//!
//! 打开一个 1200x900 的窗口，显示：
//! - 基础径向渐变卡片
//! - 多色标径向渐变
//! - 按钮、头像等应用场景
//! - 大型背景渐变效果

use iris_engine::orchestrator::RuntimeOrchestrator;
use iris_gpu::Renderer;
use iris_layout::vdom::{VElement, VNode, VTree};
use tracing::{info, warn};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// 径向渐变演示应用
struct RadialGradientApp {
    window: Option<Window>,
    orchestrator: RuntimeOrchestrator,
    size: winit::dpi::PhysicalSize<u32>,
    suspended: bool,
    renderer_initialized: bool,
}

impl RadialGradientApp {
    fn new() -> Self {
        info!("Creating radial gradient application...");
        
        // 1. 创建并初始化编排器
        let mut orchestrator = RuntimeOrchestrator::new();
        orchestrator.initialize().expect("Failed to initialize orchestrator");
        info!("RuntimeOrchestrator initialized");

        // 2. 尝试加载径向渐变 Vue SFC
        let vue_path = "crates/iris-engine/examples/radial_gradient_demo.vue";
        info!("Loading Vue SFC: {}", vue_path);
        
        match orchestrator.load_sfc_with_vtree(vue_path) {
            Ok(()) => {
                info!("✅ Radial gradient Vue SFC loaded successfully");
                if let Some(vtree) = orchestrator.vtree() {
                    info!("   VTree root: {:?}", std::mem::discriminant(&vtree.root));
                }
            }
            Err(e) => {
                warn!("⚠️  Failed to load Vue SFC: {}", e);
                warn!("   Creating fallback radial gradient VTree...");
                
                // 如果加载失败，使用示例 VTree（包含径向渐变）
                let vtree = create_radial_gradient_vtree();
                orchestrator.set_vtree(vtree);
            }
        }

        // 3. 计算布局
        orchestrator.set_viewport_size(1200.0, 900.0);
        if let Err(e) = orchestrator.compute_layout() {
            warn!("Failed to compute layout: {}", e);
        } else {
            info!("✅ Layout computed");
        }

        Self {
            window: None,
            orchestrator,
            size: winit::dpi::PhysicalSize::new(1200, 900),
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
            
            // 使用 pollster 将异步操作转为同步
            match pollster::block_on(iris_gpu::BatchRenderer::new(&window)) {
                Ok(mut renderer) => {
                    info!("✅ BatchRenderer created successfully");
                    renderer.set_window_size(self.size.width as f32, self.size.height as f32);
                    info!("   Window size set: {}x{}", self.size.width, self.size.height);
                    
                    // 将渲染器注入到 orchestrator
                    self.orchestrator.set_gpu_renderer(renderer);
                    info!("✅ GPU renderer injected into orchestrator");
                    
                    self.renderer_initialized = true;
                }
                Err(e) => {
                    warn!("❌ Failed to create BatchRenderer: {}", e);
                    warn!("   Will retry on next redraw...");
                }
            }
            
            // 放回 window
            self.window = Some(window);
        }
    }

    /// 渲染一帧
    fn render(&mut self) {
        // 确保渲染器已初始化
        if !self.renderer_initialized {
            self.init_renderer_sync();
            if !self.renderer_initialized {
                return;
            }
        }

        // 收集渲染命令并提交给 GPU
        let rendered = self.orchestrator.render_frame_gpu();
        
        if rendered {
            info!("Frame rendered with GPU");
        } else {
            // 没有 GPU 渲染器或未渲染时，生成命令用于调试
            let commands = self.orchestrator.generate_render_commands();
            if !commands.is_empty() {
                info!("Generated {} render commands (waiting for GPU renderer)", commands.len());
            }
        }
    }
}

impl ApplicationHandler for RadialGradientApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        info!("Application resumed");
        self.suspended = false;
        
        // 窗口已经创建，尝试初始化渲染器
        if self.window.is_some() {
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
            WindowEvent::Resized(new_size) => {
                info!("Window resized to {}x{}", new_size.width, new_size.height);
                self.size = new_size;
                
                // 更新视口大小
                self.orchestrator.set_viewport_size(new_size.width as f32, new_size.height as f32);
                
                // 重新计算布局
                if let Err(e) = self.orchestrator.compute_layout() {
                    warn!("Failed to recompute layout: {}", e);
                }
                
                // 请求重绘
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if !self.suspended {
                    self.render();
                }
            }
            _ => {}
        }
    }
}

/// 创建包含径向渐变的示例 VTree
fn create_radial_gradient_vtree() -> VTree {
    VTree {
        root: VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![
                ("style".to_string(), "background: #1a1a2e; min-height: 100vh; padding: 40px; display: flex; flex-direction: column; align-items: center;".to_string()),
            ].into_iter().collect(),
            children: vec![
                // 标题
                VNode::Element(VElement {
                    tag: "h1".to_string(),
                    attrs: vec![
                        ("style".to_string(), "color: white; font-size: 36px; margin-bottom: 40px;".to_string()),
                    ].into_iter().collect(),
                    children: vec![VNode::Text("🎨 径向渐变 GPU 渲染演示".to_string())],
                    key: None,
                }),
                // 渐变网格容器
                VNode::Element(VElement {
                    tag: "div".to_string(),
                    attrs: vec![
                        ("style".to_string(), "display: flex; gap: 20px; justify-content: center; flex-wrap: wrap;".to_string()),
                    ].into_iter().collect(),
                    children: vec![
                        // 卡片 1: 白色到紫色
                        VNode::Element(VElement {
                            tag: "div".to_string(),
                            attrs: vec![
                                ("style".to_string(), "width: 200px; height: 200px; border-radius: 16px; background: radial-gradient(circle, white, #6b5b95); box-shadow: 0 4px 12px rgba(0,0,0,0.3);".to_string()),
                            ].into_iter().collect(),
                            children: vec![],
                            key: None,
                        }),
                        // 卡片 2: 青色到品红
                        VNode::Element(VElement {
                            tag: "div".to_string(),
                            attrs: vec![
                                ("style".to_string(), "width: 200px; height: 200px; border-radius: 16px; background: radial-gradient(circle, #00bcd4, #e91e63); box-shadow: 0 4px 12px rgba(0,0,0,0.3);".to_string()),
                            ].into_iter().collect(),
                            children: vec![],
                            key: None,
                        }),
                        // 卡片 3: 金色到红色
                        VNode::Element(VElement {
                            tag: "div".to_string(),
                            attrs: vec![
                                ("style".to_string(), "width: 200px; height: 200px; border-radius: 16px; background: radial-gradient(circle, #ffd700, #d32f2f); box-shadow: 0 4px 12px rgba(0,0,0,0.3);".to_string()),
                            ].into_iter().collect(),
                            children: vec![],
                            key: None,
                        }),
                    ],
                    key: None,
                }),
                // 说明文字
                VNode::Element(VElement {
                    tag: "p".to_string(),
                    attrs: vec![
                        ("style".to_string(), "color: #a0a0a0; margin-top: 40px; font-size: 18px;".to_string()),
                    ].into_iter().collect(),
                    children: vec![VNode::Text("✨ 所有径向渐变都在 GPU 上实时渲染".to_string())],
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

    info!("🌟 Iris Radial Gradient Window Demo");
    info!("====================================\n");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    
    // 构建应用
    let mut app = RadialGradientApp::new();

    // 在创建窗口之前设置窗口属性
    let mut app = {
        // 使用事件循环创建窗口
        let window = event_loop.create_window(
            winit::window::WindowAttributes::default()
                .with_title("Iris - Radial Gradient Demo")
                .with_inner_size(winit::dpi::PhysicalSize::new(1200, 900))
                .with_visible(false) // 先不可见，初始化完成后显示
        ).expect("Failed to create window");
        
        app.window = Some(window);
        app
    };

    // 显示窗口
    if let Some(window) = &app.window {
        window.set_visible(true);
        info!("✅ Window created and shown");
    }

    // 运行事件循环
    event_loop.run_app(&mut app).expect("Event loop failed");
}
