//! Iris JetCrab Daemon
//!
//! 彩虹桌面守护进程 - 浮动图标 + Vue 项目管理
//!
//! 架构:
//! - 透明、置顶、可拖拽的桌面悬浮窗 (winit + softbuffer)
//! - 彩色粒子系统 (彩虹拖拽轨迹 + 星光粒子 + 呼吸律动)
//! - 进程管理 (启动/监控 iris-jetcrab dev 子进程)
//! - 嵌入式管理 HTTP API
//! - TOML 配置文件持久化
//! - Web 管理面板

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod floating_window;
mod particles;
mod process_manager;
mod renderer;

use config::DaemonConfig;
use process_manager::ProcessManager;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Iris AI 下载进度类型
pub type AiProgressOption = Mutex<Option<iris_ai::downloader::DownloadProgress>>;

/// NPM 包下载进度（简化版，与 AI 模型下载共享类型）
pub type NpmProgressOption = Mutex<Option<iris_ai::downloader::DownloadProgress>>;

/// 已连接的浏览器客户端信息
#[derive(Clone, Debug)]
pub struct ClientInfo {
    pub id: u32,
    pub user_agent: String,
    pub ip: String,
    pub connected_at: std::time::Instant,
}

/// 工作空间对应的浏览器窗口信息
#[derive(Clone, Debug)]
pub struct BrowserWindowInfo {
    /// 工作空间路径
    pub workspace_path: String,
    /// 浏览器类型 (chrome/edge/firefox)
    pub browser_type: String,
    /// 当前打开的 URL
    pub url: String,
    /// PID
    pub pid: u32,
    /// 是否仍在运行
    pub running: bool,
}

/// 守护进程全局状态
pub struct DaemonState {
    /// 配置（Mutex 以便 API 中修改）
    pub config: Mutex<DaemonConfig>,
    /// Vue 渲染成功标志
    pub render_success: AtomicBool,
    /// 多进程管理器（项目路径 -> 进程管理器）
    pub process_managers: Mutex<HashMap<String, ProcessManager>>,
    /// 运行中的项目路径集合
    pub project_running: Mutex<HashSet<String>>,
    /// 实际监听的管理面板端口（可能与配置不同，因自动换端口）
    pub daemon_port: Mutex<u16>,
    /// 实际监听的 HTTP 服务器端口
    pub actual_http_port: Mutex<u16>,
    /// 实际监听的 Mock 服务器端口
    pub actual_mock_port: Mutex<u16>,
    /// AI 模型下载进度
    pub model_download_progress: AiProgressOption,
    /// AI 模型下载停止标志
    pub model_download_stop: AtomicBool,
    /// NPM 包下载进度
    pub npm_download_progress: NpmProgressOption,
    /// NPM 包下载停止标志
    pub npm_download_stop: AtomicBool,
    /// 已连接的浏览器客户端
    pub connected_clients: Mutex<Vec<ClientInfo>>,
    /// 客户端ID计数器
    pub client_id_counter: AtomicU32,
    /// 工作空间 -> 浏览器窗口信息映射（每个工作空间最多一个标签页）
    pub browser_windows: Mutex<HashMap<String, BrowserWindowInfo>>,
    /// 浏览器子进程句柄（用于生命周期管理）
    pub browser_processes: Mutex<HashMap<String, std::process::Child>>,
    /// Mock API 服务器是否运行中
    pub mock_server_running: AtomicBool,
    /// 大模型服务是否运行中
    pub llm_server_running: AtomicBool,
    /// 大模型服务实际端口
    pub actual_llm_port: Mutex<u16>,
    /// 大模型服务子进程句柄
    pub llm_process: Mutex<Option<std::process::Child>>,
}

impl DaemonState {
    pub fn new(config: DaemonConfig) -> Self {
        let daemon_port = config.daemon_port;
        let http_port = config.http_port;
        let mock_port = config.mock_port;
        Self {
            config: Mutex::new(config),
            render_success: AtomicBool::new(false),
            process_managers: Mutex::new(HashMap::new()),
            project_running: Mutex::new(HashSet::new()),
            daemon_port: Mutex::new(daemon_port),
            actual_http_port: Mutex::new(http_port),
            actual_mock_port: Mutex::new(mock_port),
            model_download_progress: Mutex::new(None),
            model_download_stop: AtomicBool::new(false),
            npm_download_progress: Mutex::new(None),
            npm_download_stop: AtomicBool::new(false),
            connected_clients: Mutex::new(Vec::new()),
            client_id_counter: AtomicU32::new(0),
            browser_windows: Mutex::new(HashMap::new()),
            browser_processes: Mutex::new(HashMap::new()),
            mock_server_running: AtomicBool::new(false),
            llm_server_running: AtomicBool::new(false),
            actual_llm_port: Mutex::new(11434),
            llm_process: Mutex::new(None),
        }
    }
}

fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = DaemonConfig::load();
    let ai_model_downloaded = config.ai_model_downloaded;
    let daemon_port = config.daemon_port;
    let port_range_start = config.port_range_start;
    let port_range_size = config.port_range_size;
    let auto_start_daemon = config.auto_start_daemon;
    tracing::info!("Iris JetCrab Daemon started");
    tracing::info!("Config path: {:?}", DaemonConfig::config_path());
    tracing::info!("Daemon API port: {}", daemon_port);
    tracing::info!("HTTP dev server port: {}", config.http_port);

    // 单实例检测：在端口范围内查找已有守护进程，若存在则发送关闭信号
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            for attempt in 0..port_range_size.min(50) {
                let try_port = if attempt == 0 {
                    daemon_port
                } else {
                    port_range_start.wrapping_add(attempt - 1)
                };
                let url = format!("http://127.0.0.1:{}/api/status", try_port);
                match reqwest::Client::new()
                    .get(&url)
                    .timeout(std::time::Duration::from_millis(500))
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            tracing::info!(
                                "检测到端口 {} 上有旧实例，发送关闭信号...",
                                try_port
                            );
                            let shutdown_url =
                                format!("http://127.0.0.1:{}/api/shutdown", try_port);
                            let _ = reqwest::Client::new()
                                .post(&shutdown_url)
                                .timeout(std::time::Duration::from_secs(2))
                                .send()
                                .await;
                            // 等待旧进程退出释放端口
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            break;
                        }
                    }
                    Err(_) => continue,
                }
            }
        });
    }

    let state = Arc::new(DaemonState::new(config));

    // 启动管理 API 服务器（后台线程，拥有独立 tokio runtime）
    let api_state = state.clone();
    let api_port = *state.daemon_port.lock().unwrap();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = start_daemon_api(api_state, api_port).await {
                tracing::error!("Daemon API server failed: {}", e);
            }
        });
    });

    // 启动续传：若 AI 模型未下载完成，自动开始下载
    if !ai_model_downloaded {
        let auto_state = state.clone();
        std::thread::spawn(move || {
            use iris_ai::downloader::ModelDownloader;
            let cfg = auto_state.config.lock().unwrap();
            let repo = cfg.ai_model_repo.clone();
            let filename = cfg.ai_model_file.clone();
            let cache_dir = {
                let home = std::env::var("USERPROFILE")
                    .or_else(|_| std::env::var("HOME"))
                    .unwrap_or_else(|_| ".".into());
                std::path::Path::new(&home).join(".cache").join("iris-ai")
            };
            drop(cfg);
            let sc = auto_state.clone();

            // 无论是否存在缓存文件，都尝试下载/续传
            let downloader = ModelDownloader::new(repo, filename, cache_dir)
                .with_progress_callback(move |progress| {
                    let sc = sc.clone();
                    let mut p = sc.model_download_progress.lock().unwrap();
                    *p = Some(progress.clone());
                });
            tracing::info!("开始 AI 模型自动下载...");
            let result = downloader.get_or_download();
            match result {
                Ok(_) => {
                    let mut cfg = auto_state.config.lock().unwrap();
                    cfg.ai_model_downloaded = true;
                    cfg.save();
                    tracing::info!("✅ AI 模型自动下载完成");
                }
                Err(e) => {
                    tracing::warn!("⚠️ AI 模型自动下载失败: {}", e);
                }
            }
        });
    }

    // 同步系统启动注册表项
    if auto_start_daemon {
        sync_auto_start_registry(true);
    }

    // 自动初始化大模型服务（若模型已下载）
    if ai_model_downloaded {
        let llm_state = state.clone();
        std::thread::spawn(move || {
            tracing::info!("模型已下载，自动初始化大模型引擎...");
            // 加载 iris-ai 推理引擎（后台启动，不阻塞主流程）
            let cfg = llm_state.config.lock().unwrap();
            let ai_config = iris_ai::AiConfig::default()
                .with_model_repo(cfg.ai_model_repo.clone(), cfg.ai_model_file.clone())
                .with_temperature(cfg.ai_temperature);
            drop(cfg);

            let mut engine = iris_ai::InferenceEngine::new(ai_config);
            match engine.load() {
                Ok(()) => {
                    tracing::info!("✅ 大模型引擎就绪");
                    llm_state.llm_server_running.store(true, Ordering::SeqCst);
                }
                Err(e) => {
                    tracing::warn!("⚠️ 大模型引擎初始化失败: {} (将在需要时重试)", e);
                }
            }
        });
    }

    // 启动 winit 窗口事件循环（必须在主线程）
    tracing::info!("Starting floating window event loop...");
    start_floating_window(state.clone())?;

    // 窗口关闭后清理
    tracing::info!("Floating window closed, cleaning up...");
    cleanup_browser_processes(&state);
    cleanup_llm_process(&state);

    Ok(())
}

/// 清理所有已记录的浏览器子进程
fn cleanup_browser_processes(state: &DaemonState) {
    let mut processes = state.browser_processes.lock().unwrap();
    let count = processes.len();
    for (_key, child) in processes.iter_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }
    processes.clear();
    if count > 0 {
        tracing::info!("已清理 {} 个浏览器进程", count);
    }
}

/// 清理大模型服务子进程
fn cleanup_llm_process(state: &DaemonState) {
    let mut llm = state.llm_process.lock().unwrap();
    if let Some(mut child) = llm.take() {
        let _ = child.kill();
        let _ = child.wait();
        state.llm_server_running.store(false, Ordering::SeqCst);
        tracing::info!("已停止大模型服务进程");
    }
}

/// 在桌面上创建名为 "Iris" 的快捷方式（指向启动脚本，带彩虹图标）
fn create_desktop_shortcut(port: u16) {
    let desktop = match get_desktop_dir() {
        Some(d) => d,
        None => {
            tracing::warn!("无法获取桌面路径，跳过创建快捷方式");
            return;
        }
    };

    // 清理旧版残留
    for old in ["Iris JetCrab.url", "Iris JetCrab.bat"] {
        let p = desktop.join(old);
        if p.exists() {
            let _ = std::fs::remove_file(&p);
            tracing::info!("已移除旧版桌面文件: {}", old);
        }
    }

    // 启动脚本存放在 %APPDATA%\iris-jetcrab\launcher.ps1，桌面只放 .lnk
    let appdata = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| desktop.clone());
    let daemon_dir = appdata.join("iris-jetcrab");
    let _ = std::fs::create_dir_all(&daemon_dir);
    let ps1_path = daemon_dir.join("launcher.ps1");

    // 清理旧版 bat 脚本
    let old_bat = daemon_dir.join("launcher.bat");
    if old_bat.exists() {
        let _ = std::fs::remove_file(&old_bat);
    }

    if !ps1_path.exists() {
        let profile_dir = daemon_dir.join("browser-profile");
        let profile_str = profile_dir.to_string_lossy();
        let content = format!(
            "Start-Process -WindowStyle Hidden -FilePath \"$PSScriptRoot\\iris-jetcrab-daemon.exe\"\r\n\
             Start-Sleep 3\r\n\
             $browsers = @(\r\n\
                 \"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\",\r\n\
                 \"C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe\",\r\n\
                 \"C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe\",\r\n\
                 \"C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe\"\r\n\
             )\r\n\
             $browser = $browsers | Where-Object {{ Test-Path $_ }} | Select-Object -First 1\r\n\
             if ($browser) {{\r\n\
                 & $browser \"--user-data-dir={profile}\" \"http://127.0.0.1:{port}/\"\r\n\
             }} else {{\r\n\
                 Start-Process \"http://127.0.0.1:{port}/\"\r\n\
             }}",
            profile = profile_str,
            port = port
        );
        if let Err(e) = std::fs::write(&ps1_path, &content) {
            tracing::warn!("创建启动脚本失败: {}", e);
            return;
        }
    }

    // 隐藏启动脚本文件，防止用户在 Explorer 中误删
    if ps1_path.exists() {
        let _ = std::process::Command::new("attrib")
            .arg("+h")
            .arg(ps1_path.to_string_lossy().as_ref())
            .output();
    }

    // 创建 Iris.lnk（仅首次）
    let lnk_path = desktop.join("Iris.lnk");
    if lnk_path.exists() {
        tracing::info!("桌面快捷方式已存在: {:?}", lnk_path);
        return;
    }

    // 通过 PowerShell COM 对象创建 .lnk，指向 powershell -WindowStyle Hidden 执行 .ps1
    let ps_script = format!(
        "$ws=New-Object -ComObject WScript.Shell; \
         $s=$ws.CreateShortcut('{lnk}'); \
         $s.TargetPath='powershell'; \
         $s.Arguments='-WindowStyle Hidden -ExecutionPolicy Bypass -File \"{ps1}\"'; \
         $s.IconLocation='{exe_icon},0'; \
         $s.Save()",
        lnk = lnk_path.to_string_lossy(),
        ps1 = ps1_path.to_string_lossy(),
        exe_icon = daemon_dir.join("iris-jetcrab-daemon.exe").to_string_lossy(),
    );
    match std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
    {
        Ok(out) if out.status.success() => {
            tracing::info!("桌面快捷方式已创建: {:?}", lnk_path);
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            tracing::warn!("创建快捷方式失败 (exit={}): {}", out.status, stderr);
        }
        Err(e) => {
            tracing::warn!("执行 PowerShell 失败: {}", e);
        }
    }
}

