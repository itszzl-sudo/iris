//! 开发服务器命令 - 原生窗口渲染
//!
//! 实现真正的多页面渲染：
//! 1. 加载 Vue SFC 文件
//! 2. 创建原生窗口（winit）
//! 3. 使用 WebGPU 渲染（iris-gpu）
//! 4. 支持热重载

use clap::Args;
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use iris_engine::orchestrator::RuntimeOrchestrator;
use iris_gpu::Renderer;
use tracing::{info, warn};
use crate::config::IrisConfig;
use crate::utils::{self, print_success, print_info, print_warning};

/// 开发服务器命令参数
#[derive(Args)]
pub struct DevCommand {
    /// 项目根目录
    #[arg(short, long, default_value = ".")]
    pub root: String,
    
    /// 开发服务器端口（保留用于未来浏览器模式）
    #[arg(short, long)]
    pub port: Option<u16>,
    
    /// 禁用热重载
    #[arg(long)]
    pub no_hot_reload: bool,
    
    /// 自动打开浏览器（保留）
    #[arg(short, long)]
    pub open: bool,
}

impl DevCommand {
    pub fn execute(&self) -> Result<()> {
        println!("{}", "🌈 Iris Runtime - Development Mode".bright_cyan().bold());
        println!("{}", "Native Window Rendering with WebGPU".bright_black());
        println!();
        
        // 找到项目根目录
        let project_root = utils::find_project_root(std::path::Path::new(&self.root))?;
        print_success(&format!("Project root: {}", project_root.display()));
        
        // 加载配置
        let mut config = IrisConfig::load(&project_root)?;
        
        // 覆盖配置
        if let Some(port) = self.port {
            config.dev_server.port = port;
        }
        if self.no_hot_reload {
            config.dev_server.hot_reload = false;
        }
        if self.open {
            config.dev_server.open = true;
        }
        
        // 显示配置
        self.print_config(&config);
        
        // 检测项目类型
        let project_type = IrisConfig::detect_project_type(&project_root);
        match project_type {
            crate::config::ProjectType::Vue3 => {
                print_success("Detected Vue 3 project");
            }
            crate::config::ProjectType::Unknown => {
                utils::print_warning("Unknown project type, using default configuration");
            }
        }
        
        println!();
        
        // 查找所有 Vue SFC 文件
        let vue_files = self.find_vue_files(&project_root, &config.src_dir);
        if vue_files.is_empty() {
            print_warning("No .vue files found in src/ directory");
            println!("Please create at least one Vue SFC file to get started.");
            return Ok(());
        }
        
        print_success(&format!("Found {} Vue SFC file(s)", vue_files.len()));
        for (i, file) in vue_files.iter().enumerate() {
            println!("  {}. {}", i + 1, file.display());
        }
        println!();
        
        // 启动原生窗口渲染
        self.start_native_renderer(&project_root, &vue_files, &config)
    }
    
    fn print_config(&self, config: &IrisConfig) {
        println!("{}", "Configuration:".bright_cyan().bold());
        println!("  Project: {}", config.name);
        println!("  Version: {}", config.version);
        println!("  Source:  {}", config.src_dir.display());
        println!("  Output:  {}", config.out_dir.display());
        println!("  Entry:   {}", config.entry);
        println!("  Hot Reload: {}", if config.dev_server.hot_reload { "Yes" } else { "No" });
        println!();
    }
    
    fn find_vue_files(&self, project_root: &std::path::Path, src_dir: &std::path::Path) -> Vec<PathBuf> {
        let src_path = project_root.join(src_dir);
        let mut vue_files = Vec::new();
        
        if !src_path.exists() {
            return vue_files;
        }
        
        // 递归查找所有 .vue 文件
        for entry in walkdir::WalkDir::new(&src_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Some(ext) = entry.path().extension() {
                if ext == "vue" {
                    vue_files.push(entry.path().to_path_buf());
                }
            }
        }
        
        vue_files
    }
    
