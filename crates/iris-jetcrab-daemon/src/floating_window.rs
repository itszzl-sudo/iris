//! 透明悬浮窗口 - 使用 winit + softbuffer

use crate::particles::ParticleSystem;
use crate::renderer::{self, RainbowIcon, WINDOW_WIDTH, WINDOW_HEIGHT};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{CursorIcon, Window, WindowId};

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;

/// 悬浮窗口位置模式
pub enum PositionMode {
    /// 桌面右下角（默认）
    DesktopCorner,
    /// 浏览器附近（Vue 渲染成功时）
    NearBrowser { x: i32, y: i32 },
    /// 自定义位置
    Custom(i32, i32),
}

/// 悬浮窗口应用
pub struct FloatingApp {
    /// winit 窗口
    window: Option<Rc<Window>>,
    /// softbuffer 缓冲区
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    /// 上下文
    context: Option<softbuffer::Context<Rc<Window>>>,
    /// 彩虹图标数据（静态缓存引用）
    icon: &'static RainbowIcon,
    /// 粒子系统
    particles: ParticleSystem,
    /// 当前位置
    pos_x: i32,
    pos_y: i32,
    /// 拖拽相关
    dragging: bool,
    /// 拖拽偏移（窗口相对坐标，记录按下时的光标位置）
    drag_offset_x: f64,
    drag_offset_y: f64,
    /// 最近光标位置（窗口相对坐标）
    cursor_x: f64,
    cursor_y: f64,
    /// 渲染帧时间
    last_frame: Instant,
    /// 窗口是否可见
    visible: bool,
    /// 守护进程管理端口
    daemon_port: u16,
}

impl FloatingApp {
    pub fn new() -> Self {
        Self {
            window: None,
            surface: None,
            context: None,
            icon: renderer::generate_rainbow_icon(),
            particles: ParticleSystem::new(),
            pos_x: 0,
            pos_y: 0,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            last_frame: Instant::now(),
            visible: true,
            daemon_port: 19999,
        }
    }

    /// 设置守护进程端口（用于打开管理页面）
    pub fn set_daemon_port(&mut self, port: u16) {
        self.daemon_port = port;
    }

    /// 设置位置
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.pos_x = x;
        self.pos_y = y;
        if let Some(window) = &self.window {
            let _ = window.set_outer_position(PhysicalPosition::new(x as f64, y as f64));
        }
    }
}