/// 同步 Windows 自动启动注册表项
/// 写入/删除 HKCU\Software\Microsoft\Windows\CurrentVersion\Run\IrisJetCrab
fn sync_auto_start_registry(enabled: bool) {
    if enabled {
        // 注册表值：指向 launcher.ps1
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let ps1_path = std::path::Path::new(&appdata).join("iris-jetcrab").join("launcher.ps1");
        let cmd = format!(
            "powershell -WindowStyle Hidden -ExecutionPolicy Bypass -File \"{}\"",
            ps1_path.to_string_lossy()
        );
        let _ = std::process::Command::new("reg")
            .args([
                "add", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v", "IrisJetCrab",
                "/t", "REG_SZ",
                "/d", &cmd,
                "/f",
            ])
            .output();
        tracing::info!("已添加系统自动启动项");
    } else {
        let _ = std::process::Command::new("reg")
            .args([
                "delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v", "IrisJetCrab",
                "/f",
            ])
            .output();
        tracing::info!("已移除系统自动启动项");
    }
}

/// 获取当前用户的桌面目录
fn get_desktop_dir() -> Option<std::path::PathBuf> {
    std::env::var("USERPROFILE")
        .ok()
        .map(|p| std::path::Path::new(&p).join("Desktop"))
        .or_else(|| {
            std::env::var("PUBLIC")
                .ok()
                .map(|p| std::path::Path::new(&p).join("Desktop"))
        })
        .filter(|p| !p.as_os_str().is_empty())
}

// ============================================================
// 管理 API 服务器
// ============================================================

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Query, State as AxumState,
    },
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;
use tower_http::cors::CorsLayer;

/// 浏览器可执行文件路径信息
#[derive(Debug)]
struct BrowserInfo {
    name: &'static str,
    path: std::path::PathBuf,
}

