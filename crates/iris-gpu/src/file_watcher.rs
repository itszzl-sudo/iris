//! Iris 文件热更新监听器
//!
//! 使用 `notify` crate 监听文件系统变化，通过 Tokio 异步通道传递事件。
//! 为后续 SFC（Single File Component）热重载提供基础设施。
//!
//! # 特性
//!
//! - **防抖（Debouncing）**：避免编辑器保存时重复触发（默认 500ms）
//! - **事件去重**：同一文件的多次变更只保留最后一次
//! - **跨平台弹窗警告**：通道满时提示用户（仅首次）
//! - **可配置**：通道容量、防抖延迟、扩展名过滤器

use notify::{
    event::{ModifyKind, RenameMode},
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// 弹窗警告标题。
const WARNING_DIALOG_TITLE: &str = "Iris - File Watcher Warning";

/// 弹窗警告消息。
const WARNING_DIALOG_MESSAGE: &str = concat!(
    "File watcher event queue is full!\n\n",
    "Some file changes may have been lost.\n",
    "Please process file events more frequently\n",
    "or restart the application."
);

/// 默认通道容量（适配大型项目）。
const DEFAULT_CHANNEL_CAPACITY: usize = 2000;

/// 默认防抖延迟（毫秒）。
const DEFAULT_DEBOUNCE_DELAY_MS: u64 = 500;

/// 文件变更事件类型。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileChange {
    /// 文件被创建。
    Created {
        /// 文件路径。
        path: PathBuf,
    },
    /// 文件被修改。
    Modified {
        /// 文件路径。
        path: PathBuf,
    },
    /// 文件被删除。
    Removed {
        /// 文件路径。
        path: PathBuf,
    },
    /// 文件被重命名。
    Renamed {
        /// 原路径。
        from: PathBuf,
        /// 新路径。
        to: PathBuf,
    },
}

impl FileChange {
    /// 获取变更涉及的主要路径。
    pub fn path(&self) -> &PathBuf {
        match self {
            FileChange::Created { path } => path,
            FileChange::Modified { path } => path,
            FileChange::Removed { path } => path,
            FileChange::Renamed { from, .. } => from,
        }
    }

    /// 获取文件扩展名（如果有）。
    pub fn extension(&self) -> Option<&str> {
        self.path().extension().and_then(|ext| ext.to_str())
    }
}

/// 文件监听器配置。
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// 要监听的目录路径。
    pub watch_path: PathBuf,
    /// 是否递归监听子目录。
    pub recursive: bool,
    /// 文件过滤器（只监听指定扩展名的文件）。
    /// 如果为 None，则监听所有文件。
    pub extensions: Option<HashSet<String>>,
    /// 通道容量（默认 2000）。
    pub channel_capacity: usize,
    /// 防抖延迟（默认 500ms）。
    pub debounce_delay: Duration,
}

impl WatcherConfig {
    /// 创建新的监听器配置。
    pub fn new<P: Into<PathBuf>>(watch_path: P) -> Self {
        Self {
            watch_path: watch_path.into(),
            recursive: true,
            extensions: None,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            debounce_delay: Duration::from_millis(DEFAULT_DEBOUNCE_DELAY_MS),
        }
    }

    /// 设置是否递归监听。
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// 设置文件扩展名过滤器。
    pub fn extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = Some(extensions.into_iter().collect());
        self
    }

    /// 设置通道容量。
    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = capacity;
        self
    }

    /// 设置防抖延迟。
    pub fn debounce_delay(mut self, delay: Duration) -> Self {
        self.debounce_delay = delay;
        self
    }
}

/// 防抖状态。
struct DebounceState {
    /// 最后一次事件的时间。
    last_event_time: Instant,
    /// 防抖延迟。
    delay: Duration,
}

impl DebounceState {
    fn new(delay: Duration) -> Self {
        Self {
            last_event_time: Instant::now(),
            delay,
        }
    }

    /// 检查是否应该触发事件（距离上次事件超过延迟时间）。
    fn should_fire(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_event_time) >= self.delay {
            self.last_event_time = now;
            true
        } else {
            false
        }
    }

    /// 更新最后事件时间（收到新事件时调用）。
    fn update(&mut self) {
        self.last_event_time = Instant::now();
    }
}

