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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 守护进程全局状态
pub struct DaemonState {
    /// 配置（Mutex 以便 API 中修改）
    pub config: Mutex<DaemonConfig>,
    /// 标记是否正在运行 Vue 服务器
    pub server_running: AtomicBool,
    /// Vue 渲染成功标志
    pub render_success: AtomicBool,
    /// 进程管理器
    pub process_manager: Mutex<Option<ProcessManager>>,
    /// 管理面板端口
    pub daemon_port: u16,
}

impl DaemonState {
    pub fn new(config: DaemonConfig) -> Self {
        let daemon_port = config.daemon_port;
        Self {
            config: Mutex::new(config),
            server_running: AtomicBool::new(false),
            render_success: AtomicBool::new(false),
            process_manager: Mutex::new(None),
            daemon_port,
        }
    }
}

fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = DaemonConfig::load();
    tracing::info!("Iris JetCrab Daemon started");
    tracing::info!("Config path: {:?}", DaemonConfig::config_path());
    tracing::info!("Daemon API port: {}", config.daemon_port);
    tracing::info!("HTTP dev server port: {}", config.http_port);

    let state = Arc::new(DaemonState::new(config));

    // 启动管理 API 服务器（后台线程，拥有独立 tokio runtime）
    let api_state = state.clone();
    let api_port = state.daemon_port;
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = start_daemon_api(api_state, api_port).await {
                tracing::error!("Daemon API server failed: {}", e);
            }
        });
    });

    // 启动 winit 窗口事件循环（必须在主线程）
    tracing::info!("Starting floating window event loop...");
    start_floating_window(state)?;

    Ok(())
}

// ============================================================
// 管理 API 服务器
// ============================================================

