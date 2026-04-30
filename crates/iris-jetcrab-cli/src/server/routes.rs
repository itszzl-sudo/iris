//! HTTP 路由处理器

use axum::{
    response::{Html, Json, IntoResponse, Response},
    extract::{State, Path},
};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::http::{StatusCode, header, Uri};
use serde_json::json;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::broadcast;
use futures_util::SinkExt;
use tracing::{info, debug, warn};
use crate::server::compiler_cache::CompilerCache;
use crate::server::hmr::{WebSocketManager, HmrEvent};
use crate::server::ai_inspector;
use crate::utils;
use anyhow::Result;

/// 服务器状态类型
pub type ServerState = (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>);

/// 主页处理器 - 返回 index.html
pub async fn index_handler(
    State(state): State<ServerState>,
) -> Html<String> {
    let (cache, _enable_hmr, _ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    let cache_lock = cache.lock().await;
    let project_root = &cache_lock.project_root;
    
    let html = generate_index_html(project_root);
    let html = ai_inspector::inject_inspector_overlay(&html);
    Html(html)
}

/// Vue 模块按需编译处理器
/// 
/// 路由: GET /@vue/*path
/// 示例: GET /@vue/App.vue → 编译 src/App.vue
pub async fn vue_module_handler(
    State(state): State<ServerState>,
    Path(path): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let (cache, _enable_hmr, _ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    
    // 获取或编译模块
    let module = {
        let cache_lock = cache.lock().await;
        cache_lock.get_or_compile(&path).await
    };
    
    match module {
        Ok(module) => {
            // 返回编译后的 JavaScript
            let response = json!({
                "code": module.script,
                "styles": module.styles.iter().map(|s| {
                    json!({
                        "code": s.code,
                        "scoped": s.scoped
                    })
                }).collect::<Vec<_>>(),
                "deps": module.deps
            });
            
            Ok(Json(response))
        }
        Err(e) => {
            let error_response = json!({
                "error": format!("Failed to compile module {}: {}", path, e)
            });
            Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// 静态资源处理器
/// 
/// 路由: GET /assets/*path
/// 示例: GET /assets/logo.png → 返回项目中的静态文件
pub async fn static_handler(
    State(state): State<ServerState>,
    Path(path): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let (cache, _enable_hmr, _ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    let cache_lock = cache.lock().await;
    let project_root = &cache_lock.project_root;
    
    let file_path = project_root.join("public").join(&path);
    
    // 检查文件是否存在
    if !file_path.exists() {
        let error_response = json!({
            "error": format!("File not found: {}", path)
        });
        return Err((axum::http::StatusCode::NOT_FOUND, Json(error_response)));
    }
    
    // 读取文件内容
    match tokio::fs::read(&file_path).await {
        Ok(content) => {
            // 根据扩展名设置 Content-Type
            let content_type = get_content_type(&path);
            
            // 返回文件内容（这里简化为 base64 编码）
            let response = json!({
                "content": base64_encode(&content),
                "content_type": content_type
            });
            
            Ok(Json(response))
        }
        Err(e) => {
            let error_response = json!({
                "error": format!("Failed to read file: {}", e)
            });
            Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

/// 项目信息 API 处理器
/// 
/// 路由: GET /api/project-info
pub async fn project_info_handler(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let (cache, enable_hmr, _ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    let cache_lock = cache.lock().await;
    let project_root = &cache_lock.project_root;
    
    // 收集项目信息
    let vue_files_count = utils::count_vue_files(project_root).unwrap_or(0);
    let (cached_count, _) = cache_lock.stats().await;
    
    let info = json!({
        "project_root": project_root.to_string_lossy(),
        "is_vue_project": utils::is_vue_project(project_root),
        "vue_files_count": vue_files_count,
        "cached_modules": cached_count,
        "hmr_enabled": enable_hmr,
        "entry_file": utils::find_entry_file(project_root)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    });
    
    Json(info)
}

/// HMR WebSocket 处理器
/// 
/// 路由: GET /@hmr
pub async fn hmr_handler(
    State(state): State<ServerState>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    let (_cache, enable_hmr, ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    
    if !enable_hmr {
        return axum::response::Json(json!({
            "error": "HMR is disabled"
        })).into_response();
    }
    
    // 升级到 WebSocket
    ws.on_upgrade(move |socket| handle_websocket(socket, ws_manager))
}

/// 处理 WebSocket 连接
async fn handle_websocket(socket: WebSocket, ws_manager: Arc<WebSocketManager>) {
    use axum::extract::ws::Message;
    use futures_util::stream::StreamExt;
    
    info!("HMR WebSocket client connected");
    
    // 分割发送和接收
    let (mut sender, mut receiver) = socket.split();
    
    // 订阅 HMR 事件
    let mut event_rx = ws_manager.subscribe();
    
    // 发送欢迎消息
    let welcome = json!({
        "type": "connected",
        "message": "HMR WebSocket connected"
    });
    
    if sender.send(Message::Text(welcome.to_string().into())).await.is_err() {
        warn!("Failed to send welcome message");
        return;
    }
    
    // 监听 HMR 事件并推送
    let sender_task = tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        warn!("Failed to send HMR event");
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("HMR client lagged, dropped {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("HMR event channel closed");
                    break;
                }
            }
        }
    });
    
    // 接收客户端消息（保持连接活跃）
    let receiver_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received from client: {}", text);
                }
                Ok(Message::Close(_)) => {
                    info!("HMR WebSocket client disconnected");
                    break;
                }
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });
    
    // 等待任一任务结束
    tokio::select! {
        _ = sender_task => info!("Sender task ended"),
        _ = receiver_task => info!("Receiver task ended"),
    }
}

/// 生成 index.html
fn generate_index_html(project_root: &std::path::PathBuf) -> String {
    // 尝试读取项目的 index.html
    let project_index = project_root.join("index.html");
    let html = if project_index.exists() {
        if let Ok(content) = std::fs::read_to_string(&project_index) {
            content
        } else {
            default_index_html()
        }
    } else {
        default_index_html()
    };
    
    // 检查是否已有 favicon
    let has_favicon = html.contains("rel=\"icon\"") || html.contains("rel='icon'") || html.contains("rel=icon");
    
    // 构建注入内容
    let mut inject_content = String::from(
        "<script>var __VUE_OPTIONS_API__=true;var __VUE_PROD_DEVTOOLS__=false;var __VUE_PROD_HYDRATION_MISMATCH_DETAILS__=false;</script>"
    );
    
    // 如果没有 favicon 链接，注入彩虹 emoji favicon
    if !has_favicon {
        inject_content.push_str(
            "<link rel=\"icon\" type=\"image/svg+xml\" href=\"/__iris-favicon.svg\">"
        );
    }
    
    // 在 </head> 前注入（所有浏览器兼容）
    if let Some(pos) = html.find("</head>") {
        let mut result = String::with_capacity(html.len() + inject_content.len());
        result.push_str(&html[..pos]);
        result.push_str(&inject_content);
        result.push_str(&html[pos..]);
        result
    } else {
        // 如果没有 </head>，追加到开头
        format!("{}\n{}", inject_content, html)
    }
}

/// 生成默认 index.html 模板
fn default_index_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate">
    <meta http-equiv="Pragma" content="no-cache">
    <meta http-equiv="Expires" content="0">
    <title>Iris JetCrab App</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        }
        #app {
            width: 100vw;
            height: 100vh;
        }
    </style>
</head>
<body>
    <div id="app"></div>
    
    <script type="module">
        // Iris JetCrab - Runtime On-Demand Compilation (v2)
        console.log('🦀 Iris JetCrab Runtime');
        console.log('📦 Compilation: On-demand');
        
        // 依赖问题检查 banner
        let depsCheckDone = false;
        
        async function checkDependencyIssues() {
            if (localStorage.getItem('iris_deps_dismissed') === 'true') {
                return false;
            }
            try {
                const res = await fetch('/api/dependency-issues');
                const data = await res.json();
                if (data.has_issues) {
                    return data;
                }
                return false;
            } catch (e) {
                console.warn('[Iris] Dependency check failed:', e);
                return false;
            }
        }
        
        // 显示依赖问题 banner
        function showDepsBanner(data) {
            const banner = document.createElement('div');
            banner.id = 'iris-deps-banner';
            banner.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                z-index: 99999;
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                color: #fff;
                padding: 10px 16px;
                display: flex;
                align-items: center;
                justify-content: space-between;
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
                font-size: 14px;
                box-shadow: 0 2px 12px rgba(0,0,0,0.2);
            `;
            
            const errors = data.issues.filter(i => i.severity === 'error').length;
            const warnings = data.issues.filter(i => i.severity === 'warning').length;
            
            banner.innerHTML = `
                <span>🛠️ 检测到 <strong>${data.issues.length}</strong> 个依赖问题（${errors} 个错误，${warnings} 个警告）</span>
                <div>
                    <a href="/resolve.html" style="color:#fff;background:rgba(255,255,255,0.2);padding:4px 14px;border-radius:4px;text-decoration:none;font-size:13px;margin-right:8px;">查看详情并修复</a>
                    <button onclick="dismissDepsBanner()" style="background:none;border:none;color:rgba(255,255,255,0.8);cursor:pointer;font-size:18px;">✕</button>
                </div>
            `;
            document.body.appendChild(banner);
            
            // 调整 app 顶部边距
            if (appContainer) {
                appContainer.style.marginTop = '44px';
            }
            
            depsCheckDone = true;
        }
        
        window.dismissDepsBanner = function() {
            const banner = document.getElementById('iris-deps-banner');
            if (banner) {
                banner.style.display = 'none';
            }
            if (appContainer) {
                appContainer.style.marginTop = '0';
            }
            localStorage.setItem('iris_deps_dismissed', 'true');
        };
        
        // HMR WebSocket 客户端
        var hmrConnected = false;
        var hmrWs = null;
        var moduleCacheTimestamp = Date.now();
        
        // 模块级 HMR 注册表：跟踪已加载的模块
        var hmrModuleRegistry = {};
        var hmrPendingModules = {};
        
        function connectHMR() {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/@hmr`;
            
            try {
                hmrWs = new WebSocket(wsUrl);
                
                hmrWs.onopen = () => {
                    console.log('[HMR] WebSocket connected');
                    hmrConnected = true;
                };
                
                hmrWs.onmessage = (event) => {
                    try {
                        const data = JSON.parse(event.data);
                        console.log('[HMR] Event:', data.type);
                        
                        if (data.type === 'file-changed') {
                            console.log(`[HMR] ⏺ File changed: ${data.file_name} (${data.path}), waiting for rebuild...`);
                        } else if (data.type === 'style-update') {
                            // 模块级 HMR: 样式热替换（无 JS 重执行）
                            console.log(`[HMR] 🎨 Style updated: ${data.path}`);
                            applyStyleUpdate(data.path, data.css);
                        } else if (data.type === 'module-update') {
                            // 模块级 HMR: Vue/TS 模块热替换
                            console.log(`[HMR] 📦 Module update: ${data.path} (${data.module_type})`);
                            hotReloadModule(data.path, data.module_type, data.timestamp);
                        } else if (data.type === 'rebuild-complete') {
                            console.log(`[HMR] ✅ Rebuild completed in ${data.duration_ms}ms (invalidated ${data.cleared_modules} modules)`);
                            // 更新缓存时间戳，使下次 import 使用新 URL
                            moduleCacheTimestamp = Date.now();
                            // 重新加载应用
                            reloadApp();
                        } else if (data.type === 'compile-error') {
                            console.error('[HMR] Compile error:', data.message);
                        }
                    } catch (e) {
                        console.warn('[HMR] Failed to parse message:', e);
                    }
                };
                
                hmrWs.onclose = () => {
                    console.log('[HMR] ⚠️ WebSocket disconnected, retrying in 3s...');
                    hmrConnected = false;
                    hmrWs = null;
                    setTimeout(connectHMR, 3000);
                };
                
                hmrWs.onerror = (err) => {
                    console.warn('[HMR] WebSocket error:', err);
                };
            } catch (e) {
                console.warn('[HMR] Connection failed:', e);
                setTimeout(connectHMR, 3000);
            }
        }
        
        // 当前 app 的挂载点
        let appContainer = document.getElementById('app');
        
        // 清理旧的 app 实例
        function cleanupApp() {
            // 移除所有之前注入的 style
            document.querySelectorAll('style[data-iris-hmr]').forEach(el => el.remove());
            
            // 清空 app 容器
            if (appContainer) {
                appContainer.innerHTML = '';
            }
            
            // 从 window 上清除 Vue app 实例
            delete window.__iris_app;
        }

        // 模块级 HMR: 样式热替换（直接替换 style 内容，无 JS 重执行）
        function applyStyleUpdate(path, cssContent) {
            // 查找已存在的 data-iris-hmr style（通过路径标识）
            const pathId = 'iris-style-' + path.replace(/[^a-zA-Z0-9]/g, '-');
            let styleEl = document.getElementById(pathId);
            
            if (!styleEl) {
                // 未找到现有 style，创建新元素
                styleEl = document.createElement('style');
                styleEl.id = pathId;
                styleEl.setAttribute('data-iris-hmr', '');
                document.head.appendChild(styleEl);
            }
            
            // 替换 CSS 内容
            styleEl.textContent = cssContent;
            console.log(`[HMR] ✅ Style applied: ${path}`);
        }
        
        // 模块级 HMR: 热替换单个模块（Vue/TS）
        async function hotReloadModule(path, moduleType, timestamp) {
            try {
                // 1. 标记该模块需要重载
                hmrPendingModules[path] = { timestamp, moduleType };
                
                // 2. 动态 import 已更新的模块（带缓存失效参数）
                const cacheBuster = `?t=${Date.now()}`;
                const moduleUrl = path.startsWith('/') ? path : `/src/${path}`;
                const fullUrl = `${moduleUrl}${cacheBuster}`;
                
                // 3. 对于 Vue 组件，标记 HMR 生效
                if (moduleType === 'vue') {
                    // Vue 3 组件热替换：如果 __VUE_HMR_RUNTIME__ 存在，用它进行精确更新
                    if (window.__VUE_HMR_RUNTIME__) {
                        try {
                            const module = await import(fullUrl);
                            console.log(`[HMR] 🔄 Vue component hot-reloaded: ${path}`);
                            // Vue HMR runtime 会处理组件替换
                            return;
                        } catch (err) {
                            console.warn(`[HMR] Vue HMR failed, falling back to re-fetch:`, err);
                        }
                    }
                    
                    // 4. 没有 Vue HMR runtime，通过 fetch 获取新编译的模块
                    const response = await fetch(fullUrl);
                    if (!response.ok) {
                        throw new Error(`HTTP ${response.status}`);
                    }
                    
                    const jsCode = await response.text();
                    console.log(`[HMR] 🔄 Module re-fetched (${jsCode.length} bytes): ${path}`);
                    
                    // 5. 对于 Vue 组件，只能重新渲染
                    // 找到 Vue 应用实例并重新挂载
                    const app = window.__iris_app;
                    if (app && app.unmount && app.mount) {
                        console.log('[HMR] Re-rendering Vue app...');
                        // 简单重挂载
                        const container = document.getElementById('app');
                        container.innerHTML = '';
                        // 卸载旧的 style
                        document.querySelectorAll('style[data-iris-hmr]').forEach(el => el.remove());
                        // 执行新的模块代码
                        const scriptEl = document.createElement('script');
                        scriptEl.type = 'module';
                        scriptEl.textContent = jsCode;
                        document.body.appendChild(scriptEl);
                    } else {
                        // 没有 Vue app 实例，直接执行
                        const scriptEl = document.createElement('script');
                        scriptEl.type = 'module';
                        scriptEl.textContent = jsCode;
                        document.body.appendChild(scriptEl);
                    }
                } else {
                    // 非 Vue 模块 (TS/JS)：直接加载
                    try {
                        await import(fullUrl);
                        console.log(`[HMR] 🔄 Script module hot-reloaded: ${path}`);
                    } catch (err) {
                        console.warn(`[HMR] Dynamic import failed for ${path}:`, err);
                    }
                }
                
                // 清除待处理标记
                delete hmrPendingModules[path];
            } catch (error) {
                console.error(`[HMR] ❌ Failed to hot-reload module ${path}:`, error);
                delete hmrPendingModules[path];
            }
        }
        
        // 重新加载应用（页面级 HMR）
        // 清理旧实例 → 清除缓存时间戳 → 重新 fetch 入口 → 创建新 script 执行
        async function reloadApp() {
            try {
                cleanupApp();
                
                // 使用动态 import 加载带缓存失效参数的模块
                const cacheBuster = `?t=${moduleCacheTimestamp}`;
                const moduleUrl = `/src/main.ts${cacheBuster}`;
                
                // 通过 fetch 获取新编译的代码
                const response = await fetch(moduleUrl);
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }
                
                let jsCode = await response.text();
                
                console.log('[HMR] Module re-fetched, executing...');
                console.log('✅ Entry module loaded');
                console.log('📝 Script length:', jsCode.length);
                
                // 移除旧 script 标签（如果存在）
                const oldScript = document.getElementById('iris-app-script');
                if (oldScript) {
                    oldScript.remove();
                }
                
                // 创建新的 script 标签执行
                const scriptEl = document.createElement('script');
                scriptEl.id = 'iris-app-script';
                scriptEl.type = 'module';
                scriptEl.textContent = jsCode;
                document.body.appendChild(scriptEl);
                
                console.log('[HMR] App reloaded successfully');
            } catch (error) {
                console.error('[HMR] Failed to reload app:', error);
            }
        }
        
        // 首次加载应用
        async function loadApp() {
            try {
                // 连接 HMR WebSocket
                connectHMR();
                
                // 检查依赖问题
                checkDependencyIssues().then(data => {
                    if (data) {
                        showDepsBanner(data);
                    }
                });
                
                // 请求编译入口文件
                const response = await fetch('/src/main.ts');
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }
                
                const jsCode = await response.text();
                
                console.log('✅ Entry module loaded');
                console.log('📝 Script length:', jsCode.length);
                
                // 创建 script 元素执行代码
                const scriptEl = document.createElement('script');
                scriptEl.type = 'module';
                scriptEl.textContent = jsCode;
                document.body.appendChild(scriptEl);
                
            } catch (error) {
                console.error('❌ Failed to load app:', error);
            }
        }
        
        loadApp();
    </script>