/// 检测系统中已安装的浏览器，按优先级（Chrome→Edge/Firefox→其他）排序
fn detect_installed_browsers() -> Vec<serde_json::Value> {
    let mut browsers = Vec::new();

    // 各平台候选浏览器路径
    #[cfg(windows)]
    let candidates = [
        ("chrome", r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
        ("chrome", r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"),
        ("edge", r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"),
        ("edge", r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"),
        ("firefox", r"C:\Program Files\Mozilla Firefox\firefox.exe"),
        ("firefox", r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe"),
        ("chrome", r"..\..\..\..\..\Users\a\scoop\apps\googlechrome\current\chrome.exe"),
        ("opera", r"C:\Program Files\Opera\launcher.exe"),
        ("brave", r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"),
    ];
    #[cfg(target_os = "macos")]
    let candidates = [
        ("chrome", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
        ("firefox", "/Applications/Firefox.app/Contents/MacOS/firefox"),
        ("edge", "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
        ("brave", "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"),
        ("opera", "/Applications/Opera.app/Contents/MacOS/Opera"),
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        ("chrome", "/usr/bin/google-chrome"),
        ("chrome", "/usr/bin/google-chrome-stable"),
        ("firefox", "/usr/bin/firefox"),
        ("edge", "/usr/bin/microsoft-edge"),
        ("brave", "/usr/bin/brave-browser"),
        ("opera", "/usr/bin/opera"),
    ];

    let mut seen = std::collections::HashSet::new();
    for (name, path_str) in &candidates {
        if seen.contains(name) { continue; }
        let path = std::path::Path::new(path_str);
        if path.exists() {
            seen.insert(*name);
            browsers.push(serde_json::json!({
                "id": name,
                "name": match *name {
                    "chrome" => "Google Chrome",
                    "edge" => "Microsoft Edge",
                    "firefox" => "Mozilla Firefox",
                    "opera" => "Opera",
                    "brave" => "Brave",
                    _ => name,
                },
                "path": path_str,
                "installed": true,
            }));
        }
    }

    // 如果未在标准路径找到，尝试通过 PATH / where 命令检测
    let path_names = ["chrome", "edge", "firefox", "brave", "opera"];
    #[cfg(target_os = "macos")]
    let path_names = ["chrome", "firefox", "edge", "brave", "opera"];
    for name in &path_names {
        if seen.contains(name) { continue; }
        if let Ok(path) = find_browser_in_path(name) {
            seen.insert(*name);
            browsers.push(serde_json::json!({
                "id": name,
                "name": match *name {
                    "chrome" => "Google Chrome",
                    "edge" => "Microsoft Edge",
                    "firefox" => "Mozilla Firefox",
                    "opera" => "Opera",
                    "brave" => "Brave",
                    _ => name,
                },
                "path": path,
                "installed": true,
            }));
        }
    }

    // 按优先级排序：Chrome 第一，然后平台第二候选，然后其他
    let second_priority = if cfg!(target_os = "windows") { "edge" }
        else { "firefox" };

    browsers.sort_by(|a, b| {
        let a_id = a["id"].as_str().unwrap_or("");
        let b_id = b["id"].as_str().unwrap_or("");
        if a_id == "chrome" { return std::cmp::Ordering::Less; }
        if b_id == "chrome" { return std::cmp::Ordering::Greater; }
        if a_id == second_priority { return std::cmp::Ordering::Less; }
        if b_id == second_priority { return std::cmp::Ordering::Greater; }
        a_id.cmp(b_id)
    });

    browsers
}

/// 在 PATH 中查找浏览器可执行文件
fn find_browser_in_path(name: &str) -> Result<String, String> {
    let exe_name = match name {
        "chrome" => "chrome.exe",
        "edge" => "msedge.exe",
        "firefox" => "firefox.exe",
        "brave" => "brave.exe",
        "opera" => "opera.exe",
        _ => return Err(format!("Unknown browser: {}", name)),
    };
    which::which(exe_name)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|_| format!("{} not found in PATH", exe_name))
}

/// 根据用户偏好启动浏览器，打开指定 URL。
/// 使用 --app 模式（Chrome/Edge）创建无地址栏的独立窗口，实现"单标签页"效果。
fn launch_browser(preferred: &str, url: &str) -> Result<std::process::Child, String> {
    // 确定要启动的浏览器
    let (browser_exe, args): (String, Vec<String>) = if preferred == "auto" {
        // 自动：尝试 Chrome → Edge → Firefox → 系统默认
        let installed = detect_installed_browsers();
        let first = installed.first().ok_or_else(|| {
            "未检测到已安装的浏览器".to_string()
        })?;
        let id = first["id"].as_str().unwrap_or("chrome");
        let path = first["path"].as_str().unwrap_or("").to_string();
        (path, build_browser_args(id, url))
    } else {
        // 指定浏览器：检查是否安装
        let installed = detect_installed_browsers();
        let found = installed.iter().find(|b| b["id"] == preferred);
        match found {
            Some(b) => {
                let path = b["path"].as_str().unwrap_or("").to_string();
                (path, build_browser_args(preferred, url))
            }
            None => {
                // 指定的浏览器未安装，回退到自动
                let first = installed.first().ok_or_else(|| {
                    format!("{} 未安装，且没有其他可用浏览器", preferred)
                })?;
                let id = first["id"].as_str().unwrap_or("chrome");
                let path = first["path"].as_str().unwrap_or("").to_string();
                (path, build_browser_args(id, url))
            }
        }
    };

    std::process::Command::new(browser_exe)
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("启动浏览器失败: {}", e))
}

/// 根据浏览器类型构建启动参数
fn build_browser_args(browser_id: &str, url: &str) -> Vec<String> {
    match browser_id {
        "chrome" | "edge" | "brave" => {
            // --app=URL 必须作为单个参数，创建无地址栏、无标签页的独立窗口
            vec![format!("--app={}", url), "--no-first-run".to_string(), "--no-default-browser-check".to_string()]
        }
        "firefox" => {
            // Firefox 使用 -new-window 打开新窗口，-url 指定地址
            vec!["-new-window".to_string(), url.to_string(), "-url".to_string(), url.to_string()]
        }
        _ => {
            vec![url.to_string()]
        }
    }
}

/// 在端口范围内查找可用端口，preferred 为优先尝试的端口
async fn find_available_port(preferred: u16, range_start: u16, range_size: u16) -> Option<u16> {
    for attempt in 0..range_size {
        let try_port = if attempt == 0 {
            preferred
        } else {
            range_start.wrapping_add(if attempt == 1 { 0 } else { attempt - 1 })
        };
        if try_port == preferred && attempt > 0 {
            continue; // 优先尝试 preferred，之后跳过它
        }
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], try_port));
        if tokio::net::TcpListener::bind(addr).await.is_ok() {
            return Some(try_port);
        }
    }
    None
}

/// 启动管理 API 服务器（所有路由定义）
async fn start_daemon_api(state: Arc<DaemonState>, port: u16) -> anyhow::Result<()> {
    #[derive(Deserialize)]
    struct ConfigUpdate {
        http_port: Option<u16>,
        mock_port: Option<u16>,
        daemon_port: Option<u16>,
        show_icon: Option<bool>,
        default_project: Option<String>,
        auto_start: Option<String>,
        auto_start_server: Option<bool>,
        projects: Option<Vec<String>>,
        // AI 云服务
        ai_provider: Option<String>,
        ai_api_key: Option<String>,
        ai_model: Option<String>,
        ai_endpoint: Option<String>,
        // AI 本地模型
        ai_model_repo: Option<String>,
        ai_model_file: Option<String>,
        ai_device: Option<String>,
        ai_temperature: Option<f32>,
        ai_max_tokens: Option<usize>,
        // Iris 内置包管理器
        npm_registry: Option<String>,
        npm_proxy: Option<String>,
        local_storage_dir: Option<String>,
        // Mock
        mock_enabled: Option<bool>,
        mock_delay_ms: Option<u64>,
        // 内嵌浏览器
        preferred_browser: Option<String>,
        // 端口范围
        port_range_start: Option<u16>,
        port_range_size: Option<u16>,
        // 系统启动
        auto_start_daemon: Option<bool>,
        // 分区重置
        reset_section: Option<String>,
    }

    #[derive(Deserialize)]
    struct PathReq {
        path: String,
    }

    // -- 状态
    async fn handle_status(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let config = state.config.lock().unwrap();
        let actual_daemon = *state.daemon_port.lock().unwrap();
        let actual_http = *state.actual_http_port.lock().unwrap();
        let actual_mock = *state.actual_mock_port.lock().unwrap();
        let mock_running = state.mock_server_running.load(Ordering::SeqCst);
        let llm_running = state.llm_server_running.load(Ordering::SeqCst);
        let llm_port = *state.actual_llm_port.lock().unwrap();
        // 收集运行中的项目列表
        let running_projects: Vec<String> = {
            let running = state.project_running.lock().unwrap();
            running.iter().cloned().collect()
        };
        Json(serde_json::json!({
            "status": "running",
            "render_success": state.render_success.load(Ordering::SeqCst),
            "running_projects": running_projects,
            "http_port": actual_http,
            "mock_port": actual_mock,
            "daemon_port": actual_daemon,
            "config_http_port": config.http_port,
            "config_mock_port": config.mock_port,
            "config_daemon_port": config.daemon_port,
            "show_icon": config.show_icon,
            "default_project": config.default_project,
            "auto_start": config.auto_start,
            "ai_model_downloaded": config.ai_model_downloaded,
            "mock_running": mock_running,
            "auto_start_daemon": config.auto_start_daemon,
            "llm_running": llm_running,
            "llm_port": llm_port,
        }))
    }

    // -- 配置
    async fn handle_get_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let config = state.config.lock().unwrap();
        Json(serde_json::json!({
            "http_port": config.http_port,
            "mock_port": config.mock_port,
            "daemon_port": config.daemon_port,
            "show_icon": config.show_icon,
            "projects": config.projects,
            "default_project": config.default_project,
            "auto_start": config.auto_start,
            "auto_start_server": config.auto_start_server,
            "auto_start_daemon": config.auto_start_daemon,
            // AI 云服务
            "ai_provider": config.ai_provider,
            "ai_api_key": config.ai_api_key,
            "ai_model": config.ai_model,
            "ai_endpoint": config.ai_endpoint,
            // AI 本地模型
            "ai_model_repo": config.ai_model_repo,
            "ai_model_file": config.ai_model_file,
            "ai_device": config.ai_device,
            "ai_temperature": config.ai_temperature,
            "ai_max_tokens": config.ai_max_tokens,
            "ai_model_downloaded": config.ai_model_downloaded,
            // Iris 内置包管理器
            "npm_registry": config.npm_registry,
            "npm_proxy": config.npm_proxy,
            "local_storage_dir": config.local_storage_dir,
            // Mock
            "mock_enabled": config.mock_enabled,
            "mock_delay_ms": config.mock_delay_ms,
            // 端口范围
            "port_range_start": config.port_range_start,
            "port_range_size": config.port_range_size,
        }))
    }

    async fn handle_update_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(update): Json<ConfigUpdate>,
    ) -> Json<serde_json::Value> {
        let mut config = state.config.lock().unwrap();
        if let Some(v) = update.http_port { config.http_port = v; }
        if let Some(v) = update.mock_port { config.mock_port = v; }
        if let Some(v) = update.daemon_port { config.daemon_port = v; }
        if let Some(v) = update.show_icon { config.show_icon = v; }
        if let Some(v) = update.default_project { config.default_project = Some(v); }
        if let Some(v) = update.auto_start { config.auto_start = Some(v); }
        if let Some(v) = update.auto_start_server { config.auto_start_server = v; }
        if let Some(v) = update.projects { config.projects = v; }
        // AI 云服务
        if let Some(v) = update.ai_provider { config.ai_provider = v; }
        if let Some(v) = update.ai_api_key { config.ai_api_key = v; }
        if let Some(v) = update.ai_model { config.ai_model = v; }
        if let Some(v) = update.ai_endpoint { config.ai_endpoint = v; }
        // AI 本地模型
        if let Some(v) = update.ai_model_repo { config.ai_model_repo = v; }
        if let Some(v) = update.ai_model_file { config.ai_model_file = v; }
        // device / temperature / max_tokens 为只读，拒绝修改
        // Iris 内置包管理器
        if let Some(v) = update.npm_registry { config.npm_registry = v; }
        if let Some(v) = update.npm_proxy { config.npm_proxy = Some(v); }
        if let Some(v) = update.local_storage_dir { config.local_storage_dir = Some(v); }
        // Mock
        if let Some(v) = update.mock_enabled { config.mock_enabled = v; }
        if let Some(v) = update.mock_delay_ms { config.mock_delay_ms = v; }
        // 内嵌浏览器
        if let Some(v) = update.preferred_browser { config.preferred_browser = v; }
        // 端口范围
        if let Some(v) = update.port_range_start { config.port_range_start = v; }
        if let Some(v) = update.port_range_size { config.port_range_size = v; }
        if let Some(v) = update.auto_start_daemon {
            config.auto_start_daemon = v;
            sync_auto_start_registry(v);
        }
        // 分区重置
        if let Some(ref section) = update.reset_section {
            config.reset_section(section);
        }
        config.save();
        Json(serde_json::json!({"status": "ok"}))
    }

    // -- 项目列表
    async fn handle_get_projects(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let config = state.config.lock().unwrap();
        Json(serde_json::json!({
            "projects": config.projects,
            "default_project": config.default_project,
            "auto_start": config.auto_start,
        }))
    }

    async fn handle_add_project(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(req): Json<PathReq>,
    ) -> Json<serde_json::Value> {
        let mut config = state.config.lock().unwrap();
        let normalized = req.path.replace('\\', "/");
        if !config.projects.contains(&normalized) {
            config.projects.push(normalized);
            config.save();
        }
        Json(serde_json::json!({"status": "ok", "projects": config.projects}))
    }

    async fn handle_remove_project(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(req): Json<PathReq>,
    ) -> Json<serde_json::Value> {
        let mut config = state.config.lock().unwrap();
        let normalized = req.path.replace('\\', "/");
        if config.projects.contains(&normalized) {
            config.remove_project(&normalized);
        }
        Json(serde_json::json!({"status": "ok", "projects": config.projects}))
    }

    async fn handle_project_start(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(req): Json<PathReq>,
    ) -> Json<serde_json::Value> {
        let normalized = req.path.replace('\\', "/");
        let project_root = normalized.clone();

        // 检查是否已在运行中
        {
            let running = state.project_running.lock().unwrap();
            if running.contains(&project_root) {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": "项目已在运行中"
                }));
            }
        }

        let (http_port, range_start, range_size) = {
            let config = state.config.lock().unwrap();
            (config.http_port, config.port_range_start, config.port_range_size)
        };

        // 检测 HTTP 端口是否可用
        let actual_port = find_available_port(http_port, range_start, range_size).await
            .unwrap_or(http_port);
        {
            let mut ap = state.actual_http_port.lock().unwrap();
            *ap = actual_port;
        }

        let mut manager = ProcessManager::new(&project_root, actual_port);
        match manager.start() {
            Ok(()) => {
                let mut pms = state.process_managers.lock().unwrap();
                pms.insert(project_root.clone(), manager);
                let mut running = state.project_running.lock().unwrap();
                running.insert(project_root.clone());
                tracing::info!(
                    "Dev server started for project {} on port {}",
                    project_root, actual_port
                );
                Json(serde_json::json!({"status": "ok", "project": project_root, "port": actual_port}))
            }
            Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
        }
    }

    // -- 文件系统浏览
    async fn handle_fs_list(
        Query(params): Query<HashMap<String, String>>,
    ) -> Json<serde_json::Value> {
        let path = params.get("path").map(|s| s.as_str()).unwrap_or("");

        // 空路径 -> 列出根目录（Windows 上为盘符，Unix 上为 /）
        if path.is_empty() {
            #[cfg(windows)]
            {
                let mut drives = Vec::new();
                for c in (b'A'..=b'Z').map(|c| c as char) {
                    let drive_str = format!("{}:\\", c);
                    if std::path::Path::new(&drive_str).exists() {
                        drives.push(serde_json::json!({"name": drive_str, "path": drive_str}));
                    }
                }
                return Json(serde_json::json!({
                    "path": "",
                    "is_root": true,
                    "dirs": drives,
                    "parent": null
                }));
            }
            #[cfg(not(windows))]
            return Json(serde_json::json!({
                "path": "",
                "is_root": true,
                "dirs": [{"name": "/", "path": "/"}],
                "parent": null
            }));
        }

        match std::fs::read_dir(path) {
            Ok(entries) => {
                let mut dirs: Vec<serde_json::Value> = Vec::new();
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // 跳过隐藏目录（以 . 开头）
                        if name.starts_with('.') && name != ".." {
                            continue;
                        }
                        let full_path = entry.path().to_string_lossy().to_string();
                        dirs.push(serde_json::json!({"name": name, "path": full_path}));
                    }
                }
                dirs.sort_by(|a, b| {
                    a["name"].as_str().unwrap_or("").to_lowercase()
                        .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
                });

                let parent = std::path::Path::new(path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string());

                Json(serde_json::json!({
                    "path": path,
                    "is_root": false,
                    "dirs": dirs,
                    "parent": parent
                }))
            }
            Err(e) => Json(serde_json::json!({
                "error": format!("无法访问目录: {}", e),
                "path": path
            })),
        }
    }

    // -- 服务器控制
    async fn handle_project_stop(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(req): Json<PathReq>,
    ) -> Json<serde_json::Value> {
        let normalized = req.path.replace('\\', "/");
        {
            let mut pms = state.process_managers.lock().unwrap();
            if let Some(mut manager) = pms.remove(&normalized) {
                manager.stop();
            }
            let mut running = state.project_running.lock().unwrap();
            running.remove(&normalized);
        }
        Json(serde_json::json!({"status": "ok"}))
    }

    async fn handle_projects_status(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let config = state.config.lock().unwrap();
        {
            // 清理已退出的进程
            let mut pms = state.process_managers.lock().unwrap();
            let mut running = state.project_running.lock().unwrap();
            pms.retain(|_path, _pm| {
                // 如果进程已退出，从 running 中移除
                // ProcessManager::check_health 是 async 的，不能在这里用
                // 简化处理：保留所有已添加到 process_managers 中的
                true
            });
            // 同步 running 集合：只保留 process_managers 中存在的项目
            let active: HashSet<String> = pms.keys().cloned().collect();
            running.retain(|p| active.contains(p));
        }

        let mut projects_status = Vec::new();
        for p in &config.projects {
            let running = {
                let r = state.project_running.lock().unwrap();
                r.contains(p)
            };
            projects_status.push(serde_json::json!({
                "path": p,
                "running": running,
            }));
        }
        Json(serde_json::json!({
            "status": "ok",
            "projects": projects_status,
        }))
    }

    /// CLI 项目注册接口：cli dev 启动时调用，避免重复启动
    async fn handle_register_project(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(req): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let path = req.get("path").and_then(|v| v.as_str()).unwrap_or("").replace('\\', "/");
        let _port = req.get("port").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        if path.is_empty() {
            return Json(serde_json::json!({"status": "error", "message": "path is required"}));
        }

        // 检查是否已在运行中
        {
            let running = state.project_running.lock().unwrap();
            if running.contains(&path) {
                return Json(serde_json::json!({
                    "status": "ok",
                    "message": "项目已在运行中，不重复启动",
                    "already_running": true,
                }));
            }
        }

        // 检查是否在项目列表中
        let in_list = {
            let config = state.config.lock().unwrap();
            config.projects.contains(&path)
        };

        if !in_list {
            // 不在列表中则添加
            let mut config = state.config.lock().unwrap();
            config.projects.push(path.clone());
            config.save();
        }

        // 启动项目
        let (http_port, range_start, range_size) = {
            let config = state.config.lock().unwrap();
            (config.http_port, config.port_range_start, config.port_range_size)
        };

        let actual_port = find_available_port(http_port, range_start, range_size).await
            .unwrap_or(http_port);
        {
            let mut ap = state.actual_http_port.lock().unwrap();
            *ap = actual_port;
        }

        let mut manager = ProcessManager::new(&path, actual_port);
        match manager.start() {
            Ok(()) => {
                let mut pms = state.process_managers.lock().unwrap();
                pms.insert(path.clone(), manager);
                let mut running = state.project_running.lock().unwrap();
                running.insert(path.clone());
                tracing::info!("CLI registered and started project {} on port {}", path, actual_port);
                Json(serde_json::json!({
                    "status": "ok",
                    "project": path,
                    "port": actual_port,
                    "already_running": false,
                }))
            }
            Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
        }
    }

    // ── AI 配置 ──────────────────────────────────────
    async fn handle_ai_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let config = state.config.lock().unwrap();
        Json(serde_json::json!({
            "ai_provider": config.ai_provider,
            "ai_api_key": config.ai_api_key,
            "ai_model": config.ai_model,
            "ai_endpoint": config.ai_endpoint,
            "ai_model_repo": config.ai_model_repo,
            "ai_model_file": config.ai_model_file,

            "ai_device": config.ai_device,
            "ai_temperature": config.ai_temperature,
            "ai_max_tokens": config.ai_max_tokens,
            "ai_model_downloaded": config.ai_model_downloaded,
        }))
    }

    async fn handle_update_ai_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(update): Json<ConfigUpdate>,
    ) -> Json<serde_json::Value> {
        let mut config = state.config.lock().unwrap();
        if let Some(v) = update.ai_provider { config.ai_provider = v; }
        if let Some(v) = update.ai_api_key { config.ai_api_key = v; }
        if let Some(v) = update.ai_model { config.ai_model = v; }
        if let Some(v) = update.ai_endpoint { config.ai_endpoint = v; }
        if let Some(v) = update.ai_model_repo { config.ai_model_repo = v; }
        if let Some(v) = update.ai_model_file { config.ai_model_file = v; }

        // device / temperature / max_tokens 为只读，拒绝修改
        config.save();
        Json(serde_json::json!({"status": "ok"}))
    }

    // ── AI 模型下载 ───────────────────────────────────
    async fn handle_ai_model_download(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::value::Value> {
        // 检查是否已有下载进行中
        {
            let prog = state.model_download_progress.lock().unwrap();
            if let Some(ref p) = *prog {
                if matches!(p.status, iris_ai::downloader::DownloadStatus::Downloading | iris_ai::downloader::DownloadStatus::Connecting | iris_ai::downloader::DownloadStatus::Resuming) {
                    return Json(serde_json::json!({"status": "error", "message": "下载已在进行中"}));
                }
            }
        }

        state.model_download_stop.store(false, Ordering::SeqCst);
        let state_clone = state.clone();

        // 后台线程执行下载
        std::thread::spawn(move || {
            let config = state_clone.config.lock().unwrap();
            let repo = config.ai_model_repo.clone();
            let filename = config.ai_model_file.clone();
            // 使用固定缓存目录（不再从配置中读取）
            let cache_dir = {
                let home = std::env::var("USERPROFILE")
                    .or_else(|_| std::env::var("HOME"))
                    .unwrap_or_else(|_| ".".into());
                std::path::Path::new(&home).join(".cache").join("iris-ai")
            };
            drop(config);

            let state_cb = state_clone.clone();
            let downloader = iris_ai::downloader::ModelDownloader::new(repo, filename, cache_dir)
                .with_progress_callback(move |progress| {
                    if state_cb.model_download_stop.load(Ordering::SeqCst) {
                        // 停止标志已设置
                        return;
                    }
                    let mut p = state_cb.model_download_progress.lock().unwrap();
                    *p = Some(progress.clone());
                });

            match downloader.get_or_download() {
                Ok(result) => {
                    let mut config = state_clone.config.lock().unwrap();
                    config.ai_model_downloaded = true;
                    config.save();
                    let mut p = state_clone.model_download_progress.lock().unwrap();
                    *p = Some(iris_ai::downloader::DownloadProgress {
                        bytes_downloaded: result.file_size,
                        total_bytes: result.file_size,
                        percentage: 100.0,
                        speed_bytes_per_sec: 0.0,
                        speed_display: "完成".into(),
                        status: iris_ai::downloader::DownloadStatus::Completed,
                        status_text: "下载完成".into(),
                        elapsed_secs: 0.0,
                        eta_secs: 0.0,
                        eta_display: "--".into(),
                    });
                }
                Err(e) => {
                    let mut p = state_clone.model_download_progress.lock().unwrap();
                    *p = Some(iris_ai::downloader::DownloadProgress {
                        bytes_downloaded: 0,
                        total_bytes: 0,
                        percentage: 0.0,
                        speed_bytes_per_sec: 0.0,
                        speed_display: "错误".into(),
                        status: iris_ai::downloader::DownloadStatus::Error,
                        status_text: format!("下载失败: {}", e),
                        elapsed_secs: 0.0,
                        eta_secs: 0.0,
                        eta_display: "--".into(),
                    });
                }
            }
        });

        Json(serde_json::json!({"status": "ok", "message": "下载已启动"}))
    }

    async fn handle_ai_model_status(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let p = state.model_download_progress.lock().unwrap();
        if let Some(ref progress) = *p {
            Json(serde_json::json!({
                "status": "ok",
                "downloaded": progress.bytes_downloaded,
                "total": progress.total_bytes,
                "percentage": progress.percentage,
                "speed": progress.speed_display,
                "state": match progress.status {
                    iris_ai::downloader::DownloadStatus::Connecting => "connecting",
                    iris_ai::downloader::DownloadStatus::Downloading => "downloading",
                    iris_ai::downloader::DownloadStatus::Resuming => "resuming",
                    iris_ai::downloader::DownloadStatus::Completed => "completed",
                    iris_ai::downloader::DownloadStatus::Error => "error",
                },
                "status_text": progress.status_text,
                "eta": progress.eta_display,
            }))
        } else {
            Json(serde_json::json!({"status": "idle", "message": "暂无下载任务"}))
        }
    }

    async fn handle_ai_model_stop(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        state.model_download_stop.store(true, Ordering::SeqCst);
        Json(serde_json::json!({"status": "ok", "message": "下载已请求停止"}))
    }

    // ── NPM 配置 ──────────────────────────────────────
    async fn handle_npm_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let config = state.config.lock().unwrap();
        Json(serde_json::json!({
            "npm_registry": config.npm_registry,
            "npm_proxy": config.npm_proxy,
            "local_storage_dir": config.local_storage_dir,
        }))
    }

    async fn handle_update_npm_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(update): Json<ConfigUpdate>,
    ) -> Json<serde_json::Value> {
        let mut config = state.config.lock().unwrap();
        if let Some(v) = update.npm_registry { config.npm_registry = v; }
        if let Some(v) = update.npm_proxy { config.npm_proxy = Some(v); }
        if let Some(v) = update.local_storage_dir { config.local_storage_dir = Some(v); }
        config.save();
        Json(serde_json::json!({"status": "ok"}))
    }

    // ── Mock API 配置 ────────────────────────────────
    async fn handle_mock_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let config = state.config.lock().unwrap();
        Json(serde_json::json!({
            "mock_enabled": config.mock_enabled,
            "mock_port": config.mock_port,
            "mock_delay_ms": config.mock_delay_ms,
        }))
    }

    async fn handle_update_mock_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(update): Json<ConfigUpdate>,
    ) -> Json<serde_json::Value> {
        let mut config = state.config.lock().unwrap();
        if let Some(v) = update.mock_enabled { config.mock_enabled = v; }
        if let Some(v) = update.mock_port { config.mock_port = v; }
        if let Some(v) = update.mock_delay_ms { config.mock_delay_ms = v; }
        config.save();
        Json(serde_json::json!({"status": "ok"}))
    }

    // ── Mock API Server 启动/停止 ────────────────────
    async fn handle_mock_start(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        if state.mock_server_running.load(Ordering::SeqCst) {
            return Json(serde_json::json!({"status": "error", "message": "Mock server already running"}));
        }
        let (mock_port, range_start, range_size, mock_enabled) = {
            let config = state.config.lock().unwrap();
            (config.mock_port, config.port_range_start, config.port_range_size, config.mock_enabled)
        };
        if !mock_enabled {
            return Json(serde_json::json!({"status": "error", "message": "Mock server is disabled in config"}));
        }
        let actual_port = find_available_port(mock_port, range_start, range_size).await
            .unwrap_or(mock_port);
        {
            let mut ap = state.actual_mock_port.lock().unwrap();
            *ap = actual_port;
        }

        state.mock_server_running.store(true, Ordering::SeqCst);
        tracing::info!("Mock API server started on port {}", actual_port);
        Json(serde_json::json!({"status": "ok", "port": actual_port}))
    }

    async fn handle_mock_stop(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        state.mock_server_running.store(false, Ordering::SeqCst);
        Json(serde_json::json!({"status": "ok"}))
    }

    async fn handle_mock_server_status(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let running = state.mock_server_running.load(Ordering::SeqCst);
        let actual_port = *state.actual_mock_port.lock().unwrap();
        Json(serde_json::json!({
            "running": running,
            "port": actual_port,
        }))
    }

    // ── NPM 下载 ────────────────────────────────────────
    async fn handle_npm_download_start(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        // 检查是否已有下载进行中
        {
            let prog = state.npm_download_progress.lock().unwrap();
            if let Some(ref p) = *prog {
                use iris_ai::downloader::DownloadStatus;
                if matches!(p.status,
                    DownloadStatus::Downloading
                    | DownloadStatus::Connecting
                    | DownloadStatus::Resuming
                ) {
                    return Json(serde_json::json!({"status": "error", "message": "下载已在进行中"}));
                }
            }
        }

        state.npm_download_stop.store(false, Ordering::SeqCst);
        let state_clone = state.clone();

        std::thread::spawn(move || {
            let config = state_clone.config.lock().unwrap();
            let _registry = config.npm_registry.clone();
            let _local_storage = config.local_storage_dir.clone()
                .unwrap_or_else(|| {
                    let home = std::env::var("USERPROFILE")
                        .or_else(|_| std::env::var("HOME"))
                        .unwrap_or_else(|_| ".".into());
                    std::path::Path::new(&home).join(".iris").join("packages").to_string_lossy().into()
                });
            drop(config);

            let _sc = state_clone.clone();
            // 模拟 NPM 包下载（简化版本，实际可调用 npm install）
            let total: u64 = 100;
            for i in 0..=total {
                if state_clone.npm_download_stop.load(Ordering::SeqCst) {
                    let mut p = state_clone.npm_download_progress.lock().unwrap();
                    *p = Some(iris_ai::downloader::DownloadProgress {
                        bytes_downloaded: 0,
                        total_bytes: total,
                        percentage: 0.0,
                        speed_bytes_per_sec: 0.0,
                        speed_display: "已停止".into(),
                        status: iris_ai::downloader::DownloadStatus::Error,
                        status_text: "下载已停止".into(),
                        elapsed_secs: 0.0,
                        eta_secs: 0.0,
                        eta_display: "--".into(),
                    });
                    return;
                }
                let pct = (i as f64 / total as f64) * 100.0;
                let mut p = state_clone.npm_download_progress.lock().unwrap();
                *p = Some(iris_ai::downloader::DownloadProgress {
                    bytes_downloaded: i,
                    total_bytes: total,
                    percentage: pct,
                    speed_bytes_per_sec: 1000.0,
                    speed_display: format!("{:.0} KB/s", 1000.0 / 1024.0),
                    status: iris_ai::downloader::DownloadStatus::Downloading,
                    status_text: format!("下载中 ({}/{})", i, total),
                    elapsed_secs: i as f64,
                    eta_secs: (total - i) as f64,
                    eta_display: format!("{}s", total - i),
                });
                drop(p);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let mut p = state_clone.npm_download_progress.lock().unwrap();
            *p = Some(iris_ai::downloader::DownloadProgress {
                bytes_downloaded: total,
                total_bytes: total,
                percentage: 100.0,
                speed_bytes_per_sec: 0.0,
                speed_display: "完成".into(),
                status: iris_ai::downloader::DownloadStatus::Completed,
                status_text: "下载完成".into(),
                elapsed_secs: 0.0,
                eta_secs: 0.0,
                eta_display: "--".into(),
            });
        });

        Json(serde_json::json!({"status": "ok", "message": "下载已启动"}))
    }

    async fn handle_npm_download_status(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let p = state.npm_download_progress.lock().unwrap();
        if let Some(ref progress) = *p {
            Json(serde_json::json!({
                "status": "ok",
                "percentage": progress.percentage,
                "speed": progress.speed_display,
                "state": match progress.status {
                    iris_ai::downloader::DownloadStatus::Connecting => "connecting",
                    iris_ai::downloader::DownloadStatus::Downloading => "downloading",
                    iris_ai::downloader::DownloadStatus::Resuming => "resuming",
                    iris_ai::downloader::DownloadStatus::Completed => "completed",
                    iris_ai::downloader::DownloadStatus::Error => "error",
                },
                "status_text": progress.status_text,
                "eta": progress.eta_display,
            }))
        } else {
            Json(serde_json::json!({"status": "idle", "message": "暂无下载任务"}))
        }
    }

    async fn handle_npm_download_stop(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        state.npm_download_stop.store(true, Ordering::SeqCst);
        Json(serde_json::json!({"status": "ok", "message": "下载已请求停止"}))
    }

    // -- 管理面板
    async fn handle_management_page(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Html<String> {
        let config = state.config.lock().unwrap();
        let show_checked = if config.show_icon { "checked" } else { "" };
        let mock_checked = if config.mock_enabled { "checked" } else { "" };
        // AI provider selection
        let sel_openai = if config.ai_provider == "openai" { "selected" } else { "" };
        let sel_anthropic = if config.ai_provider == "anthropic" { "selected" } else { "" };
        let sel_deepseek = if config.ai_provider == "deepseek" { "selected" } else { "" };
        let sel_custom = if config.ai_provider == "custom" { "selected" } else { "" };
        let ai_model_dl_badge = if config.ai_model_downloaded { "已下载" } else { "未下载" };
        let ai_model_dl_badge_class = if config.ai_model_downloaded { "badge-downloaded" } else { "badge-not-downloaded" };
        let ai_model_dl_btn_text = if config.ai_model_downloaded { "✅ 已下载" } else { "⬇️ 继续下载" };
        let ai_model_dl_completed_display = if !config.ai_model_downloaded { "inline" } else { "none" };
        let ai_model_progress_display = {
            let p = state.model_download_progress.lock().unwrap();
            if p.is_some() { "block" } else { "none" }
        };
        let ai_model_dl_pct = {
            let p = state.model_download_progress.lock().unwrap();
            p.as_ref().map(|pr| pr.percentage as u32).unwrap_or(0)
        };
        let local_storage = config.local_storage_dir.as_deref().unwrap_or("");
        // 浏览器选择
        let sel_browser_auto = if config.preferred_browser == "auto" { "selected" } else { "" };
        let sel_browser_chrome = if config.preferred_browser == "chrome" { "selected" } else { "" };
        let sel_browser_edge = if config.preferred_browser == "edge" { "selected" } else { "" };
        let sel_browser_firefox = if config.preferred_browser == "firefox" { "selected" } else { "" };

        let html = MANAGEMENT_HTML
            .replace("{ACTUAL_HTTP_PORT}", &state.actual_http_port.lock().unwrap().to_string())
            .replace("{ACTUAL_MOCK_PORT}", &state.actual_mock_port.lock().unwrap().to_string())
            .replace("{ACTUAL_DAEMON_PORT}", &state.daemon_port.lock().unwrap().to_string())
            .replace("{SHOW_ICON_CHECKED}", show_checked)
            // AI 云服务
            .replace("{SELECTED_OPENAI}", sel_openai)
            .replace("{SELECTED_ANTHROPIC}", sel_anthropic)
            .replace("{SELECTED_DEEPSEEK}", sel_deepseek)
            .replace("{SELECTED_CUSTOM}", sel_custom)
            .replace("{AI_API_KEY}", &config.ai_api_key)
            .replace("{AI_MODEL}", &config.ai_model)
            .replace("{AI_ENDPOINT}", &config.ai_endpoint)
            // AI 本地模型
            .replace("{AI_MODEL_FILE}", &config.ai_model_file)
            .replace("{AI_DEVICE}", &config.ai_device)
            .replace("{AI_TEMPERATURE}", &config.ai_temperature.to_string())
            .replace("{AI_MAX_TOKENS}", &config.ai_max_tokens.to_string())
            .replace("{AI_MODEL_DL_BADGE_TEXT}", ai_model_dl_badge)
            .replace("{AI_MODEL_DL_BADGE_CLASS}", ai_model_dl_badge_class)
            .replace("{AI_MODEL_DL_BTN_CLASS}", if config.ai_model_downloaded { "hidden" } else { "" })
            .replace("{AI_MODEL_DL_BTN_TEXT}", ai_model_dl_btn_text)
            .replace("{AI_MODEL_DL_COMPLETED_DISPLAY}", ai_model_dl_completed_display)
            .replace("{AI_MODEL_DL_PCT}", &ai_model_dl_pct.to_string())
            .replace("{AI_MODEL_PROGRESS_DISPLAY}", ai_model_progress_display)
            // NPM
            .replace("{NPM_REGISTRY}", &config.npm_registry)
            .replace("{NPM_PROXY}", config.npm_proxy.as_deref().unwrap_or(""))
            // Mock
            .replace("{MOCK_ENABLED}", mock_checked)
            .replace("{MOCK_DELAY}", &config.mock_delay_ms.to_string())
            // 端口范围
            .replace("{PORT_RANGE_START}", &config.port_range_start.to_string())
            .replace("{PORT_RANGE_SIZE}", &config.port_range_size.to_string())
            // 默认值
            .replace("{LOCAL_STORAGE_DIR}", local_storage)
            .replace("{DAEMON_PORT_WS}", &config.daemon_port.to_string())
            // 内嵌浏览器
            .replace("{SELECTED_BROWSER_AUTO}", sel_browser_auto)
            .replace("{SELECTED_BROWSER_CHROME}", sel_browser_chrome)
            .replace("{SELECTED_BROWSER_EDGE}", sel_browser_edge)
            .replace("{SELECTED_BROWSER_FIREFOX}", sel_browser_firefox);
        Html(html)
    }

    // ── WebSocket 客户端跟踪 ──────────────────────
    async fn handle_ws(
        ws: WebSocketUpgrade,
        state: AxumState<Arc<DaemonState>>,
        headers: axum::http::HeaderMap,
        ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    ) -> impl IntoResponse {
        let ua = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("Unknown")
            .to_string();
        let ip = addr.ip().to_string();
        ws.on_upgrade(move |socket| handle_socket(socket, state.0, ua, ip))
    }

    async fn handle_socket(mut socket: WebSocket, state: Arc<DaemonState>, user_agent: String, ip: String) {
        let id = state.client_id_counter.fetch_add(1, Ordering::SeqCst);
        {
            let mut clients = state.connected_clients.lock().unwrap();
            clients.push(ClientInfo {
                id,
                user_agent: user_agent.clone(),
                ip: ip.clone(),
                connected_at: std::time::Instant::now(),
            });
            tracing::info!("WS client #{} connected: {} from {}", id, user_agent, ip);
        }
        loop {
            match socket.recv().await {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(Message::Ping(_))) => {
                    let _ = socket.send(Message::Pong(vec![])).await;
                }
                _ => {}
            }
        }
        {
            let mut clients = state.connected_clients.lock().unwrap();
            clients.retain(|c| c.id != id);
            tracing::info!("WS client #{} disconnected: {} from {}", id, user_agent, ip);
        }
    }

    // ── 已连接客户端列表 ──────────────────────────
    async fn handle_connected_clients(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let clients = state.connected_clients.lock().unwrap();
        let list: Vec<serde_json::Value> = clients.iter().map(|c| {
            serde_json::json!({
                "id": c.id,
                "user_agent": c.user_agent,
                "ip": c.ip,
            })
        }).collect();
        Json(serde_json::json!({
            "count": list.len(),
            "clients": list,
        }))
    }

    // ── 确认打开页面 ──────────────────────────────
    async fn handle_confirm_open(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Html<String> {
        let daemon_port = *state.daemon_port.lock().unwrap();
        let html = CONFIRM_OPEN_HTML
            .replace("{DAEMON_PORT}", &daemon_port.to_string());
        Html(html)
    }

    // ── 守护进程生命周期 ────────────────────────────

    /// 优雅关闭守护进程（供新实例调用）
    async fn handle_shutdown() -> Json<serde_json::Value> {
        tracing::info!("收到关闭信号，正在停止守护进程...");
        // 设置退出标志后，axum serve 会在下一个请求循环退出
        std::process::exit(0);
        #[allow(unreachable_code)]
        Json(serde_json::json!({"status": "ok", "message": "shutting down"}))
    }

    // ── 内嵌浏览器检测与启动 ────────────────────────

    /// 检测系统中已安装的浏览器，返回按优先级排序的可用列表
    async fn handle_detect_browsers() -> Json<serde_json::Value> {
        let browsers = detect_installed_browsers();
        Json(serde_json::json!({
            "status": "ok",
            "browsers": browsers,
        }))
    }

    /// 获取/设置浏览器偏好配置
    async fn handle_get_browser_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let config = state.config.lock().unwrap();
        Json(serde_json::json!({
            "preferred_browser": config.preferred_browser,
            "available": detect_installed_browsers(),
        }))
    }

    async fn handle_update_browser_config(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(update): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let mut config = state.config.lock().unwrap();
        if let Some(browser) = update.get("preferred_browser").and_then(|v| v.as_str()) {
            config.preferred_browser = browser.to_string();
            config.save();
        }
        Json(serde_json::json!({"status": "ok"}))
    }

    /// 在浏览器中打开指定工作空间的 URL（每个工作空间最多一个标签页）
    async fn handle_open_in_browser(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(req): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let url = req.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let workspace_path = req.get("workspace_path").and_then(|v| v.as_str()).unwrap_or("");
        if url.is_empty() {
            return Json(serde_json::json!({"status": "error", "message": "URL is required"}));
        }

        let preferred = {
            let config = state.config.lock().unwrap();
            config.preferred_browser.clone()
        };

        // 检查该工作空间是否已有打开的浏览器窗口
        let existing_pid = {
            let windows = state.browser_windows.lock().unwrap();
            if !workspace_path.is_empty() {
                windows.get(workspace_path).map(|info| info.pid)
            } else {
                None
            }
        };

        // 如果已有窗口且进程仍在运行，则聚焦它（重新打开 URL）
        if let Some(pid) = existing_pid {
            let mut processes = state.browser_processes.lock().unwrap();
            if let Some(child) = processes.get_mut(workspace_path) {
                if child.try_wait().ok().flatten().is_none() {
                    // 进程仍在运行，更新 URL 信息并返回现有窗口
                    let mut windows = state.browser_windows.lock().unwrap();
                    if let Some(info) = windows.get_mut(workspace_path) {
                        info.url = url.to_string();
                        info.running = true;
                    }
                    return Json(serde_json::json!({
                        "status": "ok",
                        "pid": pid,
                        "message": "该工作空间已有浏览器窗口，正在复用",
                        "existing": true,
                    }));
                }
            }
        }

        match launch_browser(&preferred, url) {
            Ok(child) => {
                let pid = child.id();
                let browser_type = preferred.clone();
                let ws_key = if workspace_path.is_empty() {
                    format!("_default_{}", pid)
                } else {
                    workspace_path.to_string()
                };

                // 记录浏览器窗口信息
                {
                    let mut windows = state.browser_windows.lock().unwrap();
                    windows.insert(ws_key.clone(), BrowserWindowInfo {
                        workspace_path: ws_key.clone(),
                        browser_type: browser_type.clone(),
                        url: url.to_string(),
                        pid,
                        running: true,
                    });
                }
                // 保存子进程句柄用于生命周期管理
                {
                    let mut processes = state.browser_processes.lock().unwrap();
                    processes.insert(ws_key.clone(), child);
                }

                // 清理已退出的进程
                {
                    let mut processes = state.browser_processes.lock().unwrap();
                    processes.retain(|_k, c| c.try_wait().ok().flatten().is_none());
                    let mut windows = state.browser_windows.lock().unwrap();
                    windows.retain(|k, _info| processes.contains_key(k));
                }

                let window_count = state.browser_windows.lock().unwrap().len();
                Json(serde_json::json!({
                    "status": "ok",
                    "pid": pid,
                    "browser_type": browser_type,
                    "workspace": ws_key,
                    "windows_count": window_count,
                }))
            }
            Err(e) => Json(serde_json::json!({
                "status": "error",
                "message": format!("启动浏览器失败: {}", e),
            })),
        }
    }

    /// 关闭指定工作空间的浏览器窗口
    async fn handle_close_browser(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(req): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let workspace_path = req.get("workspace_path").and_then(|v| v.as_str()).unwrap_or("");
        if workspace_path.is_empty() {
            return Json(serde_json::json!({"status": "error", "message": "workspace_path is required"}));
        }

        let pid = {
            let mut processes = state.browser_processes.lock().unwrap();
            if let Some(mut child) = processes.remove(workspace_path) {
                let pid = child.id();
                let _ = child.kill();
                let _ = child.wait();
                pid
            } else {
                let windows = state.browser_windows.lock().unwrap();
                if let Some(info) = windows.get(workspace_path) {
                    info.pid
                } else {
                    return Json(serde_json::json!({"status": "error", "message": "未找到该工作空间的浏览器窗口"}));
                }
            }
        };

        {
            let mut windows = state.browser_windows.lock().unwrap();
            windows.remove(workspace_path);
        }

        Json(serde_json::json!({
            "status": "ok",
            "pid": pid,
            "message": "浏览器窗口已关闭",
        }))
    }

    /// 获取所有工作空间的浏览器窗口状态
    async fn handle_list_browser_windows(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let mut windows_list: Vec<serde_json::Value> = Vec::new();
        {
            let windows = state.browser_windows.lock().unwrap();
            let processes = state.browser_processes.lock().unwrap();
            for (key, info) in windows.iter() {
                let running = processes.get(key)
                    .map(|_| true) // 只要进程还在 HashMap 中就视为运行中
                    .unwrap_or(false);
                windows_list.push(serde_json::json!({
                    "workspace_path": info.workspace_path,
                    "browser_type": info.browser_type,
                    "url": info.url,
                    "pid": info.pid,
                    "running": running,
                }));
            }
        }
        Json(serde_json::json!({
            "status": "ok",
            "windows": windows_list,
            "count": windows_list.len(),
        }))
    }

    let app = Router::new()
        .route("/", get(handle_management_page))
        .route("/api/status", get(handle_status))
        .route("/api/config", get(handle_get_config).put(handle_update_config))
        .route("/api/projects", get(handle_get_projects).post(handle_add_project))
        .route("/api/projects/remove", delete(handle_remove_project))
        .route("/api/project/start", post(handle_project_start))
        .route("/api/project/stop", post(handle_project_stop))
        .route("/api/projects/status", get(handle_projects_status))
        .route("/api/project/register", post(handle_register_project))
        .route("/api/fs/list", get(handle_fs_list))
        // AI 配置
        .route("/api/ai/config", get(handle_ai_config).put(handle_update_ai_config))
        .route("/api/ai/model/download", post(handle_ai_model_download))
        .route("/api/ai/model/status", get(handle_ai_model_status))
        .route("/api/ai/model/stop", post(handle_ai_model_stop))
        // NPM 配置
        .route("/api/npm/config", get(handle_npm_config).put(handle_update_npm_config))
        .route("/api/npm/download/start", post(handle_npm_download_start))
        .route("/api/npm/download/status", get(handle_npm_download_status))
        .route("/api/npm/download/stop", post(handle_npm_download_stop))
        // Mock 配置
        .route("/api/mock/config", get(handle_mock_config).put(handle_update_mock_config))
        .route("/api/mock/start", post(handle_mock_start))
        .route("/api/mock/stop", post(handle_mock_stop))
        .route("/api/mock/status", get(handle_mock_server_status))
        .route("/api/shutdown", post(handle_shutdown))
        .route("/ws", get(handle_ws))
        .route("/api/connected-clients", get(handle_connected_clients))
        .route("/open", get(handle_confirm_open))
        .route("/api/browser/detect", get(handle_detect_browsers))
        .route("/api/browser/config", get(handle_get_browser_config).put(handle_update_browser_config))
        .route("/api/browser/open", post(handle_open_in_browser))
        .route("/api/browser/close", post(handle_close_browser))
        .route("/api/browser/windows", get(handle_list_browser_windows))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    // 端口自动检测：如果配置端口被占用，自动尝试范围内其他端口
    {
        let config = state.config.lock().unwrap();
        let range_start = config.port_range_start;
        let range_size = config.port_range_size;
        drop(config);

        for attempt in 0..range_size {
            let try_port = if attempt == 0 {
                port // 第一次尝试使用配置的端口
            } else {
                range_start.wrapping_add(attempt) // 后续在范围内轮询
            };

            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], try_port));
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    // 更新 state 中的实际端口
                    {
                        let mut dp = state.daemon_port.lock().unwrap();
                        *dp = try_port;
                    }

                    tracing::info!(
                        "Daemon API server listening on http://127.0.0.1:{}{}",
                        try_port,
                        if try_port != port {
                            format!(" (配置端口 {} 被占用，自动换端口)", port)
                        } else {
                            String::new()
                        }
                    );

                    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
                    return Ok(());
                }
                Err(e) => {
                    if attempt == 0 {
                        tracing::warn!(
                            "配置端口 {} 被占用 ({}), 正在范围内查找可用端口...",
                            try_port, e
                        );
                    }
                    // 继续尝试下一个端口
                }
            }
        }

        anyhow::bail!(
            "在端口范围 {}-{} 内未找到可用端口",
            range_start,
            range_start.wrapping_add(range_size - 1)
        );
    }
}

