//! HMR (Hot Module Replacement) 模块
//!
//! 负责文件监听和热更新推送
//!
//! 架构：
//! 1. notify 监听 src 目录文件变化
//! 2. 收集文件路径 + 防抖处理（避免频繁触发）
//! 3. 使编译缓存失效
//! 4. 通过 WebSocket 推送更新到浏览器
//!
//! HMR 策略：
//! - CSS/SCSS 文件变更：直接编译并推送新 CSS 内容到浏览器（无 JS 重执行）
//! - Vue/TS 文件变更：推送模块更新事件，浏览器动态 import 替换
//! - 入口文件变更：全量重载兜底

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tracing::{info, debug, warn};
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// HMR 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HmrEvent {
    /// 连接成功
    #[serde(rename = "connected")]
    Connected {
        message: String,
    },
    /// 文件变更
    #[serde(rename = "file-changed")]
    FileChanged {
        /// 变更的路径（相对于项目根目录）
        path: String,
        /// 变更文件名
        file_name: String,
        timestamp: u64,
    },
    /// 重新编译完成
    #[serde(rename = "rebuild-complete")]
    RebuildComplete {
        /// 已清理的缓存模块数
        cleared_modules: usize,
        /// 耗时（毫秒）
        duration_ms: u64,
    },
    /// 模块级更新（模块级 HMR）
    /// 对 Vue/TS 文件：浏览器动态 import 替换
    #[serde(rename = "module-update")]
    ModuleUpdate {
        /// 模块路径（如 /src/App.vue）
        path: String,
        /// 模块类型: "vue" | "script" | "style"
        module_type: String,
        /// 时间戳
        timestamp: u64,
    },
    /// 样式更新（CSS/SCSS 文件热替换，无 JS 重执行）
    #[serde(rename = "style-update")]
    StyleUpdate {
        /// 样式路径
        path: String,
        /// 新 CSS 内容
        css: String,
        /// 时间戳
        timestamp: u64,
    },
    /// 编译错误
    #[serde(rename = "compile-error")]
    CompileError {
        message: String,
    },
    /// npm 包下载进度
    #[serde(rename = "npm-download")]
    NpmDownload {
        /// 包名
        package: String,
        /// 版本号
        version: String,
        /// 进度百分比 (0-100)
        progress: u8,
        /// 状态: downloading | extracting | installed | error
        status: String,
        /// 错误信息（如果有）
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}

/// WebSocket 客户端管理器
pub struct WebSocketManager {
    /// 广播频道发送器
    tx: broadcast::Sender<HmrEvent>,
}

impl WebSocketManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<HmrEvent>(100);
        Self { tx }
    }

    /// 获取事件订阅器
    pub fn subscribe(&self) -> broadcast::Receiver<HmrEvent> {
        self.tx.subscribe()
    }

    /// 广播事件
    pub fn broadcast(&self, event: HmrEvent) {
        let _ = self.tx.send(event);
    }
}

/// HMR 管理器
pub struct HMRManager {
    /// 项目根目录
    project_root: PathBuf,
    /// 文件监听器
    watcher: Option<RecommendedWatcher>,
    /// WebSocket 管理器
    ws_manager: Arc<WebSocketManager>,
    /// 是否启用
    enabled: bool,
}

impl HMRManager {
    /// 创建新的 HMR 管理器
    pub fn new(project_root: PathBuf, enabled: bool) -> Self {
        Self {
            project_root,
            watcher: None,
            ws_manager: Arc::new(WebSocketManager::new()),
            enabled,
        }
    }

    /// 获取 WebSocket 管理器
    pub fn ws_manager(&self) -> Arc<WebSocketManager> {
        self.ws_manager.clone()
    }

