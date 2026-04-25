//! Iris Application Entry Point
//!
//! 面向开发者的最终入口：
//! - 桌面原生模式：编译为独立 EXE / App / 二进制
//! - 浏览器 Wasm 模式：嵌入任意现代浏览器，基于 WebGPU 运行
//!
//! 直接运行 .vue / .ts / .tsx 原始源码，毫秒级热更新，零配置。

use iris_core::{self, window::WindowConfig, Application, Context};
use iris_gpu::{FileChange, WatcherConfig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

/// 最大缓存条目数（防止内存泄漏）。
const MAX_CACHE_SIZE: usize = 100;

/// 最小修改时间阈值（避免快速修改检测不到）。
const MIN_MODIFY_THRESHOLD: Duration = Duration::from_millis(10);

/// 文件轮询间隔（降低每帧开销）。
const FILE_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// SFC 模块编译状态。
#[derive(Debug, Clone)]
enum SfcModuleState {
    /// 编译成功。
    Compiled,
    /// 编译失败（保留错误信息）。
    CompileError {
        /// 错误消息。
        #[allow(dead_code)]
        error: String,
        /// 失败时间。
        #[allow(dead_code)]
        timestamp: SystemTime,
    },
}

/// SFC 模块缓存（路径 → 编译结果）。
#[derive(Debug, Clone)]
struct SfcModuleCache {
    /// 文件路径。
    path: PathBuf,
    /// 最后修改时间（用于检测变更）。
    last_modified: SystemTime,
    /// 文件大小（辅助检测）。
    cached_size: u64,
    /// 编译状态。
    state: SfcModuleState,
}

impl SfcModuleCache {
    fn new(path: PathBuf) -> Self {
        let (last_modified, cached_size) = Self::get_file_info(&path);

        Self {
            path,
            last_modified,
            cached_size,
            state: SfcModuleState::Compiled,
        }
    }

    /// 获取文件信息（修改时间 + 大小）。
    fn get_file_info(path: &Path) -> (SystemTime, u64) {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let modified = metadata
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let size = metadata.len();
                (modified, size)
            }
            Err(_) => (SystemTime::UNIX_EPOCH, 0),
        }
    }

    /// 检查文件是否已修改。
    #[allow(dead_code)]
    fn is_modified(&self) -> bool {
        match std::fs::metadata(&self.path) {
            Ok(metadata) => {
                // 优先使用修改时间
                if let Ok(modified) = metadata.modified() {
                    let time_diff = modified
                        .duration_since(self.last_modified)
                        .unwrap_or_default();
                    return time_diff > MIN_MODIFY_THRESHOLD;
                }

                // 降级：使用文件大小检测
                metadata.len() != self.cached_size
            }
            Err(_) => false,
        }
    }

    /// 更新缓存信息。
    #[allow(dead_code)]
    fn update(&mut self) {
        let (modified, size) = Self::get_file_info(&self.path);
        self.last_modified = modified;
        self.cached_size = size;
    }
}

/// 规范化路径（处理大小写问题）。
fn normalize_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        // Windows 文件系统不区分大小写，统一转小写
        PathBuf::from(path.to_string_lossy().to_lowercase())
    }
    #[cfg(not(windows))]
    {
        // Unix 文件系统区分大小写，保持原样
        path.to_path_buf()
    }
}

/// Iris 桌面应用实例。
struct IrisApp {
    /// GPU 渲染器（同时持有窗口所有权）。
    renderer: Option<iris_gpu::Renderer>,
    /// SFC 模块缓存（路径 → 编译结果）。
    sfc_cache: HashMap<PathBuf, SfcModuleCache>,
    /// 上次文件轮询时间。
    last_poll_time: Instant,
}

