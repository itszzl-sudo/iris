//! 模型下载器 —— 支持断点续传、重启续传、实时进度和网速展示
//!
//! # 架构
//!
//! ```text
//! 缓存目录/
//!   ├── qwen2.5-coder-... .gguf            ← 下载完成的模型
//!   ├── qwen2.5-coder-... .gguf.part       ← 下载中的临时文件
//!   └── .iris-dl-tracker.json              ← 断点续传状态文件（持久化）
//! ```
//!
//! # 流程
//!
//! 1. 完整文件已存在 → 直接返回
//! 2. HEAD 请求获取服务端信息（大小、ETag、Last-Modified）
//! 3. 检查 tracker + .part 文件 → 如可续传则 Range 续传
//! 4. 否则全新 HTTP 流式下载
//! 5. 每 200ms 回调报告进度（百分比 + 网速 + ETA）
//! 6. 每 2s 保存 tracker 到磁盘（支持重启 iris 后续传）
//! 7. 完成后重命名 .part → 正式文件，清除 tracker

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ============================================================
// 公共类型
// ============================================================

/// 下载状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DownloadStatus {
    /// 正在连接
    Connecting,
    /// 正在下载
    Downloading,
    /// 续传中
    Resuming,
    /// 完成
    Completed,
    /// 出错
    Error,
}

/// 下载进度信息
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// 已下载字节数
    pub bytes_downloaded: u64,
    /// 总字节数（0 表示未知）
    pub total_bytes: u64,
    /// 下载百分比 0.0~100.0
    pub percentage: f64,
    /// 瞬时网速（字节/秒）
    pub speed_bytes_per_sec: f64,
    /// 可读的速度文本（如 "12.5 MB/s"）
    pub speed_display: String,
    /// 当前状态
    pub status: DownloadStatus,
    /// 状态描述文本
    pub status_text: String,
    /// 已用时间（秒）
    pub elapsed_secs: f64,
    /// 预估剩余时间（秒）
    pub eta_secs: f64,
    /// 可读的剩余时间文本（如 "约 45 秒"）
    pub eta_display: String,
}

impl DownloadProgress {
    fn new(total_bytes: u64) -> Self {
        Self {
            bytes_downloaded: 0,
            total_bytes,
            percentage: 0.0,
            speed_bytes_per_sec: 0.0,
            speed_display: "0 B/s".into(),
            status: DownloadStatus::Connecting,
            status_text: "连接中...".into(),
            elapsed_secs: 0.0,
            eta_secs: f64::INFINITY,
            eta_display: "计算中...".into(),
        }
    }

    fn format_speed(bytes_per_sec: f64) -> String {
        if bytes_per_sec >= 1_000_000.0 {
            format!("{:.1} MB/s", bytes_per_sec / 1_000_000.0)
        } else if bytes_per_sec >= 1_000.0 {
            format!("{:.0} KB/s", bytes_per_sec / 1_000.0)
        } else {
            format!("{:.0} B/s", bytes_per_sec)
        }
    }

    fn format_eta(secs: f64) -> String {
        if !secs.is_finite() || secs <= 0.0 {
            return "计算中...".into();
        }
        if secs >= 3600.0 {
            let h = (secs / 3600.0) as u64;
            let m = ((secs % 3600.0) / 60.0) as u64;
            format!("约 {} 小时 {} 分钟", h, m)
        } else if secs >= 60.0 {
            format!("约 {:.0} 分钟", secs / 60.0)
        } else if secs >= 10.0 {
            format!("约 {:.0} 秒", secs)
        } else {
            format!("{:.0} 秒", secs)
        }
    }
}

/// 模型下载结果
#[derive(Debug)]
pub struct DownloadResult {
    /// 下载完成的模型路径
    pub model_path: PathBuf,
    /// 最终文件大小
    pub file_size: u64,
    /// 是否通过续传完成
    pub resumed: bool,
}

// ============================================================
// 持久化下载状态（支持跨进程/跨重启续传）
// ============================================================

/// 持久化到磁盘的下载跟踪信息
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadState {
    /// 下载来源 URL
    url: String,
    /// 文件名
    filename: String,
    /// 服务端文件总大小
    total_size: u64,
    /// 已下载大小
    downloaded_size: u64,
    /// ETag（检测服务端文件是否变更）
    etag: Option<String>,
    /// Last-Modified（备用检测）
    last_modified: Option<String>,
    /// 最后更新时间（Unix 时间戳）
    last_updated: u64,
}

impl DownloadState {
    const TRACKER_FILENAME: &str = ".iris-dl-tracker.json";

