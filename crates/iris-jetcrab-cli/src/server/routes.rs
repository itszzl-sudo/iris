//! HTTP 路由处理器

use axum::{
    response::{Html, Json, IntoResponse, Response},
    extract::{State, Path},
};
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::http::{StatusCode, header};
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
        
        // 加载入口模块
        async function loadApp() {
            try {
                // 请求编译入口文件（使用 /src/ 路径）
                const response = await fetch('/src/main.ts');
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }
                
                // 直接获取 JavaScript 代码并执行
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
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    let package_name = parts[0];
    let sub_path = if parts.len() > 1 { parts[1] } else { "" };
    
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
                            let rewritten_js = rewrite_bare_imports(&js_with_env);
                            
                            // 返回 JavaScript 模块
                            return (axum::http::StatusCode::OK, [
                                (header::CONTENT_TYPE, "application/javascript"),
                            ], rewritten_js).into_response();
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
                rewrite_bare_imports(&js_with_env)
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

/// 重写裸模块导入（bare module imports）为 /@npm/ 路径
/// 
/// 例如：
/// - `import { ref } from 'vue'` → `import { ref } from '/@npm/vue'`
/// - `import { defineStore } from 'pinia'` → `import { defineStore } from '/@npm/pinia'`
fn rewrite_bare_imports(script: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;
    
    // 匹配 import ... from 'package' 或 import ... from "package"
    // 不使用 lookahead，简单匹配所有 from 'xxx'
    static BARE_IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"from\s+['"]([^'"]+)['"]"#).unwrap()
    });
    
    BARE_IMPORT_RE.replace_all(script, |caps: &regex::Captures| {
        let package_name = &caps[1];
        
        // 跳过相对路径和绝对路径
        if package_name.starts_with("./") || 
           package_name.starts_with("../") || 
           package_name.starts_with("/") {
            // 重写相对路径为 /src/ 路径
            if package_name.starts_with("./") || package_name.starts_with("../") {
                // 提取文件名，转换为 /src/ 路径
                // 例如：./App.vue → /src/App.vue
                let filename = if package_name.starts_with("./") {
                    &package_name[2..]
                } else {
                    // 处理 ../ 的情况
                    package_name.split('/').last().unwrap_or(package_name)
                };
                format!("from '/src/{}'", filename)
            } else {
                // 保持原样
                caps[0].to_string()
            }
        } else {
            // 重写为 /@npm/ 路径
            format!("from '/@npm/{}'", package_name)
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
        let mut cache_lock = cache.lock().await;
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
            let rewritten_script = rewrite_bare_imports(&module.script);
            
            // 添加编译后的脚本
            js_code.push_str(&rewritten_script);
            
            // 注入样式
            for style in &module.styles {
                js_code.push_str("\n\n/* Injected Styles */\n");
                js_code.push_str("const style = document.createElement('style');\n");
                js_code.push_str("style.textContent = `");
                js_code.push_str(&style.code.replace('`', "\\`"));
                js_code.push_str("`;\n");
                js_code.push_str("document.head.appendChild(style);\n");
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
