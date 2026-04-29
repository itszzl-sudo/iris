//! HTTP 路由处理器

use axum::{
    response::{Html, Json, IntoResponse},
    extract::{State, Path},
};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use futures_util::SinkExt;
use tracing::{info, debug, warn};
use crate::server::compiler_cache::CompilerCache;
use crate::server::hmr::{HMRManager, WebSocketManager};
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
        let mut cache_lock = cache.lock().await;
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
    let (cached_count, _) = cache_lock.stats();
    
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
    if project_index.exists() {
        if let Ok(content) = std::fs::read_to_string(&project_index) {
            return content;
        }
    }
    
    // 默认模板
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
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
        // Iris JetCrab - Runtime On-Demand Compilation
        console.log('🦀 Iris JetCrab Runtime');
        console.log('📦 Compilation: On-demand');
        
        // 加载入口模块
        async function loadApp() {
            try {
                // 请求编译入口文件
                const response = await fetch('/@vue/main.js');
                const module = await response.json();
                
                console.log('✅ Entry module loaded');
                console.log('📝 Styles:', module.styles.length);
                console.log('🔗 Dependencies:', module.deps.length);
                
                // 注入样式
                module.styles.forEach(style => {
                    const styleEl = document.createElement('style');
                    styleEl.textContent = style.code;
                    document.head.appendChild(styleEl);
                });
                
                // 执行脚本
                const scriptEl = document.createElement('script');
                scriptEl.type = 'module';
                scriptEl.textContent = module.code;
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
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".html") {
        "text/html"
    } else {
        "application/octet-stream"
    }
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
