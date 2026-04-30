// 径向渐变渲染示例
// 展示 iris-gpu 的径向渐变功能

use iris_engine::orchestrator::RuntimeOrchestrator;
use iris_gpu::{Renderer, DrawCommand};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct RadialGradientApp {
    orchestrator: RuntimeOrchestrator,
}

impl ApplicationHandler for RadialGradientApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {

        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Iris Radial Gradient Demo")
                    .with_inner_size(winit::dpi::PhysicalSize::new(800, 600)),
            )
            .unwrap();

        println!("✅ Window created");

        // 初始化 GPU 渲染器
        match pollster::block_on(Renderer::new(window)) {
            Ok(renderer) => {
                self.orchestrator.set_gpu_renderer(renderer);
                println!("✅ GPU renderer initialized");
            }
            Err(e) => {
                eprintln!("❌ Failed to initialize GPU renderer: {}", e);
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_size) => {
                // GPU 渲染器会自行处理窗口大小变化
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
            }
            _ => {}
        }
    }
}

impl RadialGradientApp {
    fn render_frame(&mut self) {
        if let Some(renderer) = self.orchestrator.gpu_renderer_mut() {
            // 示例 1: 基础径向渐变（白色到紫色）
            renderer.submit_command(DrawCommand::RadialGradientRect {
                center_x: 150.0,
                center_y: 150.0,
                radius: 100.0,
                start_color: [1.0, 1.0, 1.0, 1.0], // 白色中心
                end_color: [0.4, 0.3, 0.8, 1.0],   // 紫色边缘
            });

            // 示例 2: 青色到品红的径向渐变
            renderer.submit_command(DrawCommand::RadialGradientRect {
                center_x: 400.0,
                center_y: 150.0,
                radius: 120.0,
                start_color: [0.0, 0.8, 0.8, 1.0],   // 青色中心
                end_color: [1.0, 0.0, 1.0, 1.0],     // 品红边缘
            });

            // 示例 3: 金色到深红的径向渐变
            renderer.submit_command(DrawCommand::RadialGradientRect {
                center_x: 650.0,
                center_y: 150.0,
                radius: 110.0,
                start_color: [1.0, 0.84, 0.0, 1.0],  // 金色中心
                end_color: [0.6, 0.0, 0.0, 1.0],     // 深红边缘
            });

            // 示例 4: 大尺寸径向渐变（背景效果）
            renderer.submit_command(DrawCommand::RadialGradientRect {
                center_x: 400.0,
                center_y: 420.0,
                radius: 250.0,
                start_color: [0.2, 0.8, 0.6, 1.0],   // 青绿色中心
                end_color: [0.1, 0.2, 0.5, 1.0],     // 深蓝边缘
            });

            // 示例 5: 小尺寸径向渐变（按钮效果）
            for i in 0..5 {
                renderer.submit_command(DrawCommand::RadialGradientRect {
                    center_x: 100.0 + i as f32 * 150.0,
                    center_y: 500.0,
                    radius: 40.0,
                    start_color: [1.0, 1.0, 1.0, 0.9],
                    end_color: [0.5 + i as f32 * 0.1, 0.3, 0.8, 0.9],
                });
            }

            // 刷新渲染
            println!("🎨 Rendering radial gradients...");
        }
    }
}

fn main() {
    println!("🌟 Iris Radial Gradient Demo");
    println!("============================\n");

    let event_loop = EventLoop::new().unwrap();
    let mut app = RadialGradientApp {
        orchestrator: RuntimeOrchestrator::new(),
    };

    event_loop.run_app(&mut app).unwrap();
}
