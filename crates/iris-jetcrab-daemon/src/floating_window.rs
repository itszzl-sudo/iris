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
    /// 子像素位置（f64 保持精度）
    pos_accum_x: f64,
    pos_accum_y: f64,
    /// 拖拽相关
    dragging: bool,
    /// 按下时光标客户端坐标（拖拽期间恒定不变）
    drag_offset_x: f64,
    drag_offset_y: f64,
    /// 按下时光标屏幕绝对坐标（拖拽期间恒定不变）
    press_screen_x: f64,
    press_screen_y: f64,
    /// 上一帧光标屏幕绝对坐标（用于轨迹位置推算）
    last_screen_x: f64,
    last_screen_y: f64,
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
            pos_accum_x: 0.0,
            pos_accum_y: 0.0,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            press_screen_x: 0.0,
            press_screen_y: 0.0,
            last_screen_x: 0.0,
            last_screen_y: 0.0,
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

        // Windows: 设置鼠标穿透区域（仅图标区域可交互）
        #[cfg(target_os = "windows")]
        {
            use winit::raw_window_handle::HasWindowHandle;
            let handle = w.window_handle().unwrap();
            let raw = handle.as_raw();
            if let winit::raw_window_handle::RawWindowHandle::Win32(win32) = raw {
                let hwnd = win32.hwnd.get() as *mut std::ffi::c_void;
                unsafe {
                    #[link(name = "gdi32")]
                    extern "system" {
                        fn CreateRectRgn(x1: i32, y1: i32, x2: i32, y2: i32) -> *mut std::ffi::c_void;
                    }
                    #[link(name = "user32")]
                    extern "system" {
                        fn SetWindowRgn(hWnd: *mut std::ffi::c_void, hRgn: *mut std::ffi::c_void, bRedraw: i32) -> i32;
                    }
                    let icon_x = (WINDOW_WIDTH as i32 - 64) / 2;
                    let icon_y = (WINDOW_HEIGHT as i32 - 64) / 2 + 2;
                    let rgn = CreateRectRgn(icon_x, icon_y, icon_x + 64, icon_y + 64);
                    if !rgn.is_null() {
                        SetWindowRgn(hwnd, rgn, 1);
                        tracing::info!("Set click-through region to icon area ({},{}) {}x{}",
                            icon_x, icon_y, 64, 64);
                    }
                }
            }
        }

        // softbuffer context + surface
        let context = softbuffer::Context::new(w.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, w.clone()).unwrap();

        // 设置初始位置（右下角，若画布超出屏幕则左上角对齐）
        let monitors = event_loop.available_monitors();
        let mut screen_w = 1920i32;
        let mut screen_h = 1080i32;
        for monitor in monitors {
            let size = monitor.size();
            screen_w = size.width as i32;
            screen_h = size.height as i32;
        }
        self.pos_x = (screen_w - WINDOW_WIDTH as i32 - 20).max(0);
        self.pos_y = (screen_h - WINDOW_HEIGHT as i32 - 60).max(0); // 任务栏上方
        self.pos_accum_x = self.pos_x as f64;
        self.pos_accum_y = self.pos_y as f64;
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
                    // ===== 屏幕绝对坐标推算方案 =====
                    // winit CursorMoved 的 position 是窗口客户端坐标。
                    // set_outer_position 后 Windows 会自动发送一个合成 CursorMoved
                    // （窗口动了，光标相对坐标变了，但光标屏幕坐标不变）。
                    // 如果用增量式（累加 dx），合成事件会产生反方向 delta 导致抖动。
                    //
                    // 方案：推算光标屏幕绝对坐标，再反算窗口位置。
                    //   cursor_screen = 当前窗口位置 + 当前客户端坐标
                    //   window_pos    = cursor_screen - 按下时的客户端坐标
                    //
                    // 合成事件中 cursor_screen 不变 → window_pos 不变 → 零抖动。
                    // ====================================

                    // 当前光标屏幕坐标 = 当前窗口位置 + 客户端坐标
                    let cursor_screen_x = self.pos_accum_x + position.x;
                    let cursor_screen_y = self.pos_accum_y + position.y;

                    // 新窗口位置 = 光标屏幕坐标 - 按下时的客户端坐标
                    let new_x_f64 = cursor_screen_x - self.drag_offset_x;
                    let new_y_f64 = cursor_screen_y - self.drag_offset_y;

                    // 轨迹点应该出现在「鼠标上一帧所在屏幕位置」，相对于新窗口坐标
                    // trail_screen = last_screen (鼠标上一帧屏幕绝对位置)
                    // new_window = cursor_screen - drag_offset (窗口即将移动到的位置)
                    // trail_buffer = trail_screen - new_window
                    //              = last_screen - (cursor_screen - drag_offset)
                    //              = drag_offset + last_screen - cursor_screen
                    let prev_screen_x = self.last_screen_x;
                    let prev_screen_y = self.last_screen_y;
                    // 更新 last_screen 为当前光标屏幕坐标（用于下一次计算）
                    self.last_screen_x = cursor_screen_x;
                    self.last_screen_y = cursor_screen_y;

                    // 判断是否为合成事件（光标屏幕坐标无变化 = set_outer_position 触发）
                    // 同时要求最小移动 3px 才添加轨迹点，避免轨迹过度集中
                    let cursor_moved = (cursor_screen_x - prev_screen_x).abs() > 3.0
                        || (cursor_screen_y - prev_screen_y).abs() > 3.0;

                    // 仅在真实鼠标移动时添加轨迹点
                    if cursor_moved {
                        // 轨迹水平位置跟随光标方向：中心 + (光标总位移 × 0.3)
                        // 鼠标右移 → 心形向右扩散，鼠标左移 → 心形向左扩散
                        let trail_pan = (cursor_screen_x - self.press_screen_x) * 0.3;
                        let tx = ((WINDOW_WIDTH / 2) as f64 + trail_pan)
                            .clamp(4.0, (WINDOW_WIDTH - 5) as f64);
                        // Y 坐标固定在图标下方（不遮挡图标）
                        // 图标 64x64 居中，下边缘 ≈75
                        let ty = (WINDOW_HEIGHT - 5) as f64;
                        self.particles.add_trail_point(
                            tx.round() as i32,
                            ty.round() as i32,
                        );
                    }

                    self.pos_accum_x = new_x_f64;
                    self.pos_accum_y = new_y_f64;
                    let new_x = new_x_f64.round() as i32;
                    let new_y = new_y_f64.round() as i32;
                    self.pos_x = new_x;
                    self.pos_y = new_y;
                    if let Some(window) = &self.window {
                        let _ = window.set_outer_position(PhysicalPosition::new(new_x as f64, new_y as f64));
                    }
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
                            // 记录按下时光标的屏幕绝对坐标 = 窗口位置 + 客户端坐标
                            self.press_screen_x = self.pos_accum_x + cursor_pos.0;
                            self.press_screen_y = self.pos_accum_y + cursor_pos.1;
                            self.last_screen_x = self.press_screen_x;
                            self.last_screen_y = self.press_screen_y;
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

                        // 1. 呼吸光晕（底层）
                        let glow_alpha = self.particles.breathe_glow_alpha();
                        renderer::draw_breathe_glow(raw, glow_alpha);

                        // 2. 彩虹图标（先画，作为背景层）
                        renderer::draw_icon(raw, self.icon);

                        // 3. 拖拽轨迹（在图标上层，不会被遮挡）
                        renderer::draw_trail(raw, &self.particles);

                        // 4. 星光粒子（最上层）
                        renderer::draw_sparkles(raw, &self.particles);

                        // 5. 交换 R/B 通道适配 softbuffer BGRA 格式
                        for chunk in raw.chunks_exact_mut(4) {
                            chunk.swap(0, 2);
                        }
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