    fn tracker_path(cache_dir: &PathBuf) -> PathBuf {
        cache_dir.join(Self::TRACKER_FILENAME)
    }

    fn load(cache_dir: &PathBuf) -> Option<Self> {
        let path = Self::tracker_path(cache_dir);
        if !path.exists() {
            return None;
        }
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).ok(),
            Err(_) => {
                warn!("Corrupted tracker file, ignoring");
                None
            }
        }
    }

    fn save(&self, cache_dir: &PathBuf) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let path = Self::tracker_path(cache_dir);
            if let Err(e) = fs::write(&path, &json) {
                warn!("Failed to save download tracker: {}", e);
            }
        }
    }

    fn remove(cache_dir: &PathBuf) {
        let _ = fs::remove_file(Self::tracker_path(cache_dir));
    }
}

// ============================================================
// 网速计算器（滑动窗口平均）
// ============================================================

struct SpeedCalculator {
    window: Duration,
    samples: Vec<(Instant, u64)>,
}

impl SpeedCalculator {
    fn new(window: Duration) -> Self {
        Self { window, samples: Vec::new() }
    }

    fn record(&mut self, now: Instant, bytes: u64) {
        self.samples.push((now, bytes));
        let cutoff = now - self.window;
        self.samples.retain(|&(t, _)| t > cutoff);
    }

    fn speed(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let first = self.samples.first().unwrap();
        let last = self.samples.last().unwrap();
        let duration = (last.0 - first.0).as_secs_f64();
        if duration <= 0.0 {
            return 0.0;
        }
        let total_bytes = last.1 - first.1;
        total_bytes as f64 / duration
    }
}

// ============================================================
// 进度回调
// ============================================================

/// 进度回调函数类型
pub type ProgressCallback = Box<dyn Fn(&DownloadProgress) + Send + Sync>;

// ============================================================
// 模型下载器
// ============================================================

/// 模型下载器
///
/// 支持：
/// - 断点续传（HTTP Range）
/// - 重启续传（持久化 tracker）
/// - 实时下载进度
/// - 网速计算与展示
/// - ETA 预估
pub struct ModelDownloader {
    repo: String,
    filename: String,
    cache_dir: PathBuf,
    client: ureq::Agent,
    progress_callback: Option<ProgressCallback>,
}