</body>
</html>"#.to_string()
}

/// 依赖问题扫描 API 处理器
/// 
/// 路由: GET /api/dependency-issues
pub async fn dependency_issues_handler(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let (cache, _enable_hmr, _ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    let cache_lock = cache.lock().await;
    let project_root = &cache_lock.project_root;

    let scanner = iris_jetcrab_engine::DependencyScanner::new(project_root.clone());
    let scan_result = scanner.scan();

    let issues_json: Vec<serde_json::Value> = scan_result.issues.iter().map(|issue| {
        json!({
            "issue_type": issue.issue_type,
            "import_path": issue.import_path,
            "source_file": issue.source_file,
            "source_line": issue.source_line,
            "description": issue.description,
            "solution": issue.solution,
            "severity": issue.severity,
            "can_auto_fix": issue.can_auto_fix,
        })
    }).collect();

    Json(json!({
        "issues": issues_json,
        "declared_packages": scan_result.declared_packages,
        "installed_packages": scan_result.installed_packages,
        "has_node_modules": scan_result.has_node_modules,
        "source_file_count": scan_result.source_file_count,
        "has_issues": !scan_result.issues.is_empty(),
        "fixable_count": scan_result.issues.iter().filter(|i| i.can_auto_fix).count(),
        "iris_resolved": scan_result.iris_resolved,
    }))
}

/// 依赖问题解决页面处理器
/// 
/// 路由: GET /resolve.html
pub async fn resolve_page_handler() -> Html<String> {
    Html(generate_resolve_page())
}

/// 自动解决依赖问题处理器
/// 
/// 路由: POST /api/resolve-dependencies
pub async fn resolve_dependencies_handler(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let (cache, _enable_hmr, ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    let project_root = {
        let cache_lock = cache.lock().await;
        cache_lock.project_root.clone()
    };

    // 创建扫描器并找到需要下载的包
    let scanner = iris_jetcrab_engine::DependencyScanner::new(project_root.clone());
    let uninstalled_packages = scanner.find_uninstalled_npm_packages();
    
    if uninstalled_packages.is_empty() {
        info!("No uninstalled npm packages found");
        return Json(json!({
            "status": "skipped",
            "message": "没有需要下载的 npm 包",
            "downloaded": [],
        }));
    }

    info!("Found {} uninstalled npm packages", uninstalled_packages.len());

    // 广播开始事件
    ws_manager.broadcast(HmrEvent::NpmDownload {
        package: uninstalled_packages[0].clone(),
        version: String::new(),
        progress: 0,
        status: "starting".to_string(),
        error: None,
    });

    // 在后台任务中执行下载
    let ws_manager_clone = ws_manager.clone();
    let ws_manager_for_download = ws_manager.clone();
    let packages_clone = uninstalled_packages.clone();
    let project_root_clone = project_root.clone();

    tokio::spawn(async move {
        let node_modules = project_root_clone.join("node_modules");
        let downloader = iris_jetcrab_engine::NpmDownloader::new(node_modules.clone())
            .with_progress_callback(move |pkg, ver, progress, status| {
                ws_manager_for_download.broadcast(HmrEvent::NpmDownload {
                    package: pkg.to_string(),
                    version: ver.to_string(),
                    progress,
                    status: status.to_string(),
                    error: None,
                });
            });

        let mut downloaded_versions = HashMap::new();

        for pkg in &packages_clone {
            info!("Downloading npm package: {}", pkg);
            match downloader.download_and_install(pkg, None) {
                Ok(path) => {
                    info!("Successfully downloaded {} to {:?}", pkg, path);
                    // 读取实际安装的版本
                    let pkg_json_path = path.join("package.json");
                    if pkg_json_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&pkg_json_path) {
                            if let Ok(pkg_json) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(ver) = pkg_json.get("version").and_then(|v| v.as_str()) {
                                    downloaded_versions.insert(pkg.clone(), ver.to_string());
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to download {}: {}", pkg, e);
                    ws_manager_clone.broadcast(HmrEvent::NpmDownload {
                        package: pkg.clone(),
                        version: String::new(),
                        progress: 0,
                        status: "error".to_string(),
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        // 写入 irisResolved 到 package.json
        if !downloaded_versions.is_empty() {
            info!("Writing {} resolved versions to package.json irisResolved", downloaded_versions.len());
            ws_manager_clone.broadcast(HmrEvent::NpmDownload {
                package: "irisResolved".to_string(),
                version: String::new(),
                progress: 0,
                status: "writing_iris_resolved".to_string(),
                error: None,
            });

            let scanner = iris_jetcrab_engine::DependencyScanner::new(project_root_clone.clone());
            match scanner.write_iris_resolved(downloaded_versions) {
                Ok(()) => {
                    info!("Successfully updated package.json irisResolved");
                }
                Err(e) => {
                    warn!("Failed to write irisResolved: {}", e);
                    ws_manager_clone.broadcast(HmrEvent::NpmDownload {
                        package: "irisResolved".to_string(),
                        version: String::new(),
                        progress: 0,
                        status: "error".to_string(),
                        error: Some(format!("写入 irisResolved 失败: {}", e)),
                    });
                }
            }
        }

        // 下载完成，广播完成事件
        info!("All downloads completed");
        ws_manager_clone.broadcast(HmrEvent::RebuildComplete {
            cleared_modules: packages_clone.len(),
            duration_ms: 0,
        });
    });

    Json(json!({
        "status": "started",
        "message": format!("开始下载 {} 个 npm 包，请查看 WebSocket 进度", uninstalled_packages.len()),
        "downloading": uninstalled_packages,
    }))
}

/// 生成依赖问题解决页面 HTML
fn generate_resolve_page() -> String {
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Iris Runtime - 依赖问题解决</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            color: #333;
        }
        .container {
            max-width: 900px;
            margin: 0 auto;
            padding: 40px 20px;
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        .header h1 {
            font-size: 28px;
            color: #fff;
            margin-bottom: 8px;
        }
        .header p {
            color: rgba(255,255,255,0.85);
            font-size: 16px;
        }
        .card {
            background: #fff;
            border-radius: 12px;
            box-shadow: 0 4px 20px rgba(0,0,0,0.15);
            padding: 24px;
            margin-bottom: 20px;
        }
        .card h2 {
            font-size: 18px;
            margin-bottom: 16px;
            color: #444;
        }
        .card h2 .badge {
            display: inline-block;
            padding: 2px 10px;
            border-radius: 10px;
            font-size: 12px;
            margin-left: 8px;
        }
        .badge-error { background: #fee2e2; color: #dc2626; }
        .badge-warning { background: #fef3c7; color: #d97706; }
        .badge-success { background: #dcfce7; color: #16a34a; }
        .badge-info { background: #e0f2fe; color: #0284c7; }
        
        .issue-item {
            border: 1px solid #e5e7eb;
            border-radius: 8px;
            padding: 16px;
            margin-bottom: 12px;
            transition: border-color 0.2s;
        }
        .issue-item:last-child { margin-bottom: 0; }
        .issue-header {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            margin-bottom: 8px;
        }
        .issue-title {
            font-weight: 600;
            font-size: 15px;
            color: #1f2937;
        }
        .issue-source {
            font-size: 13px;
            color: #6b7280;
            margin-top: 4px;
        }
        .issue-source code {
            background: #f3f4f6;
            padding: 1px 6px;
            border-radius: 4px;
            font-size: 12px;
        }
        .issue-body {
            margin-top: 8px;
        }
        .issue-desc {
            font-size: 14px;
            color: #4b5563;
            margin-bottom: 8px;
        }
        .issue-solution {
            font-size: 13px;
            color: #059669;
            background: #f0fdf4;
            padding: 8px 12px;
            border-radius: 6px;
        }
        
        .btn {
            display: inline-flex;
            align-items: center;
            padding: 10px 24px;
            border-radius: 8px;
            font-size: 15px;
            font-weight: 600;
            cursor: pointer;
            border: none;
            transition: all 0.2s;
            text-decoration: none;
        }
        .btn:disabled {
            opacity: 0.6;
            cursor: not-allowed;
        }
        .btn-primary {
            background: #6366f1;
            color: #fff;
        }
        .btn-primary:hover:not(:disabled) {
            background: #4f46e5;
            transform: translateY(-1px);
            box-shadow: 0 4px 12px rgba(99,102,241,0.4);
        }
        .btn-secondary {
            background: #e5e7eb;
            color: #374151;
        }
        .btn-secondary:hover {
            background: #d1d5db;
        }
        .btn-success {
            background: #16a34a;
            color: #fff;
        }
        .btn-success:hover:not(:disabled) {
            background: #15803d;
        }
        .actions {
            display: flex;
            gap: 12px;
            justify-content: center;
            margin-top: 24px;
        }
        
        /* 进度条 */
        .progress-container {
            display: none;
            margin-top: 16px;
        }
        .progress-container.active { display: block; }
        .progress-bar-bg {
            background: #e5e7eb;
            border-radius: 8px;
            height: 24px;
            overflow: hidden;
            position: relative;
        }
        .progress-bar-fill {
            height: 100%;
            background: linear-gradient(90deg, #6366f1, #8b5cf6);
            border-radius: 8px;
            transition: width 0.3s ease;
            width: 0%;
        }
        .progress-text {
            text-align: center;
            margin-top: 8px;
            font-size: 14px;
            color: #6b7280;
        }
        .progress-status {
            text-align: center;
            margin-top: 4px;
            font-size: 13px;
            color: #9ca3af;
        }
        
        /* 日志 */
        .log {
            display: none;
            background: #1f2937;
            border-radius: 8px;
            padding: 16px;
            font-family: 'Fira Code', monospace;
            font-size: 13px;
            color: #d1d5db;
            max-height: 200px;
            overflow-y: auto;
            margin-top: 12px;
            line-height: 1.6;
        }
        .log.active { display: block; }
        .log .log-info { color: #60a5fa; }
        .log .log-success { color: #34d399; }
        .log .log-error { color: #f87171; }
        .log .log-warn { color: #fbbf24; }
        
        .empty-state {
            text-align: center;
            padding: 40px;
            color: #6b7280;
        }
        .empty-state .icon { font-size: 48px; margin-bottom: 12px; }
        .empty-state p { font-size: 16px; }

        .loading-spinner {
            display: inline-block;
            width: 16px;
            height: 16px;
            border: 2px solid rgba(99,102,241,0.3);
            border-radius: 50%;
            border-top-color: #6366f1;
            animation: spin 0.8s linear infinite;
            margin-right: 8px;
            vertical-align: middle;
        }
        @keyframes spin { to { transform: rotate(360deg); } }

        .severity-error { border-left: 4px solid #ef4444; }
        .severity-warning { border-left: 4px solid #f59e0b; }
        .severity-info { border-left: 4px solid #3b82f6; }

        .skip-link {
            display: inline-block;
            margin-top: 16px;
            color: rgba(255,255,255,0.7);
            font-size: 14px;
            cursor: pointer;
            text-decoration: underline;
        }
        .skip-link:hover { color: #fff; }
        
        .summary-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 12px;
            margin-bottom: 16px;
        }
        .summary-item {
            text-align: center;
            padding: 12px;
            background: #f9fafb;
            border-radius: 8px;
        }
        .summary-item .num {
            font-size: 24px;
            font-weight: 700;
            color: #1f2937;
        }
        .summary-item .label {
            font-size: 13px;
            color: #6b7280;
            margin-top: 4px;
        }
        
        .resolved-list {
            display: flex;
            flex-wrap: wrap;
            gap: 6px;
        }
        .resolved-tag {
            display: inline-block;
            padding: 3px 10px;
            background: #dcfce7;
            color: #16a34a;
            border-radius: 12px;
            font-size: 12px;
            font-weight: 500;
        }
        .iris-resolved-section {
            margin-bottom: 16px;
            padding: 12px;
            background: #f0fdf4;
            border: 1px solid #bbf7d0;
            border-radius: 8px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🛠️ Iris Runtime 依赖管理</h1>
            <p>扫描项目中的依赖问题，并提供一键修复方案</p>
        </div>

        <div class="card" id="scanning-card">
            <div class="empty-state" id="loading-state">
                <div class="loading-spinner"></div>
                <p>正在扫描项目依赖...</p>
            </div>
            <div class="empty-state" id="empty-state" style="display:none;">
                <div class="icon">✅</div>
                <p>项目依赖检查通过，没有发现需要处理的问题</p>
                <button class="btn btn-primary" style="margin-top:16px;" onclick="window.location.href='/'">返回应用</button>
            </div>
            <div id="issues-container" style="display:none;">
                <h2>
                    发现的问题
                    <span class="badge badge-error" id="issue-count">0</span>
                </h2>
                
                <div class="summary-grid" id="summary-grid"></div>
                
                <div id="iris-resolved-section" class="iris-resolved-section" style="display:none;">
                    <h3 style="font-size:14px;margin-bottom:8px;color:#059669;">✅ 已由 iris 解析的软件包 (记录在 package.json 的 irisResolved 字段)</h3>
                    <div id="iris-resolved-list" class="resolved-list"></div>
                </div>
                
                <div id="issues-list"></div>
                
                <div class="actions">
                    <button class="btn btn-primary" id="fix-btn" onclick="startFix()">
                        🚀 一键修复依赖问题
                    </button>
                    <button class="btn btn-secondary" onclick="skipResolve()">
                        跳过，继续浏览
                    </button>
                </div>
            </div>
        </div>

        <div class="card" id="progress-card" style="display:none;">
            <h2>📦 正在处理依赖</h2>
            <div class="progress-container active">
                <div class="progress-bar-bg">
                    <div class="progress-bar-fill" id="progress-fill"></div>
                </div>
                <div class="progress-text" id="progress-text">准备中...</div>
                <div class="progress-status" id="progress-status"></div>
            </div>
            <div class="log active" id="log"></div>
            <div class="actions" id="post-fix-actions" style="display:none;">
                <button class="btn btn-success" onclick="reloadApp()">
                    ✅ 重新加载应用
                </button>
            </div>
        </div>

        <div style="text-align:center;">
            <a class="skip-link" href="/" onclick="dismissIssues()">暂不处理，直接进入应用 →</a>
        </div>
    </div>

    <script>
        let ws = null;
        let fixComplete = false;

        // 连接 HMR WebSocket
        function connectWS() {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            ws = new WebSocket(`${protocol}//${window.location.host}/@hmr`);
            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    if (data.type === 'npm-download') {
                        updateProgress(data);
                    } else if (data.type === 'rebuild-complete') {
                        fixComplete = true;
                        document.getElementById('progress-text').textContent = '✅ 所有依赖已处理完成！';
                        document.getElementById('progress-status').textContent = '';
                        document.getElementById('post-fix-actions').style.display = 'flex';
                        addLog('success', '所有依赖处理完成，可以重新加载应用了');
                    }
                } catch(e) {}
            };
        }

        // 更新进度
        function updateProgress(data) {
            const fill = document.getElementById('progress-fill');
            const text = document.getElementById('progress-text');
            const status = document.getElementById('progress-status');
            
            fill.style.width = data.progress + '%';
            
            if (data.status === 'resolving') {
                text.textContent = `🔍 正在解析 ${data.package} 的版本信息...`;
            } else if (data.status === 'downloading') {
                text.textContent = `📥 正在下载 ${data.package}@${data.version} (${data.progress}%)`;
            } else if (data.status === 'extracting') {
                text.textContent = `📦 正在解压 ${data.package}...`;
            } else if (data.status === 'installed') {
                text.textContent = `✅ ${data.package}@${data.version} 安装完成`;
                addLog('success', `${data.package}@${data.version} 安装完成`);
            } else if (data.status === 'writing_iris_resolved') {
                text.textContent = `📝 正在将已解析版本写入 package.json 的 irisResolved 字段...`;
                addLog('info', '正在更新 package.json...');
            } else if (data.status === 'error') {
                text.textContent = `❌ ${data.package} 下载失败`;
                status.textContent = data.error || '未知错误';
                addLog('error', `${data.package} 下载失败: ${data.error || '未知错误'}`);
            } else if (data.status === 'starting') {
                addLog('info', `开始处理依赖...`);
            }
            
            if (data.status !== 'error') {
                status.textContent = `${data.package}@${data.version} - ${data.status}`;
            }
        }

        // 添加日志
        function addLog(level, message) {
            const log = document.getElementById('log');
            const entry = document.createElement('div');
            entry.className = 'log-' + level;
            entry.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
            log.appendChild(entry);
            log.scrollTop = log.scrollHeight;
        }

        // 开始修复
        function startFix() {
            document.getElementById('fix-btn').disabled = true;
            document.getElementById('fix-btn').textContent = '⏳ 处理中...';
            document.getElementById('progress-card').style.display = 'block';
            document.getElementById('progress-card').scrollIntoView({ behavior: 'smooth' });
            
            connectWS();
            
            fetch('/api/resolve-dependencies', { method: 'POST' })
                .then(r => r.json())
                .then(data => {
                    addLog('info', data.message);
                })
                .catch(err => {
                    addLog('error', '请求失败: ' + err);
                });
        }

        // 跳过
        function skipResolve() {
            dismissIssues();
            window.location.href = '/';
        }

        // 忽略检查
        function dismissIssues() {
            localStorage.setItem('iris_deps_dismissed', 'true');
        }

        // 重新加载应用
        function reloadApp() {
            window.location.href = '/';
        }

        // 加载扫描结果
        async function loadResults() {
            try {
                const res = await fetch('/api/dependency-issues');
                const data = await res.json();
                
                document.getElementById('loading-state').style.display = 'none';
                
                if (!data.has_issues) {
                    document.getElementById('empty-state').style.display = 'block';
                    return;
                }
                
                document.getElementById('issues-container').style.display = 'block';
                
                // 统计摘要
                let errors = 0, warnings = 0, fixable = 0;
                data.issues.forEach(i => {
                    if (i.severity === 'error') errors++;
                    else if (i.severity === 'warning') warnings++;
                    if (i.can_auto_fix) fixable++;
                });
                
                document.getElementById('issue-count').textContent = data.issues.length;
                
                document.getElementById('summary-grid').innerHTML = `
                    <div class="summary-item">
                        <div class="num">${data.issues.length}</div>
                        <div class="label">问题总数</div>
                    </div>
                    <div class="summary-item">
                        <div class="num" style="color:#dc2626;">${errors}</div>
                        <div class="label">错误</div>
                    </div>
                    <div class="summary-item">
                        <div class="num" style="color:#d97706;">${warnings}</div>
                        <div class="label">警告</div>
                    </div>
                    <div class="summary-item">
                        <div class="num" style="color:#16a34a;">${fixable}</div>
                        <div class="label">可自动修复</div>
                    </div>
                `;
                
                // 显示 irisResolved 中已有的解析记录
                const irisResolved = data.iris_resolved || {};
                const resolvedKeys = Object.keys(irisResolved);
                if (resolvedKeys.length > 0) {
                    document.getElementById('iris-resolved-section').style.display = 'block';
                    document.getElementById('iris-resolved-list').innerHTML = resolvedKeys.map(key =>
                        `<span class="resolved-tag">${key}@${irisResolved[key]}</span>`
                    ).join('');
                }
                
                // 渲染问题列表
                const list = document.getElementById('issues-list');
                list.innerHTML = data.issues.map(i => `
                    <div class="issue-item severity-${i.severity}">
                        <div class="issue-header">
                            <div>
                                <div class="issue-title">
                                    ${i.severity === 'error' ? '🔴' : i.severity === 'warning' ? '🟡' : '🔵'}
                                    ${i.import_path}
                                    <span class="badge badge-${i.severity}">${i.severity}</span>
                                    ${i.can_auto_fix ? '<span class="badge badge-success">可自动修复</span>' : ''}
                                </div>
                                <div class="issue-source">
                                    文件: <code>${i.source_file}</code>
                                    ${i.source_line ? `行号: <code>${i.source_line}</code>` : ''}
                                </div>
                            </div>
                        </div>
                        <div class="issue-body">
                            <div class="issue-desc">${i.description}</div>
                            <div class="issue-solution">💡 ${i.solution}</div>
                        </div>
                    </div>
                `).join('');
                
                // 如果没有可自动修复的问题，禁用按钮
                if (fixable === 0) {
                    document.getElementById('fix-btn').disabled = true;
                    document.getElementById('fix-btn').textContent = '无需自动修复';
                }
                
            } catch (err) {
                document.getElementById('loading-state').innerHTML = `
                    <div class="icon">❌</div>
                    <p>扫描失败: ${err.message}</p>
                    <button class="btn btn-primary" style="margin-top:12px;" onclick="location.reload()">重试</button>
                `;
            }
        }

        loadResults();
    </script>
</body>
</html>"#.to_string()
}

/// 获取文件的 Content-Type
fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        "application/javascript"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".html") {
        "text/html"
    } else {
        "application/octet-stream"
    }
}

/// npm 包处理器 - 从 node_modules 提供 npm 包
/// 
/// 路由: GET /@npm/*path
/// 示例: GET /@npm/vue → 返回 vue 包的入口文件
pub async fn npm_package_handler(
    State(state): State<ServerState>,
    Path(path): Path<String>,
) -> Response {
    let (cache, _enable_hmr, _ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    let cache_lock = cache.lock().await;
    let project_root = &cache_lock.project_root;
    
    // 解析包名和子路径
    // 注意：scoped package（如 @vue/devtools-api）的 URL 路径格式为 @vue/devtools-api
    // splitn(2, '/') 会把 scoped package 名分成 ["@vue", "devtools-api"]
    // 需要额外处理来重建完整包名
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    let (full_package_name, sub_path) = if parts[0].starts_with('@') && parts.len() > 1 {
        // Scoped package：第一个 / 后的第一部分是包名后半部分
        // 检查是否有更深层的子路径
        let remaining = parts[1];
        let inner_parts: Vec<&str> = remaining.splitn(2, '/').collect();
        let scoped_full = format!("@{}/{}", &parts[0][1..], inner_parts[0]);
        (scoped_full, if inner_parts.len() > 1 { inner_parts[1] } else { "" })
    } else {
        (parts[0].to_string(), if parts.len() > 1 { parts[1] } else { "" })
    };
    
    let package_name = &full_package_name;
    
    // 构建 node_modules 中的路径
    let node_modules_path = project_root.join("node_modules");
    let package_path = if sub_path.is_empty() {
        node_modules_path.join(package_name)
    } else {
        node_modules_path.join(package_name).join(sub_path)
    };
    
    // 如果是目录，尝试读取 package.json 的 main 或 module 字段
    if package_path.is_dir() {
        let package_json_path = package_path.join("package.json");
        if package_json_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                if let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&content) {
                    // 优先使用 module 字段（ESM），然后使用 main 字段
                    let entry_file = package_json.get("module")
                        .or_else(|| package_json.get("main"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("index.js");
                    
                    let entry_path = package_path.join(entry_file);
                    if entry_path.exists() {
                        if let Ok(js_content) = std::fs::read_to_string(&entry_path) {
                            // 替换 process.env.NODE_ENV 为 'development'
                            let js_with_env = js_content.replace(
                                "process.env.NODE_ENV",
                                "'development'"
                            );
                            
                            // 重写 npm 包内部的裸模块导入
                            let rewritten_js = rewrite_bare_imports(&js_with_env, "");
                            
                            // 如果入口文件在子目录中，重写相对导入为完整 /@npm/ 路径
                            let entry_path_obj = std::path::Path::new(entry_file);
                            let entry_dir = entry_path_obj.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                            let final_js = if !entry_dir.is_empty() {
                                rewrite_npm_relative_imports(&rewritten_js, package_name, &entry_dir)
                            } else {
                                rewritten_js
                            };
                            
                            // 返回 JavaScript 模块
                            return (axum::http::StatusCode::OK, [
                                (header::CONTENT_TYPE, "application/javascript"),
                            ], final_js).into_response();
                        }
                    }
                }
            }
        }
    }
    
    // 如果是文件，直接返回
    if package_path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&package_path) {
            let content_type = get_content_type(&package_path.to_string_lossy());
            
            // 如果是 JavaScript 文件，处理环境变量和重写导入
            let final_content = if content_type == "application/javascript" {
                // 替换 process.env.NODE_ENV 为 'development'
                let js_with_env = content.replace(
                    "process.env.NODE_ENV",
                    "'development'"
                );
                rewrite_bare_imports(&js_with_env, "")
            } else {
                content
            };
            
            return (axum::http::StatusCode::OK, [
                (header::CONTENT_TYPE, content_type),
            ], final_content).into_response();
        }
    }
    
    // 包未找到
    let error_response = json!({
        "error": format!("npm package not found: {}", path)
    });
    (axum::http::StatusCode::NOT_FOUND, Json(error_response)).into_response()
}

/// Base64 编码（简化版）
fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    let mut result = String::new();
    for byte in data {
        write!(result, "{:02x}", byte).unwrap();
    }
    result
}

/// 重写 npm 包入口文件的相对导入路径
/// 
/// 当入口文件在子目录中时（如 lib/esm/index.js），
/// 将内部的 ./env.js 重写为 /@npm/pkg/lib/esm/env.js，
/// 以便浏览器能正确请求到子目录中的模块。
/// 
/// 例如：
/// - `from './env.js'` → `from '/@npm/@vue/devtools-api/lib/esm/env.js'`
/// - `import('./proxy.js')` → `import('/@npm/@vue/devtools-api/lib/esm/proxy.js')`
/// - `from '../other/file.js'` → `from '/@npm/pkg/other/file.js'`
fn rewrite_npm_relative_imports(script: &str, package_name: &str, subdir: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;
    
    // 将子目录中的 / 转为系统路径分隔符以规范化 ../ 解析
    // 但保持 URL 格式使用 /
    let subdir_normalized = subdir.replace(std::path::MAIN_SEPARATOR_STR, "/");
    
    // 匹配 from './xxx'、from '../xxx'、import('./xxx')、import('../xxx')
    // 捕获前缀（from ' 或 import('）、相对路径（./ 或 ../）、路径内容
    static RELATIVE_IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?:from\s+['"]|import\(['"])(\.\.?/)([^'"]+)['"]"#).unwrap()
    });
    
    RELATIVE_IMPORT_RE.replace_all(script, |caps: &regex::Captures| {
        let full_match = &caps[0];
        let relative_prefix = &caps[1]; // "./" or "../"
        let import_path = &caps[2];     // the rest of the path after ./ or ../
        
        let resolved_path = if relative_prefix == "./" {
            // ./xxx → 拼接到子目录之后
            format!("/@npm/{}/{}/{}", package_name, subdir_normalized, import_path)
        } else {
            // ../xxx → 从子目录回退一级再拼接
            // 例如 subdir="lib/esm", ../xxx → lib/xxx → /@npm/pkg/lib/xxx
            let parent_dir = std::path::Path::new(&subdir_normalized)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            if parent_dir.is_empty() {
                // 子目录只有一层，../ 回到根目录
                format!("/@npm/{}/{}", package_name, import_path)
            } else {
                format!("/@npm/{}/{}/{}", package_name, parent_dir, import_path)
            }
        };
        
        // 根据匹配的前缀决定使用 from '...' 还是 import('...') 包装
        if full_match.starts_with("import") {
            format!("import('{}')", resolved_path)
        } else {
            format!("from '{}'", resolved_path)
        }
    }).to_string()
}

/// 重写裸模块导入（bare module imports）为 /@npm/ 路径
/// 
/// 例如：
/// - `import { ref } from 'vue'` → `import { ref } from '/@npm/vue'`
/// - `import { defineStore } from 'pinia'` → `import { defineStore } from '/@npm/pinia'`
/// - `import('../views/X.vue')` → `import('/src/views/X.vue')`
/// 
/// module_path: 源文件的请求路径（如 "router", "main.ts"），用于正确解析 ../ 相对路径
fn rewrite_bare_imports(script: &str, module_path: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;
    
    // 匹配 import ... from 'package' 或 import ... from "package"
    static BARE_IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"from\s+['"]([^'"]+)['"]"#).unwrap()
    });
    
    // 匹配动态导入 import('path') 或 import("path")
    static DYNAMIC_IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"import\(['"]([^'"]+)['"]\)"#).unwrap()
    });
    
    // 获取源文件的目录路径（相对于 src/），用于解析 ../ 路径
    // 对于目录索引路径（如 "router" 对应 router/index.ts），父目录就是 module_path 本身
    let source_dir = {
        let p = std::path::Path::new(module_path);
        let parent = p.parent().map(|p| p.to_string_lossy().to_string());
        match parent {
            Some(dir) if !dir.is_empty() && dir != "." => dir,
            _ => {
                // 如果 module_path 是裸名称（无扩展名、无路径分隔符），
                // 说明是目录索引请求（如 "router" → router/index.ts）
                if !module_path.is_empty() && !module_path.contains('.') && !module_path.contains('/') && !module_path.contains('\\') {
                    module_path.to_string()
                } else {
                    String::new()
                }
            }
        }
    };
    
    // 解析相对路径为 /src/ 下的绝对路径
    let is_npm_module = module_path.is_empty();
    let resolve_to_src = |import_path: &str| -> String {
        // npm 包模块：保留相对导入原样，让浏览器基于模块 URL 正确解析
        // 只重写裸模块名（如 from 'vue' → from '/@npm/vue'）
        if is_npm_module {
            if import_path.starts_with("./") || import_path.starts_with("../") || import_path.starts_with('/') {
                return import_path.to_string();
            } else {
                return format!("/@npm/{}", import_path);
            }
        }
        
        if import_path.starts_with("./") {
            // ./xxx → 去掉 ./，直接拼接到 /src/
            format!("/src/{}", &import_path[2..])
        } else if import_path.starts_with("../") {
            if source_dir.is_empty() {
                // 源文件在 src/ 根目录，../ 会超出 src/
                // 这种情况不应该发生，回退到直接使用文件名
                let filename = import_path.split('/').last().unwrap_or(import_path);
                format!("/src/{}", filename)
            } else {
                // 将相对路径与源文件目录拼接后归一化
                let combined = format!("{}/{}", source_dir, import_path);
                let p = std::path::Path::new(&combined);
                let mut parts = Vec::new();
                for component in p.components() {
                    match component {
                        std::path::Component::Normal(c) => {
                            parts.push(c.to_string_lossy().to_string());
                        }
                        std::path::Component::ParentDir => {
                            parts.pop();
                        }
                        _ => {}
                    }
                }
                format!("/src/{}", parts.join("/"))
            }
        } else if import_path.starts_with('/') {
            // 已经是绝对路径
            import_path.to_string()
        } else {
            // 裸模块名（npm 包）
            format!("/@npm/{}", import_path)
        }
    };
    
    // 1. 先处理静态导入: from '...'
    let result = BARE_IMPORT_RE.replace_all(script, |caps: &regex::Captures| {
        let import_path = &caps[1];
        let resolved = resolve_to_src(import_path);
        format!("from '{}'", resolved)
    });
    
    // 2. 再处理动态导入: import('...')
    DYNAMIC_IMPORT_RE.replace_all(&result, |caps: &regex::Captures| {
        let import_path = &caps[1];
        
        // 只重写相对路径（./ 和 ../），不处理裸模块
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let resolved = resolve_to_src(import_path);
            format!("import('{}')", resolved)
        } else {
            // 保持原样（包括 npm 包动态导入）
            caps[0].to_string()
        }
    }).to_string()
}

/// 源文件处理器 - 编译并返回可执行的 JavaScript 模块
/// 
/// 路由: GET /src/*path
/// 示例: GET /src/main.ts → 编译并返回 JavaScript 模块
pub async fn source_file_handler(
    State(state): State<ServerState>,
    Path(path): Path<String>,
) -> Response {
    let (cache, _enable_hmr, _ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    
    // 编译模块
    let compile_result = {
        let cache_lock = cache.lock().await;
        cache_lock.get_or_compile(&path).await
    };
    
    match compile_result {
        Ok(module) => {
            // 构建完整的 JavaScript 模块
            let mut js_code = String::new();
            
            // 添加 HMR 客户端代码（开发模式）
            js_code.push_str("/* Iris JetCrab HMR Client */\n");
            js_code.push_str("if (import.meta.hot) {\n");
            js_code.push_str("  import.meta.hot.accept((newModule) => {\n");
            js_code.push_str("    console.log('[HMR] Module updated:', import.meta.url);\n");
            js_code.push_str("  });\n");
            js_code.push_str("}\n\n");
            
            // 重写裸模块导入（bare imports）为 /@npm/ 路径
            let rewritten_script = rewrite_bare_imports(&module.script, &path);
            
            // 添加编译后的脚本
            js_code.push_str(&rewritten_script);
            
            // 注入样式
            for style in &module.styles {
                js_code.push_str("\n\n/* Injected Styles */\n");
                js_code.push_str("const style = document.createElement('style');\n");
                js_code.push_str("style.setAttribute('data-iris-hmr', '');\n");
                js_code.push_str("style.textContent = `");
                js_code.push_str(&style.code.replace('`', "\\`"));
                js_code.push_str("`;\n");
                js_code.push_str("document.head.appendChild(style);\n");
            }
            
            // 检测是否为入口文件，注入 __iris_app 暴露代码
            let mut is_entry = path == "main.ts" || path == "main.js" || path == "main.tsx" || path == "main.jsx";
            // 也检查 src/ 前缀的入口文件
            if !is_entry {
                let trimmed = path.trim_start_matches("src/");
                is_entry = trimmed == "main.ts" || trimmed == "main.js" || trimmed == "main.tsx" || trimmed == "main.jsx";
            }
            if is_entry {
                js_code.push_str("\n\n/* Iris Runtime: Expose Vue app instance for Inspector */\n");
                js_code.push_str("(function() {\n");
                js_code.push_str("  var irisCheck = setInterval(function() {\n");
                js_code.push_str("    var appEl = document.getElementById('app');\n");
                js_code.push_str("    if (appEl && appEl.__vue_app__) {\n");
                js_code.push_str("      window.__iris_app = appEl.__vue_app__;\n");
                js_code.push_str("      clearInterval(irisCheck);\n");
                js_code.push_str("    }\n");
                js_code.push_str("  }, 50);\n");
                js_code.push_str("  setTimeout(function() { clearInterval(irisCheck); }, 5000);\n");
                js_code.push_str("})();\n");
            }
            
            // 返回 JavaScript 模块
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/javascript"),
                    (header::CACHE_CONTROL, "no-cache"),
                ],
                js_code,
            ).into_response()
        }
        Err(e) => {
            warn!("Failed to compile module {}: {}", path, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                format!("Failed to compile module: {}", e),
           ).into_response()
        }
    }
}

// 彩虹 emoji SVG favicon（内嵌 Iris 项目信息）
fn rainbow_favicon_svg() -> &'static str {
    r##"<!-- Iris JetCrab v0.1.1 - Rainbow Favicon -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" fill="#f8f4ff" rx="14"/><text x="50" y="82" text-anchor="middle" font-size="72">&#127752;</text><text x="50" y="96" text-anchor="middle" font-size="6" font-family="sans-serif" fill="#aaa">Iris</text></svg>"##
}

/// 检查路径是否为图片文件
fn is_image_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
        || lower.ends_with(".gif") || lower.ends_with(".webp") || lower.ends_with(".bmp")
        || lower.ends_with(".ico") || lower.ends_with(".svg")
}

/// 生成占位 SVG 图片（含 Iris 项目标识）
fn placeholder_svg(path: &str) -> String {
    let filename = path.split('/').last().unwrap_or(path);
    let safe_name = if filename.len() > 30 { format!("{}...", &filename[..27]) } else { filename.to_string() };
    format!(
        r##"<!-- Iris JetCrab v0.1.1 - Placeholder Image -->
<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#faf8ff"/>
      <stop offset="100%" stop-color="#f5f0ff"/>
    </linearGradient>
    <linearGradient id="border" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#e8d5ff"/>
      <stop offset="50%" stop-color="#b8d4ff"/>
      <stop offset="100%" stop-color="#ffe0b0"/>
    </linearGradient>
  </defs>
  <rect width="400" height="300" fill="url(#bg)" rx="12"/>
  <rect x="2" y="2" width="396" height="296" fill="none" stroke="url(#border)" stroke-width="2" rx="12"/>
  <text x="200" y="140" text-anchor="middle" font-size="64">&#128196;</text>
  <text x="200" y="195" text-anchor="middle" font-size="14" font-family="sans-serif" fill="#b0a8c0">{}</text>
  <text x="200" y="285" text-anchor="middle" font-size="10" font-family="sans-serif" fill="#ccc0d8">Iris JetCrab placeholder</text>
</svg>"##,
        safe_name
    )
}

/// Fallback 处理器 - 处理 favicon、图片和其他静态资源
///
/// 当请求的文件在项目根目录不存在时：
/// - /__iris-favicon.svg 或 /favicon.ico → 返回彩虹 emoji favicon
/// - 其他图片文件 → 返回占位 SVG
pub async fn fallback_handler(
    State(state): State<ServerState>,
    uri: Uri,
) -> Response {
    let (cache, _enable_hmr, _ws_manager): (Arc<tokio::sync::Mutex<CompilerCache>>, bool, Arc<WebSocketManager>) = state;
    let cache_lock = cache.lock().await;
    let project_root = cache_lock.project_root.clone();
    drop(cache_lock);

    let path = uri.path();
    if path == "/" {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
            "Not Found".to_string(),
        ).into_response();
    }

    let relative_path = path.trim_start_matches('/');
    let file_path = project_root.join(relative_path);

    // 特殊处理 Iris 彩虹 favicon
    if path == "/__iris-favicon.svg" || path == "/favicon.ico" {
        if path == "/favicon.ico" && file_path.exists() && file_path.is_file() {
            // 项目实际有 favicon.ico，读取并返回
            match tokio::fs::read(&file_path).await {
                Ok(data) => {
                    return (
                        StatusCode::OK,
                        [
                            (header::CONTENT_TYPE, "image/x-icon"),
                            (header::CACHE_CONTROL, "public, max-age=3600"),
                        ],
                        data,
                    ).into_response();
                }
                Err(_) => {}
            }
        }
        // 返回彩虹 emoji SVG favicon
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/svg+xml"),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            rainbow_favicon_svg(),
        ).into_response();
    }

    // 图片文件：存在则返回，不存在则生成占位图
    if is_image_path(path) {
        if file_path.exists() && file_path.is_file() {
            match tokio::fs::read(&file_path).await {
                Ok(data) => {
                    let ct = get_content_type(path);
                    return (
                        StatusCode::OK,
                        [
                            (header::CONTENT_TYPE, ct),
                            (header::CACHE_CONTROL, "public, max-age=3600"),
                        ],
                        data,
                    ).into_response();
                }
                Err(_) => {}
            }
        }
        // 生成占位 SVG
        let placeholder = placeholder_svg(relative_path);
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/svg+xml"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            placeholder,
        ).into_response();
    }

    // 其他文件类型
    if file_path.exists() && file_path.is_file() {
        match tokio::fs::read(&file_path).await {
            Ok(data) => {
                let ct = get_content_type(path);
                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, ct),
                        (header::CACHE_CONTROL, "public, max-age=3600"),
                    ],
                    data,
                ).into_response();
            }
            Err(_) => {}
        }
    }

    // 404
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/plain")],
        "Not Found".to_string(),
    ).into_response()
}
