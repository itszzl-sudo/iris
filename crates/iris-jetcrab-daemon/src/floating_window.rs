//! 透明悬浮窗口 - 使用 winit + softbuffer

use crate::particles::ParticleSystem;
use crate::renderer::{self, RainbowIcon};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
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
    /// 画布尺寸（根据屏幕分辨率动态计算）
    window_width: u32,
    window_height: u32,
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
    /// 守护进程状态引用（用于访问下载进度等）
    daemon_state: Option<Arc<crate::DaemonState>>,
    /// 鼠标是否悬停在图标上
    hovering: bool,
    /// 是否显示下载进度条
    show_download: bool,
    /// 模型下载进度百分比缓存
    model_dl_pct: f64,
    /// NPM 下载进度百分比缓存
    npm_dl_pct: f64,
}

impl FloatingApp {
    pub fn new() -> Self {
        Self {
            window: None,
            surface: None,
            context: None,
            window_width: 300,
            window_height: 400,
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
            daemon_state: None,
            hovering: false,
            show_download: false,
            model_dl_pct: 0.0,
            npm_dl_pct: 0.0,
        }
    }

    /// 设置守护进程端口（用于打开管理页面）
    pub fn set_daemon_port(&mut self, port: u16) {
        self.daemon_port = port;
    }

    /// 设置守护进程状态引用
    pub fn set_daemon_state(&mut self, state: Arc<crate::DaemonState>) {
        self.daemon_state = Some(state);
    }