#[cfg(not(target_arch = "wasm32"))]
impl Application for IrisApp {
    fn initialize(&mut self, ctx: &Context, event_loop: &winit::event_loop::ActiveEventLoop) {
        info!("Initializing Iris application");

        let config = WindowConfig::new("Iris Engine 光之瞳", 1280, 720);
        match iris_core::window::create_window(event_loop, config) {
            Ok(window) => {
                let window_size = window.inner_size();
                info!(size = ?window_size, "Window created");

                // 同步初始化 wgpu 渲染器（block_on 将异步 Device/Queue 创建阻塞到完成）
                match ctx.block_on(iris_gpu::Renderer::new(window)) {
                    Ok(mut renderer) => {
                        info!(size = ?renderer.size(), "GPU renderer initialized");

                        // 启动文件热更新监听器（监听当前目录的 .vue/.js/.css 文件）
                        let watch_path = std::env::current_dir().unwrap_or_default();
                        renderer.start_file_watcher(
                            WatcherConfig::new(&watch_path)
                                .recursive(true)
                                .extensions(vec![
                                    "vue".to_string(),
                                    "js".to_string(),
                                    "ts".to_string(),
                                    "css".to_string(),
                                ]),
                        );

                        self.renderer = Some(renderer);
                        self.last_poll_time = Instant::now();
                        info!("File watcher started");
                    }
                    Err(e) => error!(
                        error = %e,
                        window_size = ?window_size,
                        "Failed to initialize GPU renderer"
                    ),
                }
            }
            Err(e) => error!(error = %e, "Failed to create window"),
        }
    }

    fn window_event(
        &mut self,
        _ctx: &Context,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                info!("Close requested, exiting...");
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                    debug!(size = ?size, "Window resized");
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, _ctx: &Context, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // 降低轮询频率（避免每帧调用）
        let now = Instant::now();
        if now.duration_since(self.last_poll_time) < FILE_POLL_INTERVAL {
            // 未到轮询时间，直接渲染
            if let Some(renderer) = self.renderer.as_mut() {
                if let Err(e) = renderer.render() {
                    error!(error = ?e, "Render error");
                }
            }
            return;
        }

        self.last_poll_time = now;

        // 检查文件变更
        let changes = if let Some(renderer) = self.renderer.as_mut() {
            renderer.poll_file_changes()
        } else {
            Vec::new()
        };

        if !changes.is_empty() {
            debug!(count = changes.len(), "File changes detected");
            // 处理 SFC 热重载
            self.handle_sfc_hot_reload(changes);
        }

        // 渲染
        if let Some(renderer) = self.renderer.as_mut() {
            if let Err(e) = renderer.render() {
                error!(error = ?e, "Render error");
            }
        }
    }
}

impl IrisApp {
    /// 处理 SFC 热重载逻辑。
    ///
    /// # 参数
    ///
    /// * `changes` - 文件变更事件列表
    fn handle_sfc_hot_reload(&mut self, changes: Vec<FileChange>) {
        for change in changes {
            match change {
                FileChange::Created { path } => {
                    if is_vue_file(&path) {
                        let normalized = normalize_path(&path);
                        info!(path = ?path, "New Vue file created, compiling...");
                        self.compile_and_cache_sfc(&normalized);
                    }
                }
                FileChange::Modified { path } => {
                    if is_vue_file(&path) {
                        let normalized = normalize_path(&path);
                        if self.sfc_cache.contains_key(&normalized) {
                            info!(path = ?path, "Vue file modified, hot reloading...");
                            self.hot_reload_sfc(&normalized);
                        } else {
                            debug!(path = ?path, "Vue file modified but not cached, skipping");
                        }
                    }
                }
                FileChange::Removed { path } => {
                    let normalized = normalize_path(&path);
                    if self.sfc_cache.remove(&normalized).is_some() {
                        info!(path = ?path, "Vue file removed, clearing cache");
                    }
                }
                FileChange::Renamed { from, to } => {
                    let from_normalized = normalize_path(&from);
                    let to_normalized = normalize_path(&to);
                    
                    if self.sfc_cache.contains_key(&from_normalized) {
                        info!(from = ?from, to = ?to, "Vue file renamed");
                        if let Some(mut module) = self.sfc_cache.remove(&from_normalized) {
                            // 更新缓存键和路径
                            module.path = to_normalized.clone();
                            self.sfc_cache.insert(to_normalized, module);
                        }
                    }
                }
            }
        }
    }

