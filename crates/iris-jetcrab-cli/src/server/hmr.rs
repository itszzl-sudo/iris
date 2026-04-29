//! HMR (Hot Module Replacement) 模块
//!
//! 负责文件监听和热更新推送
//!
//! 架构：
//! 1. notify 监听 src 目录文件变化
//! 2. 防抖处理（避免频繁触发）
//! 3. 使编译缓存失效
//! 4. 通过 WebSocket 推送更新到浏览器

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
        path: String,
        timestamp: u64,
    },
    /// 重新编译完成
    #[serde(rename = "rebuild-complete")]
    RebuildComplete {
        modules_count: usize,
        duration_ms: u64,
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

        // 创建防抖处理任务
        let (debounce_tx, mut debounce_rx) = tokio::sync::mpsc::channel::<()>(1);

        let cache_clone = cache.clone();
        let ws_manager = self.ws_manager.clone();

        tokio::spawn(async move {
            loop {
                // 等待信号，使用 debounce 延迟
                tokio::select! {
                    _ = debounce_rx.recv() => {
                        // 等待 300ms 防抖
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        
                        info!("File change detected, triggering rebuild...");
                        
                        // 广播文件变更事件
                        ws_manager.broadcast(HmrEvent::FileChanged {
                            path: "src/".to_string(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        });
                        
                        // 记录开始时间
                        let start = std::time::Instant::now();
                        
                        // 重新编译
                        match cache_clone.lock().await.rebuild().await {
                            Ok(()) => {
                                let duration = start.elapsed().as_millis() as u64;
                                info!("Rebuild completed in {}ms", duration);
                                
                                // 广播重新编译完成事件
                                ws_manager.broadcast(HmrEvent::RebuildComplete {
                                    modules_count: 0, // TODO: 从缓存获取
                                    duration_ms: duration,
                                });
                            }
                            Err(e) => {
                                warn!("Rebuild failed: {}", e);
                                
                                // 广播编译错误
                                ws_manager.broadcast(HmrEvent::CompileError {
                                    message: e.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        });

        // 创建文件监听器
        let debounce_tx_clone = debounce_tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    // 只关心修改和创建事件
                    if matches!(
                        event.kind,
                        notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                    ) {
                        debug!("File event: {:?}", event);
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
    pub fn stop(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
            info!("HMR file watcher stopped");
        }
    }
}