/// 文件热更新监听器。
///
/// 监听指定目录的文件变化，通过异步通道发送变更事件。
/// 内置防抖机制，避免编辑器保存时重复触发。
pub struct FileWatcher {
    /// notify 监听器（在后台线程运行）。
    _watcher: RecommendedWatcher,
    /// 事件接收通道。
    receiver: mpsc::Receiver<FileChange>,
    /// 监听器配置。
    config: WatcherConfig,
    /// 通道满警告标志（确保只弹窗一次）。
    channel_full_warned: Arc<AtomicBool>,
    /// 防抖状态。
    debounce: DebounceState,
}

/// 检查文件是否匹配扩展名过滤器。
///
/// # 参数
///
/// * `path` - 文件路径
/// * `extensions` - 允许的扩展名列表（不含点，不区分大小写）
///
/// # 返回
///
/// 如果文件扩展名在允许列表中，返回 `true`。
fn matches_extension(path: &Path, extensions: &HashSet<String>) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext_lower = ext.to_lowercase();
            extensions
                .iter()
                .any(|allowed| allowed.to_lowercase() == ext_lower)
        })
        .unwrap_or(false)
}

/// 检测是否有图形界面（用于判断是否可以显示弹窗）。
fn has_display_server() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
    }
    #[cfg(target_os = "macos")]
    {
        true // macOS 总是有图形界面
    }
    #[cfg(target_os = "windows")]
    {
        true // Windows 通常是图形界面
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false // 未知平台，保守处理
    }
}

/// 显示警告对话框（安全版本，捕获异常）。
fn show_warning_dialog() {
    let result = std::panic::catch_unwind(|| {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title(WARNING_DIALOG_TITLE)
            .set_description(WARNING_DIALOG_MESSAGE)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    });

    if result.is_err() {
        warn!("Failed to show warning dialog (display server issue?)");
    }
}