impl ApplicationHandler for FloatingApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut window_attrs = Window::default_attributes()
            .with_title("Iris JetCrab")
            .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
            .with_cursor(CursorIcon::Default);

        // Windows: 不显示在任务栏
        #[cfg(target_os = "windows")]
        {
            window_attrs = window_attrs.with_skip_taskbar(true);
        }

        let window = event_loop.create_window(window_attrs).unwrap();

        let w = Rc::new(window);

        // softbuffer context + surface
        let context = softbuffer::Context::new(w.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, w.clone()).unwrap();

        // 设置初始位置（右下角）
        let monitors = event_loop.available_monitors();
        let mut screen_w = 1920i32;
        let mut screen_h = 1080i32;
        for monitor in monitors {
            let size = monitor.size();
            screen_w = size.width as i32;
            screen_h = size.height as i32;
        }
        self.pos_x = screen_w - WINDOW_WIDTH as i32 - 20;
        self.pos_y = screen_h - WINDOW_HEIGHT as i32 - 60; // 任务栏上方
        let _ = w.set_outer_position(PhysicalPosition::new(self.pos_x as f64, self.pos_y as f64));

        self.window = Some(w.clone());
        self.context = Some(context);
        self.surface = Some(surface);
        self.last_frame = Instant::now();
        self.cursor_x = (WINDOW_WIDTH / 2) as f64;
        self.cursor_y = (WINDOW_HEIGHT / 2) as f64;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x;
                self.cursor_y = position.y;
                if self.dragging {
                    // position 是窗口相对坐标
                    // delta = 当前光标位置 - 按下时光标位置（同一坐标系：窗口相对）
                    let dx = position.x - self.drag_offset_x;
                    let dy = position.y - self.drag_offset_y;
                    // 新窗口位置 = 旧窗口位置 + delta（消除反馈环路）
                    let new_x = self.pos_x + dx.round() as i32;
                    let new_y = self.pos_y + dy.round() as i32;
                    self.pos_x = new_x;
                    self.pos_y = new_y;
                    // 重置偏移，使下一次移动基于新的窗口位置计算纯增量
                    self.drag_offset_x = position.x;
                    self.drag_offset_y = position.y;
                    if let Some(window) = &self.window {
                        let _ = window.set_outer_position(PhysicalPosition::new(new_x as f64, new_y as f64));
                    }
                    // 记录轨迹点（屏幕绝对坐标 — 窗口中心）
                    self.particles.add_trail_point(
                        new_x + WINDOW_WIDTH as i32 / 2,
                        new_y + WINDOW_HEIGHT as i32 / 2,
                    );
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // 双击左键打开管理页面
                let cursor_pos = (self.cursor_x, self.cursor_y);
                if button == MouseButton::Left && state == ElementState::Pressed {
                    // 双击检测：记录上次点击时间
                    static mut LAST_CLICK: Option<Instant> = None;
                    static mut CLICK_COUNT: u32 = 0;
                    unsafe {
                        let now = Instant::now();
                        if let Some(last) = LAST_CLICK {
                            if now.duration_since(last).as_millis() < 400 {
                                CLICK_COUNT += 1;
                            } else {
                                CLICK_COUNT = 1;
                            }
                        } else {
                            CLICK_COUNT = 1;
                        }
                        LAST_CLICK = Some(now);
                        
                        if CLICK_COUNT >= 2 {
                            CLICK_COUNT = 0;
                            // 在浏览器中打开管理页面
                            let url = format!("http://127.0.0.1:{}", self.daemon_port);
                            tracing::info!("Double-click detected, opening management page: {}", url);
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("cmd")
                                    .args(&["/c", "start", &url])
                                    .spawn();
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                let _ = std::process::Command::new("xdg-open")
                                    .arg(&url)
                                    .spawn();
                            }
                            return;
                        }
                    }
                }

                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            self.dragging = true;
                            self.particles.set_dragging(true);
                            self.drag_offset_x = cursor_pos.0;
                            self.drag_offset_y = cursor_pos.1;
                        }
                        ElementState::Released => {
                            self.dragging = false;
                            self.particles.set_dragging(false);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.05);
                self.last_frame = now;

                // 更新粒子
                self.particles.update(dt);

                // 渲染
                if let Some(surface) = &mut self.surface {
                    // 调整大小
                    surface
                        .resize(
                            NonZeroU32::new(WINDOW_WIDTH).unwrap(),
                            NonZeroU32::new(WINDOW_HEIGHT).unwrap(),
                        )
                        .ok();

                    let mut buffer = surface.buffer_mut().unwrap();

                    // 清空为全透明
                    for pixel in buffer.iter_mut() {
                        *pixel = 0u32;
                    }

                    {
                        // 获取底层字节缓冲区
                        let raw: &mut [u8] = bytemuck::cast_slice_mut(buffer.as_mut());

                        // 1. 呼吸光晕
                        let glow_alpha = self.particles.breathe_glow_alpha();
                        renderer::draw_breathe_glow(raw, glow_alpha);

                        // 2. 拖拽轨迹
                        renderer::draw_trail(raw, &self.particles);

                        // 3. 星光粒子
                        renderer::draw_sparkles(raw, &self.particles);

                        // 4. 彩虹图标（最上层）
                        renderer::draw_icon(raw, self.icon);
                    }

                    buffer.present().ok();
                }

                // 请求下一帧
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