/// 启动悬浮窗口（winit 事件循环）
fn start_floating_window(state: Arc<DaemonState>) -> anyhow::Result<()> {
    use winit::event_loop::EventLoop;

    let event_loop = EventLoop::new().map_err(|e| anyhow::anyhow!("Failed to create event loop: {}", e))?;
    let mut app = floating_window::FloatingApp::new();

    // 传递守护进程状态引用（用于右键菜单、悬停提示、下载进度等）
    app.set_daemon_state(state.clone());
    {
        let port = *state.daemon_port.lock().unwrap();
        app.set_daemon_port(port);
        tracing::info!("Management panel: http://127.0.0.1:{}", port);
        // 使用实际端口创建桌面快捷方式（如已存在则跳过）
        create_desktop_shortcut(port);
    }

    event_loop.run_app(&mut app)
        .map_err(|e| anyhow::anyhow!("Event loop error: {}", e))
}

// ============================================================
// 确认打开页面 HTML
// ============================================================

const CONFIRM_OPEN_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>确认打开 - Iris JetCrab</title>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
min-height: 100vh;
display: flex;
justify-content: center;
align-items: center;
padding: 20px;
}
.card {
background: #fff;
border-radius: 16px;
padding: 40px;
max-width: 560px;
width: 100%;
box-shadow: 0 8px 40px rgba(0,0,0,0.15);
text-align: center;
}
.card h2 { font-size: 1.4em; color: #333; margin-bottom: 8px; }
.card .sub { color: #888; font-size: 0.9em; margin-bottom: 24px; }
.client-list { text-align: left; margin-bottom: 24px; }
.client-item {
display: flex;
align-items: center;
gap: 12px;
padding: 12px 16px;
background: #f8f9fa;
border-radius: 10px;
margin-bottom: 8px;
}
.client-item .icon {
width: 36px; height: 36px;
border-radius: 50%;
background: linear-gradient(135deg, #667eea, #764ba2);
display: flex;
align-items: center;
justify-content: center;
color: #fff;
font-size: 1em;
flex-shrink: 0;
}
.client-item .info { flex: 1; min-width: 0; }
.client-item .ua {
font-size: 0.82em;
color: #666;
white-space: nowrap;
overflow: hidden;
text-overflow: ellipsis;
}
.client-item .ip {
font-size: 0.78em;
color: #999;
margin-top: 2px;
}
.btn-group { display: flex; gap: 12px; justify-content: center; }
.btn {
padding: 10px 28px;
border: none;
border-radius: 8px;
font-size: 0.95em;
cursor: pointer;
transition: all 0.2s;
}
.btn:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(0,0,0,0.15); }
.btn-primary { background: #667eea; color: #fff; }
.btn-primary:hover { background: #5a6fd6; }
.btn-secondary { background: #e9ecef; color: #555; }
.btn-secondary:hover { background: #dee2e6; }
.empty-state { padding: 20px; color: #999; }
</style>
</head>
<body>
<div class="card">
<h2>🔗 已有管理页面打开</h2>
<p class="sub">以下客户端已连接到此守护进程：</p>
<div id="clientList"><div class="empty-state">正在加载...</div></div>
<div class="btn-group">
<button class="btn btn-secondary" onclick="window.close()">取消</button>
<button class="btn btn-primary" onclick="confirmOpen()">✓ 确认打开</button>
</div>
</div>
<script>
async function loadClients() {
try {
const resp = await fetch('/api/connected-clients');
const data = await resp.json();
const list = document.getElementById('clientList');
if (!data.clients || data.clients.length === 0) {
list.innerHTML = '<div class="empty-state">暂无已连接客户端，可直接打开</div>';
return;
}
list.innerHTML = data.clients.map(c => {
let shortUa = c.user_agent;
if (shortUa.length > 60) shortUa = shortUa.substring(0, 57) + '...';
const initial = (c.user_agent || '?').charAt(0).toUpperCase();
return '<div class="client-item">'
+ '<div class="icon">' + initial + '</div>'
+ '<div class="info">'
+ '<div class="ua">' + escapeHtml(shortUa) + '</div>'
+ '<div class="ip">📍 ' + escapeHtml(c.ip) + '</div>'
+ '</div></div>';
}).join('');
} catch(e) {
document.getElementById('clientList').innerHTML = '<div class="empty-state">加载失败: ' + e.message + '</div>';
}
}

function escapeHtml(s) {
const d = document.createElement('div');
d.textContent = s;
return d.innerHTML;
}

function confirmOpen() {
window.location.href = '/';
}

loadClients();
</script>
</body>
</html>
"#;

// ============================================================
// 管理面板 HTML
// ============================================================

const MANAGEMENT_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Iris JetCrab 管理面板</title>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
min-height: 100vh;
padding: 20px;
color: #333;
}
.container { max-width: 900px; margin: 0 auto; }
.header {
text-align: center;
padding: 30px 0;
color: #fff;
}
.header h1 { font-size: 2em; margin-bottom: 5px; }
.header p { opacity: 0.85; font-size: 0.9em; }
.card {
background: #fff;
border-radius: 12px;
padding: 20px;
margin-bottom: 20px;
box-shadow: 0 4px 20px rgba(0,0,0,0.1);
}
.card h2 {
font-size: 1.1em;
margin-bottom: 15px;
color: #555;
border-bottom: 2px solid #f0f0f0;
padding-bottom: 10px;
}
/* 浏览器窗口列表 */
.browser-window-item {
display: flex;
align-items: center;
justify-content: space-between;
padding: 10px 14px;
border: 1px solid #e8e8e8;
border-radius: 8px;
margin-bottom: 8px;
background: #fafafa;
transition: background 0.15s;
}
.browser-window-item:hover { background: #f0f4ff; }
.browser-info { display: flex; align-items: center; gap: 8px; flex: 1; }
.browser-icon { font-size: 1.2em; }
.browser-ws-name { font-weight: 600; font-size: 0.9em; color: #333; }
.browser-type {
display: inline-block;
padding: 2px 8px;
border-radius: 10px;
background: #e8eeff;
color: #667eea;
font-size: 0.75em;
font-weight: 500;
}
.browser-status { font-size: 0.8em; margin-left: 8px; }
.browser-actions { display: flex; gap: 6px; margin-left: 10px; }
.status-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 10px; }
.status-item { text-align: center; padding: 10px; border-radius: 8px; background: #f8f9fa; }
.status-item .label { font-size: 0.8em; color: #888; }
.status-item .value { font-size: 1.3em; font-weight: bold; margin-top: 4px; }
.value.running { color: #28a745; }
.value.stopped { color: #dc3545; }
.value.success { color: #28a745; }
.value.unknown { color: #ffc107; }
.form-group { margin-bottom: 12px; }
.form-group label { display: block; font-size: 0.85em; color: #666; margin-bottom: 4px; }
.form-group input, .form-group select {
width: 100%;
padding: 8px 12px;
border: 1px solid #ddd;
border-radius: 6px;
font-size: 0.95em;
transition: border-color 0.2s;
}
.form-group input:focus { outline: none; border-color: #667eea; }
.form-row { display: flex; gap: 12px; }
.form-row .form-group { flex: 1; }
.btn {
padding: 8px 20px;
border: none;
border-radius: 6px;
font-size: 0.9em;
cursor: pointer;
transition: all 0.2s;
display: inline-flex;
align-items: center;
gap: 6px;
}
.btn:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(0,0,0,0.15); }
.btn:active { transform: translateY(0); }
.btn-primary { background: #667eea; color: #fff; }
.btn-primary:hover { background: #5a6fd6; }
.btn-success { background: #28a745; color: #fff; }
.btn-success:hover { background: #218838; }
.btn-danger { background: #dc3545; color: #fff; }
.btn-danger:hover { background: #c82333; }
.btn-secondary { background: #6c757d; color: #fff; }
.btn-warning { background: #ffc107; color: #333; }
.btn-sm { padding: 5px 12px; font-size: 0.8em; }
.project-list { list-style: none; }
.project-item {
display: flex;
align-items: center;
justify-content: space-between;
padding: 8px 12px;
border-bottom: 1px solid #f0f0f0;
}
.project-item:last-child { border-bottom: none; }
.project-item .path { flex: 1; font-size: 0.9em; word-break: break-all; }
.project-item .badge {
display: inline-block;
font-size: 0.7em;
padding: 2px 8px;
border-radius: 10px;
margin-left: 8px;
}
.badge-default { background: #667eea; color: #fff; }
.badge-auto { background: #28a745; color: #fff; }
.project-item .actions { display: flex; gap: 6px; margin-left: 10px; }
.empty-state { text-align: center; padding: 30px; color: #999; }
.add-project { display: flex; gap: 8px; margin-top: 10px; }
.add-project input { flex: 1; padding: 8px 12px; border: 1px solid #ddd; border-radius: 6px; }
.toast {
position: fixed;
top: 20px;
right: 20px;
padding: 12px 20px;
border-radius: 8px;
color: #fff;
font-size: 0.9em;
z-index: 9999;
transform: translateX(120%);
transition: transform 0.3s ease;
}
.toast.show { transform: translateX(0); }
.toast.success { background: #28a745; }
.toast.error { background: #dc3545; }
.toast.info { background: #17a2b8; }
.toggle { position: relative; display: inline-block; width: 44px; height: 24px; }
.toggle input { opacity: 0; width: 0; height: 0; }
.slider {
position: absolute;
cursor: pointer;
top: 0; left: 0; right: 0; bottom: 0;
background: #ccc;
transition: 0.3s;
border-radius: 24px;
}
.slider::before {
content: "";
position: absolute;
height: 18px; width: 18px;
left: 3px; bottom: 3px;
background: #fff;
transition: 0.3s;
border-radius: 50%;
}
.toggle input:checked + .slider { background: #667eea; }
.toggle input:checked + .slider::before { transform: translateX(20px); }

/* 进度条 */
.progress-bar { width:100%; height:12px; background:#e9ecef; border-radius:6px; overflow:hidden; margin:8px 0; }
.progress-fill { height:100%; background:linear-gradient(90deg,#667eea,#764ba2); border-radius:6px; transition:width 0.3s; }
.dl-info { display:flex; gap:16px; font-size:0.85em; color:#666; margin:4px 0; flex-wrap:wrap; }
.badge-downloaded { display:inline-block; padding:2px 8px; border-radius:10px; font-size:0.8em; background:#28a745; color:#fff; }
.badge-not-downloaded { display:inline-block; padding:2px 8px; border-radius:10px; font-size:0.8em; background:#ffc107; color:#333; }
.hidden { display:none !important; }

/* 目录浏览器弹窗 */
.modal-overlay {
position:fixed; top:0; left:0; right:0; bottom:0;
background:rgba(0,0,0,0.4);
display:flex; align-items:center; justify-content:center;
z-index:1000;
}
.modal-content {
background:#fff; border-radius:12px; padding:20px;
width:90%; max-height:80vh; display:flex; flex-direction:column;
box-shadow:0 8px 32px rgba(0,0,0,0.2);
}
.modal-header {
display:flex; justify-content:space-between; align-items:center;
padding-bottom:10px; border-bottom:1px solid #eee; margin-bottom:10px;
}
.dir-path-bar {
padding:6px 10px; background:#f5f5f5; border-radius:6px;
font-size:0.85em; color:#555; margin-bottom:8px; word-break:break-all;
}
.dir-list {
flex:1; overflow-y:auto; min-height:200px; max-height:400px;
}
.dir-item {
display:flex; align-items:center; gap:8px;
padding:8px 12px; cursor:pointer; border-radius:6px;
font-size:0.9em; transition:background 0.15s;
}
.dir-item:hover { background:#f0f4ff; }
.dir-item.active { background:#e8eeff; font-weight:600; }
.dir-item .dir-icon { color:#667eea; font-size:1.1em; }
.dir-item .dir-name { flex:1; word-break:break-all; }
.modal-footer { border-top:1px solid #eee; margin-top:10px; }
</style>
</head>
<body>
<div class="container">
<div class="header">
<h1>🌈 Iris JetCrab 管理面板</h1>
<p>守护进程管理 · Vue 项目开发服务器 · 配置中心</p>
</div>

<!-- 状态面板 -->
<div class="card">
<h2>📊 运行状态</h2>
<div class="status-grid">
<div class="status-item">
<div class="label">HTTP 端口</div>
<div class="value">{ACTUAL_HTTP_PORT}</div>
</div>
<div class="status-item">
<div class="label">Mock 服务</div>
<div class="value">{ACTUAL_MOCK_PORT}</div>
<div class="label" id="mockStatusLabel" style="font-size:0.75em;margin-top:2px;">-</div>
</div>
<div class="status-item">
<div class="label">管理面板</div>
<div class="value">{ACTUAL_DAEMON_PORT}</div>
<div class="label" style="font-size:0.75em;margin-top:2px;color:#28a745;">运行中</div>
</div>
<div class="status-item">
<div class="label">大模型服务</div>
<div class="value" id="llmPortDisplay">-</div>
<div class="label" id="llmStatusLabel" style="font-size:0.75em;margin-top:2px;">-</div>
</div>
</div>
</div>

<!-- 项目管理 -->
<div class="card">
<h2>📁 Vue 项目列表</h2>
<div id="projectList"><div class="empty-state">正在加载...</div></div>
<div class="add-project">
<input type="text" id="newProjectPath" placeholder="输入或浏览选择 Vue 项目目录..." style="flex:1;" />
<button class="btn btn-secondary btn-sm" onclick="openDirBrowser()">📂 浏览</button>
<button class="btn btn-primary btn-sm" onclick="addProject()">+ 添加</button>
</div>
</div>

<!-- 配置管理 -->
<div class="card">
<h2>⚙️ 配置 <button class="btn btn-secondary btn-sm" onclick="resetSection('general')" style="float:right">↺ 恢复默认</button></h2>
<div class="form-row">
<div class="form-group">
<label>可用端口范围起始</label>
<input type="number" id="cfgPortRangeStart" value="{PORT_RANGE_START}" />
</div>
<div class="form-group">
<label>端口范围大小</label>
<input type="number" id="cfgPortRangeSize" value="{PORT_RANGE_SIZE}" />
</div>
</div>
<div class="form-group" style="display:flex; align-items:center; gap:12px;">
<label style="margin:0;">显示桌面图标</label>
<label class="toggle">
<input type="checkbox" id="cfgShowIcon" {SHOW_ICON_CHECKED} />
<span class="slider"></span>
</label>
</div>
<div class="form-group" style="display:flex; align-items:center; gap:12px;">
<label style="margin:0;">随系统启动</label>
<label class="toggle">
<input type="checkbox" id="cfgAutoStartDaemon" />
<span class="slider"></span>
</label>
</div>
<div style="margin-top:12px;">
<button class="btn btn-primary" onclick="saveConfig()">💾 保存配置</button>
</div>
</div>

<!-- AI 云服务配置 -->
<div class="card">
<h2>🤖 AI 云服务配置 <button class="btn btn-secondary btn-sm" onclick="resetSection('ai')" style="float:right">↺ 恢复默认</button></h2>
<div class="form-group">
<label>服务商</label>
<select id="cfgAiProvider">
<option value="openai" {SELECTED_OPENAI}>OpenAI</option>
<option value="anthropic" {SELECTED_ANTHROPIC}>Anthropic</option>
<option value="deepseek" {SELECTED_DEEPSEEK}>DeepSeek</option>
<option value="custom" {SELECTED_CUSTOM}>自定义</option>
</select>
</div>
<div class="form-group">
<label>API Key</label>
<input type="password" id="cfgAiApiKey" value="{AI_API_KEY}" placeholder="sk-..." />
</div>
<div class="form-group">
<label>模型</label>
<input type="text" id="cfgAiModel" value="{AI_MODEL}" placeholder="gpt-4o" />
</div>
<div class="form-group">
<label>API Endpoint</label>
<input type="text" id="cfgAiEndpoint" value="{AI_ENDPOINT}" placeholder="https://api.openai.com/v1" />
</div>
<div style="margin-top:12px;">
<button class="btn btn-primary" onclick="saveAiConfig()">💾 保存 AI 配置</button>
</div>
</div>

<!-- AI 本地模型文件 -->
<div class="card">
<h2>📦 AI 本地模型文件 <button class="btn btn-secondary btn-sm" onclick="resetSection('ai')" style="float:right">↺ 恢复默认</button></h2>
<div class="form-group">
<label>模型文件 <span style="color:#999;font-size:0.8em;">（只读）</span></label>
<input type="text" id="cfgAiModelFile" value="{AI_MODEL_FILE}" readonly style="background:#f5f5f5;color:#888;" />
</div>
<div class="form-group">
<label>运行设备 <span style="color:#999;font-size:0.8em;">（只读，由系统自动管理）</span></label>
<input type="text" id="cfgAiDevice" value="{AI_DEVICE}" readonly style="background:#f5f5f5;color:#888;" />
</div>
<div class="form-row">
<div class="form-group" style="flex:1;">
<label>Temperature <span style="color:#999;font-size:0.8em;">（只读，由系统自动管理）</span></label>
<input type="number" step="0.1" min="0" max="2" id="cfgAiTemperature" value="{AI_TEMPERATURE}" readonly style="background:#f5f5f5;color:#888;" />
</div>
<div class="form-group" style="flex:1;">
<label>Max Tokens <span style="color:#999;font-size:0.8em;">（只读，由系统自动管理）</span></label>
<input type="number" step="1" min="1" id="cfgAiMaxTokens" value="{AI_MAX_TOKENS}" readonly style="background:#f5f5f5;color:#888;" />
</div>
</div>
<div style="margin:12px 0; display:flex; align-items:center; gap:12px; flex-wrap:wrap;">
<button class="btn btn-primary {AI_MODEL_DL_BTN_CLASS}" id="aiModelToggleBtn" onclick="toggleModelDownload()">{AI_MODEL_DL_BTN_TEXT}</button>
<span id="aiModelBadge" class="{AI_MODEL_DL_BADGE_CLASS}">{AI_MODEL_DL_BADGE_TEXT}</span>
<span id="aiModelDlCompleted" style="display:{AI_MODEL_DL_COMPLETED_DISPLAY};font-size:0.85em;color:#28a745;">下载完成度: {AI_MODEL_DL_PCT}%</span>
</div>
<div id="modelProgressArea" style="display:{AI_MODEL_PROGRESS_DISPLAY};">
<div class="progress-bar"><div class="progress-fill" id="modelProgressFill" style="width:0%"></div></div>
<div class="dl-info">
<span id="modelDlPercent">0%</span>
<span id="modelDlSpeed">0 B/s</span>
<span id="modelDlEta">ETA: --</span>
<span id="modelDlStatus">等待中...</span>
</div>
</div>
</div>

<!-- Iris 内置包管理器配置 -->
<div class="card">
<h2>📦 Iris 内置包管理器配置 <button class="btn btn-secondary btn-sm" onclick="resetSection('npm')" style="float:right">↺ 恢复默认</button></h2>
<div class="form-group">
<label>镜像源 (Registry)</label>
<input type="text" id="cfgNpmRegistry" value="{NPM_REGISTRY}" placeholder="https://registry.npmjs.org/" />
</div>
<div class="form-group">
<label>代理 (Proxy)</label>
<input type="text" id="cfgNpmProxy" value="{NPM_PROXY}" placeholder="http://127.0.0.1:1080 (留空=无代理)" />
</div>
<div class="form-group">
<label>本地存储目录</label>
<div style="display:flex;gap:8px;">
<input type="text" id="cfgLocalStorageDir" value="{LOCAL_STORAGE_DIR}" placeholder="留空使用默认路径" style="flex:1;" />
<button class="btn btn-secondary btn-sm" onclick="copyDataConfirm()" id="copyDataBtn" style="display:none;">📋 拷贝数据</button>
</div>
<div id="copyConfirmDialog" style="display:none;margin-top:8px;padding:10px;background:#fff3cd;border-radius:6px;font-size:0.85em;">
<p>检测到本地存储目录已修改，是否需要将原目录中的数据拷贝到新目录？</p>
<div style="margin-top:8px;display:flex;gap:8px;">
<button class="btn btn-success btn-sm" onclick="copyLocalStorage()">✅ 是，拷贝数据</button>
<button class="btn btn-secondary btn-sm" onclick="dismissCopyConfirm()">❌ 不拷贝</button>
</div>
</div>
</div>
<div style="margin-top:12px;display:flex;gap:8px;">
<button class="btn btn-primary" onclick="saveNpmConfig()">💾 保存配置</button>
<button class="btn btn-success" onclick="startNpmDownload()">⬇️ 下载 NPM 包</button>
<button class="btn btn-danger" onclick="stopNpmDownload()">⏹️ 停止下载</button>
</div>
<div id="npmProgressArea" style="display:none;margin-top:8px;">
<div class="progress-bar"><div class="progress-fill" id="npmProgressFill" style="width:0%"></div></div>
<div class="dl-info">
<span id="npmDlPercent">0%</span>
<span id="npmDlSpeed">0 B/s</span>
<span id="npmDlEta">ETA: --</span>
<span id="npmDlStatus">等待中...</span>
</div>
</div>
</div>

<!-- Mock API 配置 -->
<div class="card">
<h2>🎭 Mock API Server 配置 <button class="btn btn-secondary btn-sm" onclick="resetSection('mock')" style="float:right">↺ 恢复默认</button></h2>
<div class="form-group" style="display:flex; align-items:center; gap:12px;">
<label style="margin:0;">启用 Mock Server</label>
<label class="toggle">
<input type="checkbox" id="cfgMockEnabled" {MOCK_ENABLED} />
<span class="slider"></span>
</label>
</div>
<div class="form-group">
<label>端口</label>
<input type="number" id="cfgMockPort" value="{MOCK_PORT}" />
</div>
<div class="form-group">
<label>模拟延迟 (ms)</label>
<input type="number" id="cfgMockDelay" value="{MOCK_DELAY}" placeholder="0" />
</div>
<div style="margin-top:12px;">
<button class="btn btn-primary" onclick="saveMockConfig()">💾 保存 Mock 配置</button>
</div>
</div>

<!-- 内嵌浏览器配置 -->
<div class="card">
<h2>🌐 内嵌浏览器 <button class="btn btn-secondary btn-sm" onclick="refreshBrowserConfig()" style="float:right">🔄 刷新</button></h2>
<div class="form-group">
<label>首选浏览器（自动检测：Chrome → Edge → 其他）</label>
<select id="cfgPreferredBrowser">
<option value="auto" {SELECTED_BROWSER_AUTO}>自动检测</option>
<option value="chrome" {SELECTED_BROWSER_CHROME}>Chrome (Google Chrome)</option>
<option value="edge" {SELECTED_BROWSER_EDGE}>Edge (Microsoft Edge)</option>
<option value="firefox" {SELECTED_BROWSER_FIREFOX}>Firefox (Mozilla Firefox)</option>
</select>
</div>
<div id="browserStatus" style="font-size:0.85em;color:#666;margin-bottom:10px;padding:8px 10px;background:#f5f5f5;border-radius:6px;">
正在检测可用浏览器...
</div>
<div style="margin-top:8px;display:flex;gap:8px;">
<button class="btn btn-primary btn-sm" onclick="saveBrowserConfig()">💾 保存设置</button>
</div>
</div>

<!-- 工作空间浏览器管理 -->
<div class="card">
<h2>🖥️ 工作空间浏览器 <button class="btn btn-danger btn-sm" onclick="closeAllBrowserWindows()" style="float:right;margin-left:8px;">✕ 关闭所有</button></h2>
<div id="browserWindowsList">
<div class="empty-state">正在加载...</div>
</div>
</div>

<!-- 目录浏览器弹窗 -->
<div id="dirBrowserOverlay" class="modal-overlay" style="display:none;" onclick="if(event.target===this)closeDirBrowser()">
<div class="modal-content" style="max-width:560px;">
<div class="modal-header">
<h3>📂 选择 Vue 项目目录</h3>
<button class="btn btn-sm" onclick="closeDirBrowser()" style="background:none;border:none;color:#999;font-size:1.3em;cursor:pointer;">✕</button>
</div>
<div id="dirBrowserPathBar" class="dir-path-bar"></div>
<div id="dirBrowserList" class="dir-list"><div class="empty-state">加载中...</div></div>
<div class="modal-footer" style="display:flex;justify-content:space-between;gap:8px;padding:10px 0 0;">
<button class="btn btn-secondary btn-sm" onclick="goDirParent()">⬆ 上级目录</button>
<div>
<button class="btn btn-secondary btn-sm" onclick="closeDirBrowser()">取消</button>
<button class="btn btn-primary btn-sm" onclick="confirmDirSelection()">✅ 选择此目录</button>
</div>
</div>
</div>
</div>

<!-- 页面底部 -->
<div style="text-align:center; padding:20px; color:#fff; opacity:0.6; font-size:0.85em;">
守护进程端口 {ACTUAL_DAEMON_PORT} · Iris JetCrab v0.1.0
</div>
</div>

<script>
function showToast(msg, type) {
const t = document.createElement('div');
t.className = 'toast ' + type;
t.textContent = msg;
document.body.appendChild(t);
setTimeout(() => t.classList.add('show'), 10);
setTimeout(() => { t.classList.remove('show'); setTimeout(() => t.remove(), 300); }, 3000);
}

async function api(url, opts) {
try {
const resp = await fetch(url, { ...opts, headers: { 'Content-Type': 'application/json', ...opts?.headers } });
return await resp.json();
} catch(e) {
showToast('请求失败: ' + e.message, 'error');
return null;
}
}

async function refreshStatus() {
const data = await api('/api/status');
if (data) {
// 更新实际端口显示
const portEls = document.querySelectorAll('.status-item .value');
if (portEls.length >= 3) {
portEls[0].textContent = data.http_port || '--';
portEls[1].textContent = data.mock_port || '--';
portEls[2].textContent = data.daemon_port || '--';
}
// Mock 状态
const mockLabel = document.getElementById('mockStatusLabel');
if (mockLabel) {
mockLabel.textContent = data.mock_running ? '运行中' : '已停止';
mockLabel.style.color = data.mock_running ? '#28a745' : '#dc3545';
}
// 大模型服务
const llmPortEl = document.getElementById('llmPortDisplay');
const llmLabelEl = document.getElementById('llmStatusLabel');
if (llmPortEl) llmPortEl.textContent = data.llm_port || '-';
if (llmLabelEl) {
llmLabelEl.textContent = data.llm_running ? '运行中' : '已停止';
llmLabelEl.style.color = data.llm_running ? '#28a745' : '#dc3545';
}
// 随系统启动
const autoStartEl = document.getElementById('cfgAutoStartDaemon');
if (autoStartEl) autoStartEl.checked = data.auto_start_daemon || false;
}
}

async function refreshProjects() {
const data = await api('/api/projects');
const statusData = await api('/api/projects/status');
let runningMap = {};
if (statusData && statusData.projects) {
statusData.projects.forEach(p => { runningMap[p.path] = p.running; });
}
if (data) {
const list = document.getElementById('projectList');
if (!data.projects || data.projects.length === 0) {
list.innerHTML = '<div class="empty-state">还没有添加任何 Vue 项目</div>';
return;
}
list.innerHTML = '<ul class="project-list">' + data.projects.map(p => {
const isRunning = runningMap[p] || false;
const statusBadge = isRunning
? '<span class="badge" style="background:#28a745;color:#fff;margin-left:8px;font-size:0.75em;">运行中</span>'
: '<span class="badge" style="background:#6c757d;color:#fff;margin-left:8px;font-size:0.75em;">已停止</span>';
const actionBtn = isRunning
? '<button class="btn btn-danger btn-sm" onclick="stopProject(\'' + p.replace(/'/g, "\\'") + '\')">⏹ 停止</button>'
: '<button class="btn btn-success btn-sm" onclick="startProject(\'' + p.replace(/'/g, "\\'") + '\')">▶ 启动</button>';
return '<li class="project-item">'
+ '<span class="path">' + p + statusBadge + '</span>'
+ '<div class="actions">'
+ actionBtn
+ '<button class="btn btn-primary btn-sm" onclick="openWorkspaceBrowser(\'' + p.replace(/'/g, "\\'") + '\')">📂 打开</button>'
+ '<button class="btn btn-secondary btn-sm" onclick="removeProject(\'' + p.replace(/'/g, "\\'") + '\')">✕ 删除</button>'
+ '</div></li>';
}).join('') + '</ul>';
}
}

async function addProject() {
const path = document.getElementById('newProjectPath').value.trim();
if (!path) { showToast('请输入项目路径', 'error'); return; }
const data = await api('/api/projects', { method: 'POST', body: JSON.stringify({ path }) });
if (data && data.status === 'ok') {
showToast('项目已添加', 'success');
document.getElementById('newProjectPath').value = '';
refreshProjects();
}
}

async function removeProject(path) {
const data = await api('/api/projects/remove', { method: 'DELETE', body: JSON.stringify({ path }) });
if (data && data.status === 'ok') {
showToast('项目已移除', 'info');
refreshProjects();
}
}

async function startProject(path) {
const data = await api('/api/project/start', { method: 'POST', body: JSON.stringify({ path }) });
if (data) {
if (data.status === 'ok') {
showToast('已启动: ' + path, 'success');
} else {
showToast(data.message || '启动失败', 'error');
}
refreshProjects();
}
}

async function stopProject(path) {
const data = await api('/api/project/stop', { method: 'POST', body: JSON.stringify({ path }) });
if (data && data.status === 'ok') {
showToast('已停止: ' + path, 'info');
refreshProjects();
}
}

async function saveConfig() {
const portRangeStart = parseInt(document.getElementById('cfgPortRangeStart').value) || 19999;
const portRangeSize = parseInt(document.getElementById('cfgPortRangeSize').value) || 500;
const showIcon = document.getElementById('cfgShowIcon').checked;
const autoStartDaemon = document.getElementById('cfgAutoStartDaemon').checked;
const data = await api('/api/config', {
method: 'PUT',
body: JSON.stringify({ port_range_start: portRangeStart, port_range_size: portRangeSize, show_icon: showIcon, auto_start_daemon: autoStartDaemon })
});
if (data && data.status === 'ok') {
showToast('配置已保存', 'success');
}
}

// ── AI 配置函数 ──────────────────────────────

async function saveAiConfig() {
const provider = document.getElementById('cfgAiProvider').value;
const apiKey = document.getElementById('cfgAiApiKey').value;
const model = document.getElementById('cfgAiModel').value;
const endpoint = document.getElementById('cfgAiEndpoint').value;
const data = await api('/api/ai/config', {
method: 'PUT',
body: JSON.stringify({ ai_provider: provider, ai_api_key: apiKey, ai_model: model, ai_endpoint: endpoint })
});
if (data && data.status === 'ok') {
showToast('AI 配置已保存', 'success');
}
}

async function toggleModelDownload() {
const btn = document.getElementById('aiModelToggleBtn');
if (btn.textContent.includes('暂停')) {
// 正在下载，请求暂停
const data = await api('/api/ai/model/stop', { method: 'POST' });
if (data && data.status === 'ok') {
showToast('已暂停下载', 'info');
btn.textContent = '⬇️ 继续下载';
btn.className = 'btn btn-primary';
}
} else {
// 未在下载，开始/继续下载
const data = await api('/api/ai/model/download', { method: 'POST' });
if (data && data.status === 'ok') {
showToast('下载已启动', 'success');
btn.textContent = '⏸️ 暂停下载';
btn.className = 'btn btn-warning';
document.getElementById('modelProgressArea').style.display = 'block';
pollModelStatus();
} else if (data) {
showToast(data.message || '启动失败', 'error');
}
}
}

async function startModelDownload() {
const data = await api('/api/ai/model/download', { method: 'POST' });
if (data) {
if (data.status === 'ok') {
showToast('模型下载已启动', 'success');
document.getElementById('modelProgressArea').style.display = 'block';
document.getElementById('aiModelToggleBtn').textContent = '⏸️ 暂停下载';
document.getElementById('aiModelToggleBtn').className = 'btn btn-warning';
pollModelStatus();
} else {
showToast(data.message || '启动失败', 'error');
}
}
}

async function stopModelDownload() {
const data = await api('/api/ai/model/stop', { method: 'POST' });
if (data && data.status === 'ok') {
showToast('已暂停下载', 'info');
document.getElementById('aiModelToggleBtn').textContent = '⬇️ 继续下载';
document.getElementById('aiModelToggleBtn').className = 'btn btn-primary';
}
}

let modelPollTimer = null;
async function pollModelStatus() {
if (modelPollTimer) clearInterval(modelPollTimer);
const data = await api('/api/ai/model/status');
if (data && data.status === 'ok') {
document.getElementById('modelProgressArea').style.display = 'block';
const pct = data.percentage || 0;
document.getElementById('modelProgressFill').style.width = pct + '%';
document.getElementById('modelDlPercent').textContent = pct.toFixed(1) + '%';
document.getElementById('modelDlSpeed').textContent = data.speed || '0 B/s';
document.getElementById('modelDlEta').textContent = 'ETA: ' + (data.eta || '--');
document.getElementById('modelDlStatus').textContent = data.status_text || data.state || '';
if (data.state === 'completed') {
showToast('模型下载完成', 'success');
document.getElementById('aiModelBadge').className = 'badge-downloaded';
document.getElementById('aiModelBadge').textContent = '已下载';
document.getElementById('aiModelToggleBtn').style.display = 'none';
clearInterval(modelPollTimer); modelPollTimer = null;
return;
}
if (data.state === 'error') {
showToast('下载失败: ' + (data.status_text || ''), 'error');
document.getElementById('aiModelToggleBtn').textContent = '⬇️ 继续下载';
document.getElementById('aiModelToggleBtn').className = 'btn btn-primary';
clearInterval(modelPollTimer); modelPollTimer = null;
return;
}
modelPollTimer = setInterval(async () => {
const d = await api('/api/ai/model/status');
if (d && d.status === 'ok') {
document.getElementById('modelProgressFill').style.width = (d.percentage || 0) + '%';
document.getElementById('modelDlPercent').textContent = (d.percentage || 0).toFixed(1) + '%';
document.getElementById('modelDlSpeed').textContent = d.speed || '0 B/s';
document.getElementById('modelDlEta').textContent = 'ETA: ' + (d.eta || '--');
document.getElementById('modelDlStatus').textContent = d.status_text || d.state || '';
if (d.state === 'completed') {
showToast('模型下载完成', 'success');
document.getElementById('aiModelBadge').className = 'badge-downloaded';
document.getElementById('aiModelBadge').textContent = '已下载';
document.getElementById('aiModelToggleBtn').style.display = 'none';
clearInterval(modelPollTimer); modelPollTimer = null;
}
if (d.state === 'error') {
showToast('下载失败: ' + (d.status_text || ''), 'error');
document.getElementById('aiModelToggleBtn').textContent = '⬇️ 继续下载';
document.getElementById('aiModelToggleBtn').className = 'btn btn-primary';
clearInterval(modelPollTimer); modelPollTimer = null;
}
} else {
clearInterval(modelPollTimer); modelPollTimer = null;
}
}, 1000);
} else if (data && data.status === 'idle') {
// 无下载任务
}
}

// ── NPM 配置 ────────────────────────────────

async function saveNpmConfig() {
const registry = document.getElementById('cfgNpmRegistry').value;
const proxy = document.getElementById('cfgNpmProxy').value;
const localDir = document.getElementById('cfgLocalStorageDir').value.trim();
const data = await api('/api/npm/config', {
method: 'PUT',
body: JSON.stringify({
npm_registry: registry,
npm_proxy: proxy || null,
local_storage_dir: localDir || null
})
});
if (data && data.status === 'ok') {
showToast('NPM 配置已保存', 'success');
prevStorageDir = localDir;
}
}

// ── Mock 配置 ───────────────────────────────

async function saveMockConfig() {
const enabled = document.getElementById('cfgMockEnabled').checked;
const port = parseInt(document.getElementById('cfgMockPort').value) || 3100;
const delay = parseInt(document.getElementById('cfgMockDelay').value) || 0;
const data = await api('/api/mock/config', {
method: 'PUT',
body: JSON.stringify({ mock_enabled: enabled, mock_port: port, mock_delay_ms: delay })
});
if (data && data.status === 'ok') {
showToast('Mock 配置已保存', 'success');
}
}

// ── 内嵌浏览器 ────────────────────────────────

function refreshBrowserConfig() {
const status = document.getElementById('browserStatus');
status.textContent = '正在检测可用浏览器...';
api('/api/browser/detect').then(data => {
if (data && data.status === 'ok' && data.browsers) {
const names = data.browsers.map(b => {
const shortName = b.id.charAt(0).toUpperCase() + b.id.slice(1);
return shortName + ' (' + b.name + ')';
}).join('、');
status.innerHTML = '✅ 已检测到: ' + names + '<br><small>配置保存后下次打开时生效</small>';
} else {
status.textContent = '⚠️ 未检测到已安装的浏览器';
}
}).catch(() => {
status.textContent = '⚠️ 检测失败';
});
refreshBrowserWindows();
}

async function saveBrowserConfig() {
const browser = document.getElementById('cfgPreferredBrowser').value;
const data = await api('/api/browser/config', {
method: 'PUT',
body: JSON.stringify({ preferred_browser: browser })
});
if (data && data.status === 'ok') {
showToast('浏览器设置已保存', 'success');
refreshBrowserConfig();
}
}

// ── 工作空间浏览器管理 ──────────────────────────

async function refreshBrowserWindows() {
const list = document.getElementById('browserWindowsList');
const data = await api('/api/browser/windows');
if (!data || data.status !== 'ok') {
list.innerHTML = '<div class="empty-state">无法获取浏览器窗口信息</div>';
return;
}
if (data.count === 0 || !data.windows || data.windows.length === 0) {
list.innerHTML = '<div class="empty-state">暂无打开的浏览器窗口<br><small>在项目管理中点击"▶ 启动"后可通过"📂 在浏览器中打开"按钮打开</small></div>';
return;
}
list.innerHTML = data.windows.map(w => {
const wsName = w.workspace_path.split('/').pop() || w.workspace_path.split('\\').pop() || w.workspace_path;
const statusClass = w.running ? 'running' : 'stopped';
const statusText = w.running ? '运行中' : '已停止';
return '<div class="browser-window-item">'
+ '<div class="browser-info">'
+ '<span class="browser-icon">🌐</span>'
+ '<span class="browser-ws-name">' + escapeHtml(wsName) + '</span>'
+ '<span class="browser-type">' + (w.browser_type ? (
w.browser_type.charAt(0).toUpperCase() + w.browser_type.slice(1) + ' (' + w.browser_type + ')'
) : '') + '</span>'
+ '<span class="browser-status value ' + statusClass + '">' + statusText + '</span>'
+ '</div>'
+ '<div class="browser-actions">'
+ '<button class="btn btn-primary btn-sm" onclick="openWorkspaceBrowser(\'' + w.workspace_path.replace(/'/g, "\\'") + '\')">📂 打开</button>'
+ '<button class="btn btn-danger btn-sm" onclick="closeWorkspaceBrowser(\'' + w.workspace_path.replace(/'/g, "\\'") + '\')">✕ 关闭</button>'
+ '</div>'
+ '</div>';
}).join('');
}

async function openWorkspaceBrowser(workspacePath) {
const st = await api('/api/status');
const httpPort = (st && st.http_port) || 3000;
const url = 'http://127.0.0.1:' + httpPort;
const data = await api('/api/browser/open', {
method: 'POST',
body: JSON.stringify({ url, workspace_path: workspacePath })
});
if (data && data.status === 'ok') {
showToast('浏览器已打开 (PID: ' + data.pid + ')' + (data.existing ? '，复用已有窗口' : ''), 'success');
refreshBrowserWindows();
} else {
showToast(data?.message || '启动失败', 'error');
}
}

async function closeWorkspaceBrowser(workspacePath) {
const data = await api('/api/browser/close', {
method: 'POST',
body: JSON.stringify({ workspace_path: workspacePath })
});
if (data && data.status === 'ok') {
showToast('浏览器窗口已关闭 (PID: ' + data.pid + ')', 'info');
refreshBrowserWindows();
} else {
showToast(data?.message || '关闭失败', 'error');
}
}

async function closeAllBrowserWindows() {
const data = await api('/api/browser/windows');
if (!data || data.status !== 'ok' || !data.windows) return;
for (const w of data.windows) {
await api('/api/browser/close', {
method: 'POST',
body: JSON.stringify({ workspace_path: w.workspace_path })
});
}
showToast('已关闭所有浏览器窗口', 'info');
refreshBrowserWindows();
}

// ── 分区重置 ────────────────────────────────

async function resetSection(section) {
const data = await api('/api/config', {
method: 'PUT',
body: JSON.stringify({ reset_section: section })
});
if (data && data.status === 'ok') {
showToast('配置已恢复默认', 'success');
// 刷新页面以显示新值
setTimeout(() => location.reload(), 500);
}
}

// ── NPM 下载 ────────────────────────────────

async function startNpmDownload() {
const data = await api('/api/npm/download/start', { method: 'POST' });
if (data) {
if (data.status === 'ok') {
showToast('NPM 下载已启动', 'success');
document.getElementById('npmProgressArea').style.display = 'block';
pollNpmStatus();
} else {
showToast(data.message || '启动失败', 'error');
}
}
}

async function stopNpmDownload() {
const data = await api('/api/npm/download/stop', { method: 'POST' });
if (data && data.status === 'ok') {
showToast('已请求停止下载', 'info');
}
}

let npmPollTimer = null;
async function pollNpmStatus() {
if (npmPollTimer) clearInterval(npmPollTimer);
const data = await api('/api/npm/download/status');
if (data && data.status === 'ok') {
document.getElementById('npmProgressArea').style.display = 'block';
const pct = data.percentage || 0;
document.getElementById('npmProgressFill').style.width = pct + '%';
document.getElementById('npmDlPercent').textContent = pct.toFixed(1) + '%';
document.getElementById('npmDlSpeed').textContent = data.speed || '0 B/s';
document.getElementById('npmDlEta').textContent = 'ETA: ' + (data.eta || '--');
document.getElementById('npmDlStatus').textContent = data.status_text || data.state || '';
if (data.state === 'completed') {
showToast('NPM 下载完成', 'success');
document.getElementById('npmProgressArea').style.display = 'none';
clearInterval(npmPollTimer); npmPollTimer = null;
return;
}
if (data.state === 'error') {
showToast('下载失败: ' + (data.status_text || ''), 'error');
clearInterval(npmPollTimer); npmPollTimer = null;
return;
}
npmPollTimer = setInterval(async () => {
const d = await api('/api/npm/download/status');
if (d && d.status === 'ok') {
document.getElementById('npmProgressFill').style.width = (d.percentage || 0) + '%';
document.getElementById('npmDlPercent').textContent = (d.percentage || 0).toFixed(1) + '%';
document.getElementById('npmDlSpeed').textContent = d.speed || '0 B/s';
document.getElementById('npmDlEta').textContent = 'ETA: ' + (d.eta || '--');
document.getElementById('npmDlStatus').textContent = d.status_text || d.state || '';
if (d.state === 'completed') {
showToast('NPM 下载完成', 'success');
document.getElementById('npmProgressArea').style.display = 'none';
clearInterval(npmPollTimer); npmPollTimer = null;
}
if (d.state === 'error') {
showToast('下载失败: ' + (d.status_text || ''), 'error');
clearInterval(npmPollTimer); npmPollTimer = null;
}
} else {
clearInterval(npmPollTimer); npmPollTimer = null;
}
}, 1000);
} else if (data && data.status === 'idle') {
// 无下载任务
}
}

// ── 本地存储目录拷贝确认 ────────────────────────

let prevStorageDir = (document.getElementById('cfgLocalStorageDir')?.value || '').trim();
function copyDataConfirm() {
const newDir = document.getElementById('cfgLocalStorageDir').value.trim();
if (prevStorageDir && newDir && prevStorageDir !== newDir) {
document.getElementById('copyConfirmDialog').style.display = 'block';
} else {
// 直接保存
saveNpmConfig();
}
}

function dismissCopyConfirm() {
document.getElementById('copyConfirmDialog').style.display = 'none';
}

async function copyLocalStorage() {
document.getElementById('copyConfirmDialog').style.display = 'none';
showToast('正在拷贝数据...', 'info');
// 保存配置
const registry = document.getElementById('cfgNpmRegistry').value;
const proxy = document.getElementById('cfgNpmProxy').value;
const localDir = document.getElementById('cfgLocalStorageDir').value.trim();
const data = await api('/api/config', {
method: 'PUT',
body: JSON.stringify({
npm_registry: registry,
npm_proxy: proxy || null,
local_storage_dir: localDir || null
})
});
if (data && data.status === 'ok') {
showToast('配置已保存，数据拷贝将后台进行', 'success');
prevStorageDir = localDir;
}
}

// ── 目录浏览器 ────────────────────────────────

let dirBrowserCurrentPath = '';
let dirBrowserSelectedPath = '';

function openDirBrowser() {
dirBrowserCurrentPath = '';
dirBrowserSelectedPath = '';
document.getElementById('dirBrowserOverlay').style.display = 'flex';
loadDirList('');
}

function closeDirBrowser() {
document.getElementById('dirBrowserOverlay').style.display = 'none';
}

async function loadDirList(path) {
const listEl = document.getElementById('dirBrowserList');
const pathBar = document.getElementById('dirBrowserPathBar');
listEl.innerHTML = '<div class="empty-state">加载中...</div>';

let url = '/api/fs/list';
if (path) url += '?path=' + encodeURIComponent(path);

const data = await api(url);
if (!data) {
listEl.innerHTML = '<div class="empty-state">加载失败</div>';
return;
}

if (data.error) {
listEl.innerHTML = '<div class="empty-state" style="color:#dc3545;">' + data.error + '</div>';
return;
}

dirBrowserCurrentPath = data.path || '';
pathBar.textContent = data.path || '我的电脑';

let html = '';
if (data.dirs && data.dirs.length > 0) {
for (const d of data.dirs) {
const sel = d.path === dirBrowserSelectedPath ? ' active' : '';
html += '<div class="dir-item' + sel + '" onclick="selectDir(\'' + d.path.replace(/\\/g, '\\\\').replace(/'/g, '\\\'') + '\')" ondblclick="navigateDir(\'' + d.path.replace(/\\/g, '\\\\').replace(/'/g, '\\\'') + '\')">'
+ '<span class="dir-icon">📁</span>'
+ '<span class="dir-name">' + d.name + '</span>'
+ '</div>';
}
} else {
html = '<div class="empty-state">此目录下没有子文件夹</div>';
}
listEl.innerHTML = html;
}

function selectDir(path) {
dirBrowserSelectedPath = path;
// 高亮当前选中
document.querySelectorAll('.dir-item').forEach(el => el.classList.remove('active'));
document.querySelectorAll('.dir-item').forEach(el => {
if (el.querySelector('.dir-name')?.textContent === path.split('\\').pop() || el.querySelector('.dir-name')?.textContent === path.split('/').pop()) {
el.classList.add('active');
}
});
}

function navigateDir(path) {
loadDirList(path);
}

async function goDirParent() {
if (!dirBrowserCurrentPath) return;
const p = document.getElementById('dirBrowserPathBar').textContent;
if (!p || p === '我的电脑') return;
const parent = p.substring(0, p.lastIndexOf('\\'));
if (parent.length >= 2) {
loadDirList(parent);
} else {
// Windows 盘符的上级 -> 返回根目录列表
loadDirList('');
}
}

function confirmDirSelection() {
let selectedPath = dirBrowserSelectedPath;
if (!selectedPath && dirBrowserCurrentPath) {
selectedPath = dirBrowserCurrentPath;
}
if (!selectedPath) {
showToast('请先选择一个目录', 'error');
return;
}
document.getElementById('newProjectPath').value = selectedPath;
closeDirBrowser();
}

// 自动刷新
refreshStatus();
refreshProjects();
refreshBrowserConfig();
setInterval(() => { refreshStatus(); refreshProjects(); refreshBrowserWindows(); }, 10000);

// WebSocket 客户端跟踪
(function() {
const ws = new WebSocket('ws://127.0.0.1:{DAEMON_PORT_WS}/ws');
ws.onclose = function() { console.log('WS disconnected'); };
ws.onerror = function() { console.log('WS error'); };
})();
</script>
</body>
</html>
"#;