    /// 设置位置
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.pos_x = x;
        self.pos_y = y;
        if let Some(window) = &self.window {
            let _ = window.set_outer_position(PhysicalPosition::new(x as f64, y as f64));
        }
    }

    /// 显示右键上下文菜单
    fn show_context_menu(&self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "windows")]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            fn to_wide(s: &str) -> Vec<u16> {
                OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
            }

            if let Some(window) = &self.window {
                use winit::raw_window_handle::HasWindowHandle;
                if let Ok(handle) = window.window_handle() {
                    let raw = handle.as_raw();
                    if let winit::raw_window_handle::RawWindowHandle::Win32(win32) = raw {
                        let hwnd = win32.hwnd.get() as *mut std::ffi::c_void;
                        unsafe {
                            #[link(name = "user32")]
                            extern "system" {
                                fn CreatePopupMenu() -> *mut std::ffi::c_void;
                                fn AppendMenuW(hMenu: *mut std::ffi::c_void, uFlags: u32, uIDNewItem: usize, lpNewItem: *const u16) -> i32;
                                fn TrackPopupMenu(hMenu: *mut std::ffi::c_void, uFlags: u32, x: i32, y: i32, nReserved: i32, hWnd: *mut std::ffi::c_void, prcRect: *const std::ffi::c_void) -> i32;
                                fn DestroyMenu(hMenu: *mut std::ffi::c_void) -> i32;
                            }

                            const MF_STRING: u32 = 0;
                            const MF_SEPARATOR: u32 = 0x800;
                            const TPM_RETURNCMD: u32 = 0x0100;

                            let menu = CreatePopupMenu();
                            if menu.is_null() { return; }

                            let s_settings = to_wide("打开管理面板");
                            let s_exit = to_wide("退出");

                            AppendMenuW(menu, MF_STRING, 1, s_settings.as_ptr());

                            // 按需显示下载进度菜单
                            let mut dl_cmd_id = 0u32;
                            if self.show_download {
                                let s_dl = to_wide("下载进度");
                                AppendMenuW(menu, MF_STRING, 2, s_dl.as_ptr());
                                dl_cmd_id = 2;
                            }

                            AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                            AppendMenuW(menu, MF_STRING, 3, s_exit.as_ptr());

                            let screen_x = self.pos_x + self.cursor_x as i32;
                            let screen_y = self.pos_y + self.cursor_y as i32;
                            let cmd = TrackPopupMenu(menu, TPM_RETURNCMD, screen_x, screen_y, 0, hwnd, std::ptr::null());
                            DestroyMenu(menu);

                            match cmd {
                                1 => {
                                    // 打开管理面板
                                    let url = format!("http://127.0.0.1:{}", self.daemon_port);
                                    let _ = std::process::Command::new("cmd")
                                        .args(["/c", "start", &url])
                                        .spawn();
                                }
                                2 if dl_cmd_id == 2 => {
                                    // 打开下载进度URL
                                    let url = format!("http://127.0.0.1:{}", self.daemon_port);
                                    let _ = std::process::Command::new("cmd")
                                        .args(["/c", "start", &url])
                                        .spawn();
                                }
                                3 => {
                                    event_loop.exit();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = event_loop;
        }
    }
}

impl ApplicationHandler for FloatingApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 获取屏幕分辨率
        let monitors = event_loop.available_monitors();
        let mut screen_w = 1920u32;
        let mut screen_h = 1080u32;
        for monitor in monitors {
            let size = monitor.size();
            screen_w = size.width;
            screen_h = size.height;
        }

        // 根据屏幕分辨率动态计算窗口大小
        // 高度约为屏幕的 30%，宽度按比例 (2:3)，均设上下限
        let h = (screen_h as f32 * 0.30)
            .max(200.0)
            .min(600.0) as u32;
        let w = (h * 2 / 3).max(120);
        self.window_width = w;
        self.window_height = h;

        tracing::info!(
            "Detected screen {}x{}, setting canvas to {}x{}",
            screen_w, screen_h, w, h
        );

        let mut window_attrs = Window::default_attributes()
            .with_title("Iris JetCrab")
            .with_inner_size(PhysicalSize::new(w, h))
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

        let w_rc = Rc::new(window);

        // Windows: 设置鼠标穿透区域（仅图标区域可交互）
        #[cfg(target_os = "windows")]
        {
            use winit::raw_window_handle::HasWindowHandle;
            let handle = w_rc.window_handle().unwrap();
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
                    let icon_x = (self.window_width as i32 - 64) / 2;
                    let icon_y = (self.window_height as i32 - 64) / 2 + 2;
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
        let context = softbuffer::Context::new(w_rc.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, w_rc.clone()).unwrap();

        // 设置初始位置（右下角，若画布超出屏幕则左上角对齐）
        let screen_w = screen_w as i32;
        let screen_h = screen_h as i32;
        self.pos_x = (screen_w - self.window_width as i32 - 20).max(0);
        self.pos_y = (screen_h - self.window_height as i32 - 60).max(0); // 任务栏上方
        self.pos_accum_x = self.pos_x as f64;
        self.pos_accum_y = self.pos_y as f64;
        let _ = w_rc.set_outer_position(PhysicalPosition::new(self.pos_x as f64, self.pos_y as f64));

        self.window = Some(w_rc.clone());
        self.context = Some(context);
        self.surface = Some(surface);
        self.last_frame = Instant::now();
        self.cursor_x = (self.window_width / 2) as f64;
        self.cursor_y = (self.window_height / 2) as f64;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x;
                self.cursor_y = position.y;

                // 检测是否悬停在图标区域（居中 64x64）
                let icon_left = (self.window_width as f64 - 64.0) / 2.0;
                let icon_top = (self.window_height as f64 - 64.0) / 2.0 + 2.0;
                self.hovering = position.x >= icon_left && position.x <= icon_left + 64.0
                    && position.y >= icon_top && position.y <= icon_top + 64.0;

                // 更新下载进度缓存
                if let Some(ref st) = self.daemon_state {
                    {
                        let p = st.model_download_progress.lock().unwrap();
                        if let Some(ref prog) = *p {
                            self.model_dl_pct = prog.percentage;
                            self.show_download = self.show_download || (prog.percentage > 0.0 && prog.percentage < 100.0);
                        }
                    }
                    {
                        let p = st.npm_download_progress.lock().unwrap();
                        if let Some(ref prog) = *p {
                            self.npm_dl_pct = prog.percentage;
                            self.show_download = self.show_download || (prog.percentage > 0.0 && prog.percentage < 100.0);
                        }
                    }
                }
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
                        let center_x = (self.window_width / 2) as f64;
                        let clamp_max = (self.window_width - 5) as f64;
                        let tx = (center_x + trail_pan)
                            .clamp(4.0, clamp_max);
                        // Y 坐标固定在图标下方（不遮挡图标）
                        // 图标 64x64 居中，下边缘 = H/2+34
                        // 轨迹放在图标下方约 20+px 处
                        let icon_bottom = (self.window_height as f64 / 2.0) + 34.0;
                        let clamp_y = (self.window_height - 10) as f64;
                        let ty = (icon_bottom + 20.0).min(clamp_y);
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

                // 右键点击：显示上下文菜单
                if button == MouseButton::Right && state == ElementState::Pressed {
                    self.show_context_menu(event_loop);
                    return;
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
                            NonZeroU32::new(self.window_width).unwrap(),
                            NonZeroU32::new(self.window_height).unwrap(),
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
                        let w = self.window_width;
                        let h = self.window_height;

                        // 1. 呼吸光晕（底层）
                        let glow_alpha = self.particles.breathe_glow_alpha();
                        renderer::draw_breathe_glow(raw, w, h, glow_alpha);

                        // 2. 彩虹图标（先画，作为背景层）
                        renderer::draw_icon(raw, w, h, self.icon);

                        // 3. 拖拽轨迹（在图标上层，不会被遮挡）
                        renderer::draw_trail(raw, w, h, &self.particles);

                        // 4. 星光粒子（最上层）
                        renderer::draw_sparkles(raw, w, h, &self.particles);

                        // 5. 交换 R/B 通道适配 softbuffer BGRA 格式
                        for chunk in raw.chunks_exact_mut(4) {
                            chunk.swap(0, 2);
                        }

                        // 6. 悬停提示文字
                        if self.hovering {
                            let tooltip_y = (h / 2 + 34 + 10) as i32; // 图标下方
                            renderer::draw_tooltip(raw, w, h, "I \u{1f49e} iris", (w / 2) as i32, tooltip_y);
                        }

                        // 7. 下载进度条
                        if self.show_download {
                            let max_pct = self.model_dl_pct.max(self.npm_dl_pct);
                            if max_pct > 0.0 && max_pct < 100.0 {
                                let bar_w = 120u32;
                                let bar_h = 8u32;
                                let bar_x = ((w as i32 - bar_w as i32) / 2) as i32;
                                let bar_y = (h as i32 / 2 + 34 + 26) as i32;
                                renderer::draw_progress_bar(raw, w, h, bar_x, bar_y, bar_w, bar_h, max_pct, 102, 126, 234);
                            }
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