    /// 启动文件监听
    pub async fn start_watching(
        &mut self,
        cache: Arc<Mutex<crate::server::compiler_cache::CompilerCache>>,
    ) -> anyhow::Result<()> {
        if !self.enabled {
            info!("HMR is disabled");
            return Ok(());
        }

        info!("Starting HMR file watcher...");

        let src_dir = self.project_root.join("src");
        if !src_dir.exists() {
            warn!("src directory not found, HMR disabled");
            return Ok(());
        }

        // 共享的变更文件路径收集器
        // watcher 回调线程写入，async 任务读取并处理
        let changed_files: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));

        // 创建防抖处理任务
        let (debounce_tx, mut debounce_rx) = tokio::sync::mpsc::channel::<()>(1);

        let cache_clone = cache.clone();
        let ws_manager = self.ws_manager.clone();
        let changed_files_clone = changed_files.clone();
        let project_root = self.project_root.clone();

        tokio::spawn(async move {
            loop {
                // 等待信号，使用 debounce 延迟
                tokio::select! {
                    _ = debounce_rx.recv() => {
                        // 等待 300ms 防抖
                        tokio::time::sleep(Duration::from_millis(300)).await;

                        // 取出所有已收集的变更文件路径
                        let paths = {
                            let mut files = changed_files_clone.lock().await;
                            let paths: Vec<PathBuf> = files.drain(..).collect();
                            paths
                        };

                        if paths.is_empty() {
                            continue;
                        }

                        info!("File change detected ({} file(s)), triggering HMR...", paths.len());

                        let start = std::time::Instant::now();
                        let cache_lock = cache_clone.lock().await;

                        // 分类处理变更文件：CSS/SCSS → 直接推送样式更新
                        // Vue/TS/JS → 推送模块更新，浏览器动态 import
                        let mut has_style_changes = false;
                        let mut has_module_changes = false;
                        let mut has_entry_changes = false;

                        for file_path in &paths {
                            let relative = file_path
                                .strip_prefix(&project_root)
                                .unwrap_or(file_path)
                                .to_string_lossy()
                                .to_string()
                                .replace('\\', "/");
                            let file_name = file_path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64;

                            // 广播文件变更事件
                            let file_name_clone = file_name.clone();
                            ws_manager.broadcast(HmrEvent::FileChanged {
                                path: relative.clone(),
                                file_name: file_name_clone,
                                timestamp,
                            });

                            // 判断文件类型和 HMR 策略
                            let ext = file_path.extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("");

                            let is_entry = file_name == "main.ts"
                                || file_name == "main.js"
                                || file_name == "App.vue";

                            if is_entry {
                                // 入口文件变更 → 全量重载
                                has_entry_changes = true;
                                debug!("Entry file changed: {}, will trigger full reload", file_name);
                            } else if matches!(ext, "css" | "scss" | "sass" | "less") {
                                // CSS/SCSS 文件变更 → 推送样式更新（无 JS 重执行）
                                has_style_changes = true;

                                // 尝试编译并获取新 CSS 内容
                                if let Ok(_module_path) = cache_lock.get_module_path(&relative) {
                                    // 构建模块编译的 URL 路径
                                    let module_path_key = if relative.starts_with("src/") {
                                        format!("/{}", relative)
                                    } else {
                                        format!("/src/{}", relative.trim_start_matches("src/"))
                                    };

                                // 尝试编译（编译后的 CSS 直接推送）
                                    match cache_lock.get_or_compile(&module_path_key).await {
                                        Ok(compiled) => {
                                            // 提取 CSS 内容
                                            let css_content = compiled.styles.first()
                                                .map(|s| s.code.clone())
                                                .unwrap_or_default();

                                            let mpk = module_path_key.clone();
                                            ws_manager.broadcast(HmrEvent::StyleUpdate {
                                                path: module_path_key,
                                                css: css_content,
                                                timestamp,
                                            });
                                            debug!("Style update pushed for: {}", mpk);
                                        }
                                        Err(_) => {
                                            // 编译失败，使用原始内容
                                            warn!("Failed to compile style: {}, invalidating cache", file_name);
                                            cache_lock.invalidate(&module_path_key).await;
                                            ws_manager.broadcast(HmrEvent::CompileError {
                                                message: format!("Failed to compile: {}", file_name),
                                            });
                                        }
                                    }
                                } else {
                                    // 无法推导模块路径，直接失效缓存
                                    warn!("Cannot derive module path for: {:?}", file_path);
                                }
                            } else {
                                // Vue/TS/JS 文件变更 → 推送模块更新
                                has_module_changes = true;

                                // 推导模块的 URL 路径
                                let module_path_key = if relative.starts_with("src/") {
                                    format!("/{}", relative)
                                } else if relative.starts_with("/") {
                                    relative.clone()
                                } else {
                                    format!("/src/{}", relative)
                                };

                                // 使缓存失效
                                cache_lock.invalidate(&module_path_key).await;

                                let mpk = module_path_key.clone();
                                ws_manager.broadcast(HmrEvent::ModuleUpdate {
                                    path: module_path_key,
                                    module_type: if ext == "vue" {
                                        "vue".to_string()
                                    } else if matches!(ext, "ts" | "tsx") {
                                        "script".to_string()
                                    } else {
                                        "script".to_string()
                                    },
                                    timestamp,
                                });
                                debug!("Module update pushed for: {} ({})", mpk, ext);
                            }
                        }

                        let duration = start.elapsed().as_millis() as u64;

                        // 如果有入口文件变更，发送全量重载
                        if has_entry_changes {
                            info!("Entry file changed, full page reload in {}ms", duration);
                            ws_manager.broadcast(HmrEvent::RebuildComplete {
                                cleared_modules: 0,
                                duration_ms: duration,
                            });
                        } else if has_style_changes || has_module_changes {
                            debug!(
                                "HMR processed: {} style(s), {} module(s) in {}ms",
                                if has_style_changes { 1 } else { 0 },
                                if has_module_changes { 1 } else { 0 },
                                duration
                            );
                        }
                    }
                }
            }
        });

        // 创建文件监听器
        let debounce_tx_clone = debounce_tx.clone();
        let changed_files_inner = changed_files.clone();
        let src_dir_clone = src_dir.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    // 只关心修改和创建事件
                    if matches!(
                        event.kind,
                        notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                    ) {
                        // 收集变更的文件路径
                        for path in &event.paths {
                            // 只处理 src/ 目录下的源码文件
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                let is_source = matches!(ext, "vue" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "css" | "scss" | "sass" | "less");
                                if is_source && path.starts_with(&src_dir_clone) {
                                    debug!("File changed: {:?}", path);
                                    // 同步收集到共享 vec（阻塞式）
                                    let mut files = changed_files_inner.blocking_lock();
                                    if !files.contains(path) {
                                        files.push(path.clone());
                                    }
                                }
                            }
                        }
                        // 发送防抖信号（如果通道满了则忽略）
                        let _ = debounce_tx_clone.try_send(());
                    }
                }
            },
            Config::default(),
        )?;

        // 监听 src 目录（递归）
        watcher.watch(&src_dir, RecursiveMode::Recursive)?;
        info!("Watching: {:?}", src_dir);

        self.watcher = Some(watcher);

        Ok(())
    }

    /// 停止监听
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
            info!("HMR file watcher stopped");
        }
    }
}