    fn start_native_renderer(
        &self,
        project_root: &std::path::Path,
        vue_files: &[PathBuf],
        config: &IrisConfig,
    ) -> Result<()> {
        use winit::application::ApplicationHandler;
        use winit::event::{Event, WindowEvent};
        use winit::event_loop::{ActiveEventLoop, EventLoop};
        use winit::window::{Window, WindowId};
        use iris_engine::orchestrator::RuntimeOrchestrator;
        use tracing::info;
        use std::time::Instant;
        
        print_success("Starting native window renderer...");
        print_info("This will create native windows with WebGPU rendering");
        println!();
        print_info("Press Ctrl+C or close windows to exit");
        println!();
        
        // 注意：不在这里初始化 tracing，因为 main.rs 已经初始化了
        info!("Initializing development server with native rendering");
        
        // 创建事件循环
        let event_loop = EventLoop::new().map_err(|e| anyhow::anyhow!("Failed to create event loop: {}", e))?;
        
        // 创建应用状态
        let mut app = DevApp::new(project_root, vue_files, config)?;
        
        // 运行事件循环
        event_loop.run_app(&mut app)
            .map_err(|e| anyhow::anyhow!("Event loop error: {}", e))?;
        
        Ok(())
    }
}

/// 开发应用状态
struct DevApp {
    /// 窗口列表
    windows: HashMap<WindowId, WindowState>,
    /// Vue 文件列表
    vue_files: Vec<PathBuf>,
    /// 项目根目录
    project_root: PathBuf,
    /// 配置
    config: IrisConfig,
    /// 窗口计数器
    window_counter: u32,
    /// 是否启用热重载
    hot_reload_enabled: bool,
}

/// 单个窗口状态
struct WindowState {
    /// 窗口（初始化 GPU 渲染器后会被取走）
    window: Option<Window>,
    /// 编排器
    orchestrator: RuntimeOrchestrator,
    /// 是否已初始化渲染器
    renderer_initialized: bool,
    /// 是否暂停
    suspended: bool,
    /// 对应的 Vue 文件
    vue_file: PathBuf,
}

impl DevApp {
    fn new(project_root: &std::path::Path, vue_files: &[PathBuf], config: &IrisConfig) -> Result<Self> {
        info!("Creating development application");
        
        Ok(Self {
            windows: HashMap::new(),
            vue_files: vue_files.to_vec(),
            project_root: project_root.to_path_buf(),
            config: config.clone(),
            window_counter: 0,
            hot_reload_enabled: config.dev_server.hot_reload,
        })
    }
    
    /// 创建新窗口
    fn create_window(&mut self, event_loop: &ActiveEventLoop, vue_file: &PathBuf) -> Result<()> {
        let window_id = self.window_counter;
        self.window_counter += 1;
        
        info!("Creating window for: {}", vue_file.display());
        
        // 创建窗口
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title(format!("Iris Dev - {}", vue_file.file_name().unwrap_or_default().to_string_lossy()))
                    .with_inner_size(winit::dpi::PhysicalSize::new(1024, 768))
                    .with_resizable(true),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create window: {}", e))?;
        
        let window_id = window.id();
        
        // 创建编排器
        let mut orchestrator = RuntimeOrchestrator::new();
        orchestrator.initialize()
            .map_err(|e| anyhow::anyhow!("Failed to initialize orchestrator: {}", e))?;
        
        // 加载 Vue SFC
        match orchestrator.load_sfc_with_vtree(vue_file) {
            Ok(()) => {
                info!("✅ Vue SFC loaded: {}", vue_file.display());
            }
            Err(e) => {
                print_warning(&format!("Failed to load Vue SFC: {}", e));
            }
        }
        
        // 计算布局
        if let Err(e) = orchestrator.compute_layout() {
            print_warning(&format!("Failed to compute layout: {}", e));
        }
        
        // 启动文件监听器（如果启用）
        if self.hot_reload_enabled {
            let src_path = self.project_root.join(&self.config.src_dir);
            if src_path.exists() {
                match orchestrator.start_file_watcher(vec![src_path.clone()]) {
                    Ok(()) => {
                        info!("✅ File watcher started for: {}", src_path.display());
                    }
                    Err(e) => {
                        print_warning(&format!("Failed to start file watcher: {}", e));
                    }
                }
            }
        }
        
        // 存储窗口状态
        self.windows.insert(window_id, WindowState {
            window: Some(window),
            orchestrator,
            renderer_initialized: false,
            suspended: false,
            vue_file: vue_file.clone(),
        });
        
        Ok(())
    }
    