impl FileWatcher {
    /// 创建并启动文件监听器。
    ///
    /// # 参数
    ///
    /// * `config` - 监听器配置
    ///
    /// # 返回
    ///
    /// 返回 `FileWatcher` 实例，内部持有接收通道。
    pub fn new(config: WatcherConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let capacity = config.channel_capacity;
        let (tx, rx) = mpsc::channel(capacity);

        let watch_path = config.watch_path.clone();
        let extensions = config.extensions.clone();
        let recursive = config.recursive;
        let debounce_delay = config.debounce_delay;

        // 规范化路径（解析相对路径和符号链接）
        let normalized_path = match watch_path.canonicalize() {
            Ok(path) => path,
            Err(e) => {
                warn!(
                    path = ?watch_path,
                    error = %e,
                    "Failed to canonicalize watch path, using raw path"
                );
                watch_path.clone()
            }
        };

        // 警告状态（提升到结构体，通过 Arc 共享）
        let warned = Arc::new(AtomicBool::new(false));
        let warned_for_closure = Arc::clone(&warned);

        info!(
            path = ?normalized_path,
            recursive = recursive,
            extensions = ?extensions,
            capacity = capacity,
            debounce_ms = debounce_delay.as_millis(),
            "Starting file watcher"
        );

        // 创建 notify 监听器（在后台线程运行）
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Err(e) = &result {
                    warn!(error = %e, "File watcher error");
                    return;
                }

                let event = match result {
                    Ok(event) => event,
                    Err(_) => return,
                };

                // 跳过空路径事件
                if event.paths.is_empty() {
                    return;
                }

                // 过滤事件（使用优化后的 matches_extension）
                if let Some(allowed_exts) = &extensions {
                    let should_process = event
                        .paths
                        .iter()
                        .any(|path| matches_extension(path, allowed_exts));

                    if !should_process {
                        return;
                    }
                }

                // 转换 notify 事件为 FileChange（处理所有路径）
                let changes: Vec<FileChange> = match event.kind {
                    EventKind::Create(_) => event
                        .paths
                        .into_iter()
                        .map(|path| FileChange::Created { path })
                        .collect(),

                    EventKind::Modify(modify_kind) => match modify_kind {
                        ModifyKind::Name(rename_mode) => match rename_mode {
                            RenameMode::Both => {
                                // 重命名事件：paths[0] = from, paths[1] = to
                                let mut paths = event.paths.into_iter();
                                let from = paths.next();
                                let to = paths.next();

                                if let (Some(from_path), Some(to_path)) = (from, to) {
                                    vec![FileChange::Renamed {
                                        from: from_path,
                                        to: to_path,
                                    }]
                                } else {
                                    Vec::new()
                                }
                            }
                            _ => event
                                .paths
                                .into_iter()
                                .map(|path| FileChange::Modified { path })
                                .collect(),
                        },
                        ModifyKind::Data(_) | ModifyKind::Metadata(_) => event
                            .paths
                            .into_iter()
                            .map(|path| FileChange::Modified { path })
                            .collect(),
                        _ => Vec::new(),
                    },

                    EventKind::Remove(_) => event
                        .paths
                        .into_iter()
                        .map(|path| FileChange::Removed { path })
                        .collect(),

                    // 忽略 Access 事件，避免与 Modify 事件重复
                    EventKind::Access(_) => Vec::new(),

                    _ => Vec::new(),
                };

                // 发送所有变更事件
                for change in changes {
                    if tx.blocking_send(change).is_err() {
                        // 通道已满，只在首次警告
                        if warned_for_closure.swap(true, Ordering::SeqCst) == false {
                            warn!(
                                capacity = capacity,
                                "File watcher channel full, events may be lost"
                            );

                            // 只在有图形界面时弹窗
                            if has_display_server() {
                                // 在后台线程显示对话框，避免阻塞文件监听
                                std::thread::spawn(show_warning_dialog);
                            }
                        }

                        break; // 停止发送，避免重复警告
                    }
                }
            },
            notify::Config::default(),
        )?;

        // 开始监听
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(&normalized_path, mode)?;

        debug!("File watcher started successfully");

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            config,
            channel_full_warned: warned,
            debounce: DebounceState::new(debounce_delay),
        })
    }

    /// 异步接收下一个文件变更事件（带防抖）。
    ///
    /// 如果没有事件，会阻塞等待。
    /// 防抖：距离上次事件不足延迟时间时，继续等待。
    pub async fn recv(&mut self) -> Option<FileChange> {
        loop {
            match self.receiver.recv().await {
                Some(change) => {
                    // 更新防抖状态
                    self.debounce.update();

                    // 检查是否应该立即返回
                    if self.debounce.should_fire() {
                        return Some(change);
                    }
                    // 否则继续等待更多事件（防抖窗口内）
                }
                None => return None, // 通道关闭
            }
        }
    }

    /// 尝试非阻塞接收文件变更事件（带防抖）。
    ///
    /// # 返回
    ///
    /// - `Some(change)`：收到事件且防抖窗口已过
    /// - `None`：暂无事件或防抖窗口未过
    pub fn try_recv(&mut self) -> Option<FileChange> {
        // 先收集所有待处理事件
        let mut changes = Vec::new();
        while let Ok(change) = self.receiver.try_recv() {
            changes.push(change);
        }

        if changes.is_empty() {
            return None;
        }

        // 更新防抖状态
        self.debounce.update();

        // 检查防抖窗口
        if self.debounce.should_fire() {
            // 返回最后一个事件（最新的变更）
            changes.into_iter().last()
        } else {
            // 防抖窗口未过，暂不返回
            None
        }
    }

    /// 获取监听器配置。
    pub fn config(&self) -> &WatcherConfig {
        &self.config
    }

    /// 获取待处理的文件变更事件数量。
    pub fn pending_events(&self) -> usize {
        self.receiver.len()
    }

    /// 重置通道满警告状态。
    ///
    /// 调用后，如果再次发生通道满，会重新显示警告。
    pub fn reset_warning_state(&self) {
        self.channel_full_warned.store(false, Ordering::SeqCst);
    }
}