impl ModelDownloader {
    /// 创建下载器
    ///
    /// # 参数
    /// * `repo` - HuggingFace 仓库名，如 `"Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF"`
    /// * `filename` - GGUF 文件名，如 `"qwen2.5-coder-0.5b-instruct-q4_k_m.gguf"`
    /// * `cache_dir` - 缓存目录
    pub fn new(
        repo: impl Into<String>,
        filename: impl Into<String>,
        cache_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repo: repo.into(),
            filename: filename.into(),
            cache_dir: cache_dir.into(),
            client: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(15))
                .timeout_read(Duration::from_secs(60))
                .redirects(5)
                .build(),
            progress_callback: None,
        }
    }

    /// 设置进度回调函数
    ///
    /// 回调每秒被调用约 5 次，包含最新进度、网速和 ETA 信息。
    /// 可以在回调中刷新 UI 或打印日志。
    pub fn with_progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&DownloadProgress) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    fn report(&self, progress: &DownloadProgress) {
        if let Some(cb) = &self.progress_callback {
            cb(progress);
        }
    }

    /// 获取或下载模型文件
    ///
    /// 自动处理：
    /// - 检查缓存文件
    /// - 断点续传（HTTP Range）
    /// - 重启后续传（读取 tracker）
    /// - 服务端文件变更自动重下
    pub fn get_or_download(&self) -> Result<DownloadResult> {
        let model_path = self.cache_dir.join(&self.filename);
        let part_path = self.cache_dir.join(format!("{}.part", &self.filename));

        // ----- 1. 检查是否已有完整文件 -----
        if model_path.exists() {
            let size = fs::metadata(&model_path)?.len();
            info!("✅ 模型已缓存: {} ({:.1} MB)",
                model_path.display(), size as f64 / 1_000_000.0);
            return Ok(DownloadResult { model_path, file_size: size, resumed: false });
        }

        // 确保缓存目录存在
        fs::create_dir_all(&self.cache_dir)
            .context("无法创建模型缓存目录")?;

        // ----- 2. 构建下载 URL -----
        // 使用 HuggingFace 直链，避免硬编码预签名 URL
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            self.repo, self.filename
        );
        info!(
            "🌐 模型下载: {}/{}",
            self.repo, self.filename
        );

        // ----- 3. HEAD 请求获取服务端信息 -----
        let (server_size, etag, last_modified) = self.head_request(&url)?;
        info!(
            "📏 服务端文件: {:.1} MB, ETag: {:?}",
            server_size as f64 / 1_000_000.0,
            etag.as_ref().map(|s| &s[..s.len().min(24)])
        );

        let mut progress = DownloadProgress::new(server_size);

        // ----- 4. 检查断点续传可能 -----
        let mut resumed = false;
        let mut initial_offset: u64 = 0;

        if let Some(state) = DownloadState::load(&self.cache_dir) {
            let can_resume = state.url == url
                && state.total_size == server_size
                && etag.as_deref() == state.etag.as_deref()
                && state.downloaded_size > 0
                && state.downloaded_size < server_size
                && part_path.exists();

            if can_resume {
                initial_offset = state.downloaded_size;
                resumed = true;
                progress.status = DownloadStatus::Resuming;

                let pct = initial_offset as f64 / server_size as f64 * 100.0;
                info!(
                    "🔄 检测到未完成的下载，续传中: {:.1} MB / {:.1} MB ({:.1}%)",
                    initial_offset as f64 / 1_000_000.0,
                    server_size as f64 / 1_000_000.0,
                    pct
                );
                progress.status_text = format!("续传中  {:.1}%", pct);
                progress.bytes_downloaded = initial_offset;
                self.report(&progress);
            } else {
                // 服务端文件已变更或不支持续传
                if part_path.exists() {
                    warn!("服务端文件已变更或不支持续传，从头下载");
                    let _ = fs::remove_file(&part_path);
                }
                DownloadState::remove(&self.cache_dir);
            }
        }

        // ----- 5. 执行下载 -----
        let start_time = Instant::now();
        let mut speed_calc = SpeedCalculator::new(Duration::from_secs(5));

        // 打开 .part 文件（续传时追加模式）
        let mut part_file = if resumed && part_path.exists() {
            // 验证 .part 文件大小是否匹配
            let actual_size = fs::metadata(&part_path)?.len();
            if actual_size != initial_offset {
                warn!(".part 文件大小不匹配 (期望={}, 实际={})，从头下载", initial_offset, actual_size);
                let _ = fs::remove_file(&part_path);
                initial_offset = 0;
                resumed = false;
                fs::File::create(&part_path)?
            } else {
                fs::OpenOptions::new().append(true).open(&part_path)?
            }
        } else {
            fs::File::create(&part_path)?
        };

        // 发送 HTTP 请求
        let response = if resumed {
            let range = format!("bytes={}-", initial_offset);
            debug!("发送 Range 请求: {}", range);
            self.client.get(&url)
                .set("Range", &range)
                .call()
                .context("续传请求失败")?
        } else {
            self.client.get(&url)
                .call()
                .context("下载请求失败")?
        };

        // 检查响应状态码
        let status = response.status();
        if resumed && status == 206 {
            debug!("服务端返回 206 Partial Content，续传成功");
        } else if resumed && status == 200 {
            warn!("服务端不支持 Range 请求，从头下载");
            part_file = fs::File::create(&part_path)?;
            resumed = false;
            initial_offset = 0;
            progress.bytes_downloaded = 0;
            progress.status_text = "下载中  0.0%".into();
        } else if status != 200 && status != 206 {
            anyhow::bail!("HTTP {}: {}", status, response.status_text());
        }

        // 流式读取并写入文件
        let mut reader = response.into_reader();
        let mut buffer = [0u8; 65536]; // 64KB 缓冲区
        let mut downloaded = initial_offset;
        let mut last_report = Instant::now();
        let mut last_tracker_save = Instant::now();

        loop {
            let n = reader.read(&mut buffer)
                .context("读取下载流失败")?;
            if n == 0 {
                break; // 下载完成
            }
            part_file.write_all(&buffer[..n])
                .context("写入 .part 文件失败")?;
            downloaded += n as u64;

            // 每 200ms 报告进度
            let now = Instant::now();
            if now - last_report >= Duration::from_millis(200) {
                speed_calc.record(now, downloaded);
                let speed = speed_calc.speed();
                let elapsed = (now - start_time).as_secs_f64();

                progress.bytes_downloaded = downloaded;
                progress.speed_bytes_per_sec = speed;
                progress.speed_display = DownloadProgress::format_speed(speed);
                progress.status = DownloadStatus::Downloading;
                progress.elapsed_secs = elapsed;

                if server_size > 0 {
                    let pct = downloaded as f64 / server_size as f64 * 100.0;
                    progress.percentage = pct;
                    progress.status_text = if resumed {
                        format!("续传中  {:.1}%", pct)
                    } else {
                        format!("下载中  {:.1}%", pct)
                    };
                    // ETA
                    if speed > 0.0 {
                        let remaining = server_size - downloaded;
                        progress.eta_secs = remaining as f64 / speed;
                        progress.eta_display = DownloadProgress::format_eta(progress.eta_secs);
                    }
                } else {
                    progress.percentage = 0.0;
                    progress.eta_secs = f64::INFINITY;
                    progress.eta_display = "未知...".into();
                }

                self.report(&progress);
                last_report = now;
            }

            // 每 2 秒保存 tracker
            if now - last_tracker_save >= Duration::from_secs(2) {
                let state = DownloadState {
                    url: url.clone(),
                    filename: self.filename.clone(),
                    total_size: server_size,
                    downloaded_size: downloaded,
                    etag: etag.clone(),
                    last_modified: last_modified.clone(),
                    last_updated: now.elapsed().as_secs(),
                };
                state.save(&self.cache_dir);
                last_tracker_save = now;
            }
        }

        // 确保数据落盘
        part_file.flush()?;
        drop(part_file);

        // ----- 6. 下载完成，重命名 -----
        // 先保存 final tracker
        let state = DownloadState {
            url: url.clone(),
            filename: self.filename.clone(),
            total_size: server_size,
            downloaded_size: downloaded,
            etag: etag.clone(),
            last_modified: last_modified.clone(),
            last_updated: Instant::now().elapsed().as_secs(),
        };
        state.save(&self.cache_dir);

        // 重命名 .part → 正式文件名
        fs::rename(&part_path, &model_path)
            .context("重命名 .part 文件失败")?;

        // 清除 tracker
        DownloadState::remove(&self.cache_dir);

        // 统计信息
        let total_elapsed = start_time.elapsed().as_secs_f64();
        let avg_speed = if total_elapsed > 0.0 {
            downloaded as f64 / total_elapsed
        } else {
            0.0
        };

        let file_size_mb = downloaded as f64 / 1_000_000.0;
        info!(
            "✅ 下载完成: {:.1} MB, 耗时 {:.1}s (平均 {})",
            file_size_mb,
            total_elapsed,
            DownloadProgress::format_speed(avg_speed)
        );

        // 报告完成进度
        progress.bytes_downloaded = downloaded;
        progress.percentage = 100.0;
        progress.status = DownloadStatus::Completed;
        progress.status_text = "✅ 下载完成".into();
        progress.speed_display = format!("平均 {}", DownloadProgress::format_speed(avg_speed));
        self.report(&progress);

        Ok(DownloadResult { model_path, file_size: downloaded, resumed })
    }

    /// HEAD 请求获取服务端文件元信息
    fn head_request(&self, url: &str) -> Result<(u64, Option<String>, Option<String>)> {
        let response = self.client.head(url)
            .call()
            .context("HEAD 请求失败")?;

        let content_length = response.header("Content-Length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let etag = response.header("ETag").map(|s| s.to_string());
        let last_modified = response.header("Last-Modified").map(|s| s.to_string());

        Ok((content_length, etag, last_modified))
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_format() {
        let mb = DownloadProgress::format_speed(12_500_000.0);
        assert!(mb.contains("MB/s"));
        let kb = DownloadProgress::format_speed(856_000.0);
        assert!(kb.contains("KB/s"));
        let b = DownloadProgress::format_speed(500.0);
        assert!(b.contains("B/s"));
    }

    #[test]
    fn test_eta_format() {
        assert!(DownloadProgress::format_eta(30.0).contains("秒"));
        assert!(DownloadProgress::format_eta(120.0).contains("分钟"));
        assert!(DownloadProgress::format_eta(7200.0).contains("小时"));
    }

    #[test]
    fn test_tracker_save_load() {
        let temp = std::env::temp_dir().join("test-iris-ai-tracker");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let state = DownloadState {
            url: "https://example.com/model.gguf".into(),
            filename: "model.gguf".into(),
            total_size: 1_000_000,
            downloaded_size: 500_000,
            etag: Some("\"abc123\"".into()),
            last_modified: None,
            last_updated: 1234567890,
        };
        state.save(&temp);

        let loaded = DownloadState::load(&temp).unwrap();
        assert_eq!(loaded.url, state.url);
        assert_eq!(loaded.downloaded_size, 500_000);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_speed_calculator() {
        let mut calc = SpeedCalculator::new(Duration::from_secs(10));
        let now = Instant::now();
        calc.record(now, 0);
        calc.record(now + Duration::from_secs(1), 1_000_000);
        let speed = calc.speed();
        assert!(speed > 900_000.0 && speed < 1_100_000.0);
    }
}