use axum::{
    extract::State as AxumState,
    response::{Html, Json},
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;
use tower_http::cors::CorsLayer;

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
        Json(serde_json::json!({
            "status": "running",
            "server_running": state.server_running.load(Ordering::SeqCst),
            "render_success": state.render_success.load(Ordering::SeqCst),
            "http_port": config.http_port,
            "mock_port": config.mock_port,
            "daemon_port": config.daemon_port,
            "show_icon": config.show_icon,
            "default_project": config.default_project,
            "auto_start": config.auto_start,
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

    async fn handle_switch_project(
        AxumState(state): AxumState<Arc<DaemonState>>,
        Json(req): Json<PathReq>,
    ) -> Json<serde_json::Value> {
        let normalized = req.path.replace('\\', "/");
        let project_root = normalized.clone();
        {
            let config = state.config.lock().unwrap();
            if !config.projects.contains(&project_root) {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": "Project not in configured list"
                }));
            }
        }

        let http_port = state.config.lock().unwrap().http_port;
        let mut pm = state.process_manager.lock().unwrap();
        if let Some(ref mut manager) = *pm {
            match manager.switch_project(&project_root) {
                Ok(()) => {
                    state.server_running.store(true, Ordering::SeqCst);
                    Json(serde_json::json!({"status": "ok", "project": project_root}))
                }
                Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
            }
        } else {
            let mut manager = ProcessManager::new(&project_root, http_port);
            match manager.start() {
                Ok(()) => {
                    state.server_running.store(true, Ordering::SeqCst);
                    *pm = Some(manager);
                    Json(serde_json::json!({"status": "ok", "project": project_root}))
                }
                Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
            }
        }
    }

    // -- 服务器控制
    async fn handle_start_server(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let (project, http_port) = {
            let config = state.config.lock().unwrap();
            let project = config.default_project.clone()
                .or_else(|| config.projects.first().cloned());
            (project, config.http_port)
        };

        if let Some(project_root) = project {
            let mut pm = state.process_manager.lock().unwrap();
            if pm.is_some() {
                return Json(serde_json::json!({
                    "status": "error",
                    "message": "Server already running"
                }));
            }
            let mut manager = ProcessManager::new(&project_root, http_port);
            match manager.start() {
                Ok(()) => {
                    state.server_running.store(true, Ordering::SeqCst);
                    *pm = Some(manager);
                    Json(serde_json::json!({"status": "ok", "project": project_root}))
                }
                Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
            }
        } else {
            Json(serde_json::json!({
                "status": "error",
                "message": "No projects configured. Add a project first."
            }))
        }
    }

    async fn handle_stop_server(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let mut pm = state.process_manager.lock().unwrap();
        if let Some(ref mut manager) = *pm {
            manager.stop();
            state.server_running.store(false, Ordering::SeqCst);
        }
        *pm = None;
        Json(serde_json::json!({"status": "ok"}))
    }

    async fn handle_server_health(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Json<serde_json::Value> {
        let running = state.server_running.load(Ordering::SeqCst);
        Json(serde_json::json!({
            "status": if running { "ok" } else { "stopped" },
            "server_running": running,
        }))
    }

    // -- 管理面板
    async fn handle_management_page(
        AxumState(state): AxumState<Arc<DaemonState>>,
    ) -> Html<String> {
        let config = state.config.lock().unwrap();
        let running = state.server_running.load(Ordering::SeqCst);
        let render_ok = state.render_success.load(Ordering::SeqCst);
        let show_checked = if config.show_icon { "checked" } else { "" };
        let html = MANAGEMENT_HTML
            .replace("{HTTP_PORT}", &config.http_port.to_string())
            .replace("{MOCK_PORT}", &config.mock_port.to_string())
            .replace("{DAEMON_PORT}", &config.daemon_port.to_string())
            .replace("{SHOW_ICON_CHECKED}", show_checked)
            .replace("{SERVER_STATUS}", if running { "running" } else { "stopped" })
            .replace("{RENDER_STATUS}", if render_ok { "success" } else { "unknown" });
        Html(html)
    }

    let app = Router::new()
        .route("/", get(handle_management_page))
        .route("/api/status", get(handle_status))
        .route("/api/config", get(handle_get_config).put(handle_update_config))
        .route("/api/projects", get(handle_get_projects).post(handle_add_project))
        .route("/api/projects/remove", delete(handle_remove_project))
        .route("/api/projects/switch", post(handle_switch_project))
        .route("/api/server/start", post(handle_start_server))
        .route("/api/server/stop", post(handle_stop_server))
        .route("/api/server/health", get(handle_server_health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Daemon API server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 启动悬浮窗口（winit 事件循环）
fn start_floating_window(state: Arc<DaemonState>) -> anyhow::Result<()> {
    use winit::event_loop::EventLoop;

    let event_loop = EventLoop::new().map_err(|e| anyhow::anyhow!("Failed to create event loop: {}", e))?;
    let mut app = floating_window::FloatingApp::new();

    // 传递守护进程端口给窗口（用于双击打开管理页面）
    let daemon_port = state.daemon_port;
    app.set_daemon_port(daemon_port);

    tracing::info!("Floating window event loop started");
    tracing::info!("Management panel: http://127.0.0.1:{}", daemon_port);

    event_loop.run_app(&mut app)
        .map_err(|e| anyhow::anyhow!("Event loop error: {}", e))
}

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
<div class="label">Dev Server</div>
<div class="value {SERVER_STATUS}">{SERVER_STATUS}</div>
</div>
<div class="status-item">
<div class="label">Vue 渲染</div>
<div class="value {RENDER_STATUS}">{RENDER_STATUS}</div>
</div>
<div class="status-item">
<div class="label">HTTP 端口</div>
<div class="value">{HTTP_PORT}</div>
</div>
<div class="status-item">
<div class="label">Mock 端口</div>
<div class="value">{MOCK_PORT}</div>
</div>
</div>
<div style="margin-top:15px; display:flex; gap:10px;">
<button class="btn btn-success" onclick="startServer()">▶ 启动服务器</button>
<button class="btn btn-danger" onclick="stopServer()">⏹ 停止服务器</button>
<button class="btn btn-secondary" onclick="refreshStatus()">🔄 刷新</button>
<a href="http://127.0.0.1:{HTTP_PORT}" target="_blank" class="btn btn-warning">🌐 打开应用</a>
</div>
</div>

<!-- 项目管理 -->
<div class="card">
<h2>📁 Vue 项目列表</h2>
<div id="projectList"><div class="empty-state">正在加载...</div></div>
<div class="add-project">
<input type="text" id="newProjectPath" placeholder="输入 Vue 项目目录路径..." />
<button class="btn btn-primary btn-sm" onclick="addProject()">+ 添加</button>
</div>
</div>

<!-- 配置管理 -->
<div class="card">
<h2>⚙️ 配置</h2>
<div class="form-row">
<div class="form-group">
<label>HTTP 服务器端口</label>
<input type="number" id="cfgHttpPort" value="{HTTP_PORT}" />
</div>
<div class="form-group">
<label>Mock API 端口</label>
<input type="number" id="cfgMockPort" value="{MOCK_PORT}" />
</div>
<div class="form-group">
<label>守护进程端口</label>
<input type="number" id="cfgDaemonPort" value="{DAEMON_PORT}" />
</div>
</div>
<div class="form-group" style="display:flex; align-items:center; gap:12px;">
<label style="margin:0;">显示桌面图标</label>
<label class="toggle">
<input type="checkbox" id="cfgShowIcon" {SHOW_ICON_CHECKED} />
<span class="slider"></span>
</label>
</div>
<div style="margin-top:12px;">
<button class="btn btn-primary" onclick="saveConfig()">💾 保存配置</button>
</div>
</div>

<!-- 页面底部 -->
<div style="text-align:center; padding:20px; color:#fff; opacity:0.6; font-size:0.85em;">
守护进程端口 {DAEMON_PORT} · Iris JetCrab v0.1.0
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
document.querySelectorAll('.value.running, .value.stopped').forEach(el => {
el.className = 'value ' + (data.server_running ? 'running' : 'stopped');
el.textContent = data.server_running ? 'running' : 'stopped';
});
document.querySelectorAll('.value.success, .value.unknown').forEach(el => {
el.className = 'value ' + (data.render_success ? 'success' : 'unknown');
el.textContent = data.render_success ? 'success' : 'unknown';
});
}
}

async function refreshProjects() {
const data = await api('/api/projects');
if (data) {
const list = document.getElementById('projectList');
if (!data.projects || data.projects.length === 0) {
list.innerHTML = '<div class="empty-state">还没有添加任何 Vue 项目</div>';
return;
}
list.innerHTML = '<ul class="project-list">' + data.projects.map(p => {
let badges = '';
if (p === data.default_project) badges += '<span class="badge badge-default">默认</span>';
if (p === data.auto_start) badges += '<span class="badge badge-auto">自动</span>';
return '<li class="project-item">'
+ '<span class="path">' + p + badges + '</span>'
+ '<div class="actions">'
+ '<button class="btn btn-success btn-sm" onclick="switchProject(\'' + p.replace(/'/g, "\\'") + '\')">▶ 启动</button>'
+ '<button class="btn btn-warning btn-sm" onclick="setDefault(\'' + p.replace(/'/g, "\\'") + '\')">★ 默认</button>'
+ '<button class="btn btn-danger btn-sm" onclick="removeProject(\'' + p.replace(/'/g, "\\'") + '\')">✕</button>'
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

async function switchProject(path) {
const data = await api('/api/projects/switch', { method: 'POST', body: JSON.stringify({ path }) });
if (data) {
if (data.status === 'ok') {
showToast('已切换至: ' + path, 'success');
} else {
showToast(data.message || '切换失败', 'error');
}
refreshStatus();
}
}

async function setDefault(path) {
const data = await api('/api/config', { method: 'PUT', body: JSON.stringify({ default_project: path }) });
if (data && data.status === 'ok') {
showToast('已设为默认项目', 'success');
refreshProjects();
}
}

async function startServer() {
const data = await api('/api/server/start', { method: 'POST' });
if (data) {
if (data.status === 'ok') {
showToast('服务器已启动', 'success');
} else {
showToast(data.message || '启动失败', 'error');
}
refreshStatus();
}
}

async function stopServer() {
const data = await api('/api/server/stop', { method: 'POST' });
if (data && data.status === 'ok') {
showToast('服务器已停止', 'info');
refreshStatus();
}
}

async function saveConfig() {
const httpPort = parseInt(document.getElementById('cfgHttpPort').value) || 3000;
const mockPort = parseInt(document.getElementById('cfgMockPort').value) || 3100;
const daemonPort = parseInt(document.getElementById('cfgDaemonPort').value) || 19999;
const showIcon = document.getElementById('cfgShowIcon').checked;
const data = await api('/api/config', {
method: 'PUT',
body: JSON.stringify({ http_port: httpPort, mock_port: mockPort, daemon_port: daemonPort, show_icon: showIcon })
});
if (data && data.status === 'ok') {
showToast('配置已保存', 'success');
}
}

// 自动刷新
refreshStatus();
refreshProjects();
setInterval(() => { refreshStatus(); refreshProjects(); }, 10000);
</script>
</body>
</html>
"#;
