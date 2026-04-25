//! Iris Core —— 底层内核底座
//!
//! 提供跨端窗口管理、异步调度、内存池、文件 IO、原生网络栈、缓存系统等基础能力。
//! 是整个 Iris 引擎的根基，不依赖任何上层 crate。

#![warn(missing_docs)]

pub mod runtime;
pub mod window;

use std::sync::Arc;

/// Iris 核心上下文。
///
/// 聚合异步运行时与平台能力，供上层 crate 使用。
pub struct Context {
    /// Tokio 多线程运行时。
    runtime: tokio::runtime::Runtime,
}

impl Context {
    /// 创建并启动 Iris 核心上下文。
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("iris-worker")
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        Self { runtime }
    }

    /// 获取 Tokio 运行时句柄，用于 spawn 异步任务。
    pub fn handle(&self) -> &tokio::runtime::Handle {
        self.runtime.handle()
    }

    /// 在运行时上 spawn 一个异步任务。
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    /// 阻塞当前线程执行异步任务，直到完成。
    ///
    /// 用于在主线程（winit 事件循环线程）中同步初始化异步资源（如 wgpu Device/Queue）。
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// 应用程序生命周期 trait（桌面端）。
///
/// 上层 crate（如 iris-app）实现此 trait 来定义应用生命周期。
#[cfg(not(target_arch = "wasm32"))]
pub trait Application: Send + 'static {
    /// 当事件循环启动、窗口即将创建时调用。
    fn initialize(&mut self, ctx: &Context, event_loop: &winit::event_loop::ActiveEventLoop);

    /// 窗口事件回调。
    fn window_event(
        &mut self,
        ctx: &Context,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    );

    /// 设备/系统事件回调（如屏幕 DPI 变化、时间戳等）。
    #[allow(unused_variables)]
    fn device_event(
        &mut self,
        ctx: &Context,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
    }

    /// 每帧更新回调（AboutToWait 事件）。
    #[allow(unused_variables)]
    fn update(&mut self, ctx: &Context, event_loop: &winit::event_loop::ActiveEventLoop) {}

    /// 当应用即将退出时调用。
    #[allow(unused_variables)]
    fn exiting(&mut self, ctx: &Context) {}
}

/// 启动 Iris 桌面应用程序。
///
/// 阻塞当前线程，启动 winit 事件循环与 Tokio 运行时。
#[cfg(not(target_arch = "wasm32"))]
pub fn run_app<A: Application>(app: A) -> Result<(), Box<dyn std::error::Error>> {
    use winit::application::ApplicationHandler;
    use winit::event::{DeviceEvent, StartCause, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

    struct WinitApp<A> {
        ctx: Arc<Context>,
        app: A,
    }

    impl<A: Application> ApplicationHandler for WinitApp<A> {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            self.app.initialize(&self.ctx, event_loop);
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            window_id: winit::window::WindowId,
            event: WindowEvent,
        ) {
            self.app.window_event(&self.ctx, event_loop, window_id, event);
        }

        fn device_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            device_id: winit::event::DeviceId,
            event: DeviceEvent,
        ) {
            self.app.device_event(&self.ctx, event_loop, device_id, event);
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            self.app.update(&self.ctx, event_loop);
        }

        fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {}

        fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}

        fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
            self.app.exiting(&self.ctx);
        }
    }

    let ctx = Arc::new(Context::new());
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut winit_app = WinitApp { ctx, app };
    event_loop.run_app(&mut winit_app)?;

    Ok(())
}

/// 初始化 Iris 核心（兼容旧 API）。
pub fn init() {
    println!("iris-core initialized");
}