/// 辅助函数：去重文件变更事件（基于路径）。
///
/// 同一文件的多次变更会被合并为最后一次。
///
/// # 参数
///
/// * `changes` - 原始变更事件列表
///
/// # 返回
///
/// 去重后的变更事件列表。
#[must_use]
pub fn deduplicate_changes(changes: Vec<FileChange>) -> Vec<FileChange> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    // 反向遍历，保留每个路径的最后一次变更
    for change in changes.into_iter().rev() {
        let path = change.path().clone();
        if seen.insert(path) {
            result.push(change);
        }
    }

    // 恢复原始顺序
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_extension() {
        let exts: HashSet<String> = ["vue", "js", "ts"].iter().map(|s| s.to_string()).collect();

        // 匹配
        assert!(matches_extension(Path::new("test.vue"), &exts));
        assert!(matches_extension(Path::new("test.js"), &exts));
        assert!(matches_extension(Path::new("test.ts"), &exts));

        // 不匹配
        assert!(!matches_extension(Path::new("test.css"), &exts));
        assert!(!matches_extension(Path::new("test"), &exts));

        // 大小写不敏感
        assert!(matches_extension(Path::new("test.VUE"), &exts));
        assert!(matches_extension(Path::new("test.Js"), &exts));
    }

    #[test]
    fn test_matches_extension_empty() {
        let exts: HashSet<String> = HashSet::new();

        assert!(!matches_extension(Path::new("test.vue"), &exts));
    }

    #[test]
    fn test_deduplicate_changes() {
        let changes = vec![
            FileChange::Modified {
                path: "a.vue".into(),
            },
            FileChange::Modified {
                path: "a.vue".into(),
            },
            FileChange::Modified {
                path: "b.vue".into(),
            },
            FileChange::Created {
                path: "c.vue".into(),
            },
        ];

        let deduped = deduplicate_changes(changes);

        assert_eq!(deduped.len(), 3);
        assert_eq!(deduped[0].path(), &PathBuf::from("a.vue"));
        assert_eq!(deduped[1].path(), &PathBuf::from("b.vue"));
        assert_eq!(deduped[2].path(), &PathBuf::from("c.vue"));
    }

    #[test]
    fn test_deduplicate_changes_empty() {
        let changes: Vec<FileChange> = Vec::new();
        let deduped = deduplicate_changes(changes);
        assert!(deduped.is_empty());
    }

    #[test]
    fn test_deduplicate_changes_single() {
        let changes = vec![FileChange::Modified {
            path: "test.vue".into(),
        }];

        let deduped = deduplicate_changes(changes);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn test_file_change_extension() {
        let created = FileChange::Created {
            path: "test.vue".into(),
        };
        assert_eq!(created.extension(), Some("vue"));

        let no_ext = FileChange::Modified {
            path: "Makefile".into(),
        };
        assert_eq!(no_ext.extension(), None);
    }

    #[test]
    fn test_file_change_path() {
        let renamed = FileChange::Renamed {
            from: "old.vue".into(),
            to: "new.vue".into(),
        };
        assert_eq!(renamed.path(), &PathBuf::from("old.vue"));
    }

    #[test]
    fn test_watcher_config_builder() {
        let config = WatcherConfig::new("/tmp")
            .recursive(false)
            .extensions(vec!["vue".to_string(), "js".to_string()])
            .channel_capacity(1000)
            .debounce_delay(Duration::from_millis(300));

        assert_eq!(config.watch_path, PathBuf::from("/tmp"));
        assert!(!config.recursive);
        assert!(config.extensions.is_some());
        assert_eq!(config.channel_capacity, 1000);
        assert_eq!(config.debounce_delay, Duration::from_millis(300));
    }

    #[test]
    fn test_watcher_config_defaults() {
        let config = WatcherConfig::new("/tmp");

        assert_eq!(config.channel_capacity, DEFAULT_CHANNEL_CAPACITY);
        assert_eq!(
            config.debounce_delay,
            Duration::from_millis(DEFAULT_DEBOUNCE_DELAY_MS)
        );
        assert!(config.recursive);
        assert!(config.extensions.is_none());
    }

    #[test]
    fn test_debounce_state() {
        let mut debounce = DebounceState::new(Duration::from_millis(100));

        // 初始状态，刚创建就检查，时间间隔为 0，不应该触发
        // 但我们应该模拟“收到事件后等待”的场景
        debounce.update(); // 模拟收到事件

        // 立即检查，不应该触发（时间间隔 < 100ms）
        assert!(!debounce.should_fire());

        // 等待超过延迟时间
        std::thread::sleep(Duration::from_millis(150));

        // 超过延迟时间，应该触发
        assert!(debounce.should_fire());
    }
}