    /// 编译并缓存 SFC 模块。
    ///
    /// # 参数
    ///
    /// * `path` - .vue 文件路径（已规范化）
    fn compile_and_cache_sfc(&mut self, path: &PathBuf) {
        // 限制缓存大小
        if self.sfc_cache.len() >= MAX_CACHE_SIZE {
            if let Some(oldest) = self
                .sfc_cache
                .iter()
                .min_by_key(|(_, cache)| cache.last_modified)
                .map(|(k, _)| k.clone())
            {
                debug!(path = ?oldest, "Cache full, removing oldest entry");
                self.sfc_cache.remove(&oldest);
            }
        }

        // 实际调用 iris-sfc 编译器
        match iris_sfc::compile(path) {
            Ok(module) => {
                info!(
                    path = ?path,
                    name = %module.name,
                    style_count = module.styles.len(),
                    "SFC compiled successfully"
                );
                
                // 创建缓存
                let cache = SfcModuleCache::new(path.clone());
                self.sfc_cache.insert(path.clone(), cache);
                
                // TODO: 更新 GPU 资源
                // self.update_gpu_resources(&module);
            }
            Err(e) => {
                error!(path = ?path, error = %e, "Failed to compile SFC");
                // 仍然缓存（标记为失败状态）
                let mut cache = SfcModuleCache::new(path.clone());
                cache.state = SfcModuleState::CompileError {
                    error: e.to_string(),
                    timestamp: SystemTime::now(),
                };
                self.sfc_cache.insert(path.clone(), cache);
            }
        }
    }

    /// 热重载 SFC 模块。
    ///
    /// # 参数
    ///
    /// * `path` - .vue 文件路径（已规范化）
    ///
    /// # 注意
    ///
    /// 如果编译失败，将恢复之前的缓存状态（回滚机制）。
    fn hot_reload_sfc(&mut self, path: &PathBuf) {
        if let Some(cache) = self.sfc_cache.get(path) {
            // 一次性获取文件信息
            let (modified, size) = SfcModuleCache::get_file_info(path);

            // 检查是否真的修改了
            let time_diff = modified
                .duration_since(cache.last_modified)
                .unwrap_or_default();
            
            if time_diff <= MIN_MODIFY_THRESHOLD && size == cache.cached_size {
                debug!(path = ?path, "File not actually modified, skipping");
                return;
            }
        }

        // 保存旧状态（用于回滚）
        let old_cache = self.sfc_cache.get(path).cloned();

        // 实际重新编译
        match iris_sfc::compile(path) {
            Ok(module) => {
                info!(
                    path = ?path,
                    name = %module.name,
                    "SFC hot reloaded successfully"
                );
                
                // 更新缓存信息
                if let Some(cache) = self.sfc_cache.get_mut(path) {
                    let (modified, size) = SfcModuleCache::get_file_info(path);
                    cache.last_modified = modified;
                    cache.cached_size = size;
                    cache.state = SfcModuleState::Compiled;
                }
                
                // TODO: 增量更新 GPU 资源
                // self.patch_gpu_resources(&module);
            }
            Err(e) => {
                error!(path = ?path, error = %e, "Failed to hot reload SFC");
                
                // 回滚到旧状态（如果有）
                if let Some(old) = old_cache {
                    warn!(path = ?path, "Restoring previous version after failed hot reload");
                    self.sfc_cache.insert(path.clone(), old);
                } else {
                    // 没有旧状态，标记为失败
                    if let Some(cache) = self.sfc_cache.get_mut(path) {
                        cache.state = SfcModuleState::CompileError {
                            error: e.to_string(),
                            timestamp: SystemTime::now(),
                        };
                    }
                }
            }
        }
    }
}

/// 检查文件是否为 .vue 文件。
fn is_vue_file(path: &Path) -> bool {
    path.extension().map_or(false, |ext| ext == "vue")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 tracing 订阅者
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // 默认只显示 info 及以上级别
        EnvFilter::new("info,iris_gpu::file_watcher=debug")
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)  // 显示模块路径
        .with_thread_ids(false)  // 隐藏线程 ID（更简洁）
        .with_file(false)  // 隐藏文件名（生产环境）
        .with_line_number(false)  // 隐藏行号
        .init();

    info!("╔══════════════════════════════════════════╗");
    info!("║  Iris Engine 光之瞳                      ║");
    info!("║  Rust+WebGPU 无构建前端运行时            ║");
    info!("╚══════════════════════════════════════════╝");

    let app = IrisApp {
        renderer: None,
        sfc_cache: HashMap::new(),
        last_poll_time: Instant::now(),
    };
    iris_core::run_app(app)?;

    info!("Iris engine exited gracefully.");
    Ok(())
}
