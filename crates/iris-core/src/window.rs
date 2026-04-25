//! 跨平台窗口管理
//!
//! 桌面端基于 winit 0.30+，Wasm 端基于浏览器 canvas（待实现）。

#![warn(missing_docs)]

/// 窗口创建配置。
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// 窗口标题。
    pub title: String,
    /// 窗口初始宽度（逻辑像素）。
    pub width: u32,
    /// 窗口初始高度（逻辑像素）。
    pub height: u32,
    /// 是否允许调整窗口大小。
    pub resizable: bool,
    /// 是否最大化显示。
    pub maximized: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Iris App".to_string(),
            width: 1280,
            height: 720,
            resizable: true,
            maximized: false,
        }
    }
}

impl WindowConfig {
    /// 快速创建指定标题和尺寸的窗口配置。
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
            ..Default::default()
        }
    }
}

/// 在桌面端创建 winit 窗口。
#[cfg(not(target_arch = "wasm32"))]
pub fn create_window(
    event_loop: &winit::event_loop::ActiveEventLoop,
    config: WindowConfig,
) -> Result<winit::window::Window, Box<dyn std::error::Error>> {
    use winit::dpi::LogicalSize;

    let attributes = winit::window::Window::default_attributes()
        .with_title(config.title)
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .with_maximized(config.maximized);

    let window = event_loop.create_window(attributes)?;
    Ok(window)
}

/// Wasm 窗口创建占位（待实现）。
#[cfg(target_arch = "wasm32")]
pub fn create_window(
    _event_loop: &(),
    _config: WindowConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    unimplemented!("Wasm window creation not yet implemented")
}