    /// 渲染帧
    fn render(&mut self, window_id: &WindowId) {
        if let Some(state) = self.windows.get_mut(window_id) {
            if state.suspended {
                return;
            }
            
            // 检查文件变更并触发热重载
            if self.hot_reload_enabled {
                if let Some(changed_file) = state.orchestrator.check_file_events() {
                    info!("🔥 Hot reload triggered by: {}", changed_file.display());
                    match state.orchestrator.hot_reload(&changed_file, &self.project_root) {
                        Ok(()) => {
                            print_success(&format!("Hot reloaded: {}", changed_file.file_name().unwrap_or_default().to_string_lossy()));
                        }
                        Err(e) => {
                            print_warning(&format!("Hot reload failed: {}", e));
                        }
                    }
                }
            }
            
            // 如果渲染器还未初始化，尝试初始化
            if !state.renderer_initialized {
                // 需要取出 window 来初始化 renderer
                if let Some(window) = state.window.take() {
                    let window_size = window.inner_size();
                    info!("Initializing GPU renderer for window {:?} ({}x{})", 
                          window_id, window_size.width, window_size.height);
                    
                    // 使用 pollster 同步初始化 GPU 渲染器
                    match pollster::block_on(Renderer::new(window)) {
                        Ok(renderer) => {
                            info!("✅ GPU renderer created successfully");
                            
                            // 关键：将渲染器设置到 orchestrator 中
                            state.orchestrator.set_gpu_renderer(renderer);
                            state.orchestrator.mark_dirty();
                            state.renderer_initialized = true;
                            
                            info!("✅ GPU renderer attached to orchestrator");
                        }
                        Err(e) => {
                            print_warning(&format!("Failed to initialize GPU renderer: {}", e));
                            state.renderer_initialized = true; // 标记为已尝试，避免重复尝试
                        }
                    }
                    // 注意：Renderer::new 接收 window 所有权，所以不需要放回
                }
            }
            
            // GPU 渲染
            let rendered = state.orchestrator.render_frame_gpu();
            
            if rendered {
                info!("Frame rendered with GPU for window {:?}", window_id);
            }
        }
    }
}

impl ApplicationHandler for DevApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("Application resumed");
        
        // 为每个 Vue 文件创建窗口
        if self.windows.is_empty() {
            // 克隆文件列表以避免借用冲突
            let vue_files: Vec<_> = self.vue_files.clone();
            for vue_file in &vue_files {
                if let Err(e) = self.create_window(event_loop, vue_file) {
                    print_warning(&format!("Failed to create window: {}", e));
                }
            }
        }
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Window closed: {:?}", window_id);
                
                // 在移除之前先清理 GPU 资源和文件监听器
                if let Some(window_state) = self.windows.get_mut(&window_id) {
                    // 停止文件监听器
                    window_state.orchestrator.stop_file_watcher();
                    info!("File watcher stopped for window {:?}", window_id);
                    
                    // 清除 orchestrator 中的 GPU renderer，避免 drop 时崩溃
                    window_state.orchestrator.clear_gpu_renderer();
                    info!("GPU renderer cleared for window {:?}", window_id);
                }
                
                self.windows.remove(&window_id);
                
                // 如果所有窗口都关闭了，退出
                if self.windows.is_empty() {
                    info!("All windows closed, exiting...");
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    // 更新编排器的视口大小
                    state.orchestrator.set_viewport_size(size.width as f32, size.height as f32);
                    
                    // 如果 GPU 渲染器已初始化，调整其大小
                    if let Some(renderer) = state.orchestrator.gpu_renderer_mut() {
                        renderer.resize(size);
                    }
                    
                    // 重新计算布局
                    let _ = state.orchestrator.compute_layout();
                    // 标记需要重新渲染
                    state.orchestrator.mark_dirty();
                    // 重新渲染
                    self.render(&window_id);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render(&window_id);
            }
            _ => {}
        }
    }
}

impl Drop for DevApp {
    fn drop(&mut self) {
        // 在 panic 或者正常退出时，安全地清理所有 GPU 资源
        info!("DevApp dropping, cleaning up GPU resources...");
        
        // 如果在 panic 路径上，使用 forget 避免双重 panic
        if std::thread::panicking() {
            warn!("Dropping during panic, forgetting GPU renderers to avoid double panic");
            // 将所有 windows 中的 GPU renderer forget 掉
            for (_, state) in self.windows.drain() {
                std::mem::forget(state);
            }
        } else {
            // 正常路径：显式清理所有 GPU renderer
            for (_, state) in self.windows.iter_mut() {
                state.orchestrator.clear_gpu_renderer();
            }
            info!("All GPU resources cleaned up");
        }
    }
}
