//! AI Inspector — 可视化元素检查器 + AI 代码编辑
//!
//! 注入到开发服务器页面的覆盖层，提供：
//! 1. 右键/双击元素 → 高亮 + 展示关联源文件 + 代码段
//! 2. AI 输入框 → 直接请求 iris-ai 修改代码
//! 3. 实时预览（不修改源文件）
//! 4. 应用按钮 → 写入源文件

use axum::{
    extract::{State, Query},
    Json,
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, Mutex};
use std::fs;
use tokio::sync::Mutex as TokioMutex;
use tracing::{info, warn, error};
use anyhow::Result;

use crate::server::compiler_cache::CompilerCache;
use crate::server::hmr::WebSocketManager;

/// 服务器状态
pub type ServerState = (Arc<TokioMutex<CompilerCache>>, bool, Arc<WebSocketManager>);

// ─── 数据结构 ───────────────────────────────────────────────────

/// 元素源文件查询请求
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ElementSourceRequest {
    /// 元素标签名，如 "div"
    pub tag: String,
    /// 元素 id
    pub id: Option<String>,
    /// 元素 class 列表
    pub classes: Vec<String>,
    /// 组件源文件路径（从 data-iris-component 获取）
    pub component_path: Option<String>,
}

/// 元素源文件查询响应
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ElementSourceResponse {
    /// 源文件路径（相对于项目根目录）
    pub file_path: String,
    /// 文件内容
    pub content: String,
    /// 代码行数
    pub line_count: usize,
    /// 文件类型
    pub file_type: String,
    /// 是否为 npm 包
    pub is_npm_package: bool,
    /// 包名（如果是 npm 包）
    pub package_name: Option<String>,
    /// 包版本（如果是 npm 包）
    pub package_version: Option<String>,
}

/// AI 编辑请求
#[derive(Debug, Deserialize)]
pub struct AiEditRequest {
    /// 文件路径（相对于项目根目录）
    pub file_path: String,
    /// 修改指令
    pub instruction: String,
}

/// AI 编辑响应
#[derive(Debug, Serialize)]
pub struct AiEditResponse {
    /// 修改后的代码
    pub modified_code: String,
    /// 原始代码
    pub original_code: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 是否是模拟模式（模型未加载时返回模拟结果）
    pub simulated: bool,
}

/// 应用编辑请求
#[derive(Debug, Deserialize)]
pub struct ApplyEditRequest {
    /// 文件路径（相对于项目根目录）
    pub file_path: String,
    /// 新代码内容
    pub new_content: String,
    /// 是否创建新文件（默认 false）
    pub create_new: Option<bool>,
    /// 输出文件路径（指定则写入新路径）
    pub output_path: Option<String>,
}

/// 应用编辑响应
#[derive(Debug, Serialize)]
pub struct ApplyEditResponse {
    /// 是否成功
    pub success: bool,
    /// 写入的文件路径
    pub written_path: String,
    /// 是否是新建文件
    pub is_new_file: bool,
    /// 错误信息
    pub error: Option<String>,
}

// ─── 全局 AI 助手（懒初始化） ─────────────────────────────

static AI_ASSISTANT: OnceLock<Mutex<Option<iris_ai::AiAssistant>>> = OnceLock::new();

fn get_or_init_ai() -> Result<std::sync::MutexGuard<'static, Option<iris_ai::AiAssistant>>> {
    let lock = AI_ASSISTANT.get_or_init(|| {
        info!("Initializing AI assistant (lazy)");
        let mut config = iris_ai::AiConfig::default();
        config.temperature = 0.15;
        config.top_p = 0.9;
        config.max_tokens = 2048;
        let assistant = iris_ai::AiAssistant::new(config)
            .build()
            .map_err(|e| {
                warn!("Failed to initialize AI assistant: {}", e);
                e
            });
        Mutex::new(assistant.ok())
    });
    
    let guard = lock.lock().map_err(|e| anyhow::anyhow!("AI mutex poisoned: {}", e))?;
    Ok(guard)
}

// ─── 文件类型检测 ────────────────────────────────────────────────

fn detect_file_type(path: &str) -> &'static str {
    let ext = Path::new(path).extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext.to_lowercase().as_str() {
        "vue" => "vue",
        "ts" => "typescript",
        "tsx" => "tsx",
        "js" => "javascript",
        "jsx" => "jsx",
        "css" => "css",
        "scss" => "scss",
        "less" => "less",
        "html" => "html",
        "json" => "json",
        "md" => "markdown",
        _ => "text",
    }
}

/// 读取项目中的源文件
fn read_source_file(project_root: &Path, file_path: &str) -> Result<String> {
    // 如果是绝对路径，直接读取
    let full_path = if Path::new(file_path).is_absolute() {
        PathBuf::from(file_path)
    } else {
        // 尝试多种路径
        let candidates = [
            project_root.join(file_path),
            project_root.join("src").join(file_path),
            project_root.join(file_path.trim_start_matches('/')),
            project_root.join("src").join(file_path.trim_start_matches('/')),
        ];
        
        let mut found = None;
        for p in &candidates {
            if p.exists() {
                found = Some(p.clone());
                break;
            }
        }
        found.ok_or_else(|| anyhow::anyhow!("File not found: {}", file_path))?
    };
    
    let content = fs::read_to_string(&full_path)?;
    Ok(content)
}

/// 从 node_modules 读取 npm 包内容
fn read_npm_package(project_root: &Path, package_path: &str) -> Result<(String, String, String)> {
    // 解析包名：vue/dist/vue.esm.js → vue
    let node_modules = project_root.join("node_modules");
    let parts: Vec<&str> = package_path.split('/').collect();
    
    // 处理 scoped packages (@vue/xxx)
    let (package_name, relative_path) = if package_path.starts_with('@') {
        if parts.len() >= 2 {
            (format!("{}/{}", parts[0], parts[1]), parts[2..].join("/"))
        } else {
            (package_path.to_string(), String::new())
        }
    } else {
        if parts.len() >= 1 {
            (parts[0].to_string(), parts[1..].join("/"))
        } else {
            (package_path.to_string(), String::new())
        }
    };
    
    let pkg_dir = node_modules.join(&package_name);
    let pkg_json_path = pkg_dir.join("package.json");
    
    // 读取版本号
    let version = if pkg_json_path.exists() {
        if let Ok(content) = fs::read_to_string(&pkg_json_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                val.get("version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        }
    } else {
        "unknown".to_string()
    };
    
    // 读取源码
    let source_path = if relative_path.is_empty() {
        // 读取入口文件
        if pkg_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&pkg_json_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    let main = val.get("main")
                        .or_else(|| val.get("module"))
                        .or_else(|| val.get("exports"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("index.js");
                    pkg_dir.join(main)
                } else {
                    pkg_dir.join("index.js")
                }
            } else {
                pkg_dir.join("index.js")
            }
        } else {
            pkg_dir.join("index.js")
        }
    } else {
        pkg_dir.join(&relative_path)
    };
    
    let content = if source_path.exists() {
        fs::read_to_string(&source_path)?
    } else {
        format!("// {} (source file not found)", package_path)
    };
    
    Ok((package_name, version, content))
}

// ─── API 处理器 ──────────────────────────────────────────────────

/// 元素源文件查询 API
///
/// 路由: POST /api/element-source
pub async fn element_source_handler(
    State(state): State<ServerState>,
    Json(req): Json<ElementSourceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (cache, _, _) = state;
    let cache_lock = cache.lock().await;
    let project_root = &cache_lock.project_root;
    
    // 如果有组件路径，直接读取对应文件
    if let Some(ref comp_path) = req.component_path {
        let relative_path = comp_path.trim_start_matches('/');
        
        // 检查是否是 npm 包路径
        if is_npm_package_path(relative_path) {
            // 从 node_modules 读取
            match read_npm_package(project_root, relative_path) {
                Ok((pkg_name, pkg_ver, content)) => {
                    let line_count = content.lines().count();
                    return Ok(Json(json!({
                        "file_path": relative_path,
                        "content": content,
                        "line_count": line_count,
                        "file_type": detect_file_type(&pkg_name),
                        "is_npm_package": true,
                        "package_name": pkg_name,
                        "package_version": pkg_ver,
                    })));
                }
                Err(e) => {
                    return Ok(Json(json!({
                        "file_path": relative_path,
                        "content": format!("// Error reading npm package: {}", e),
                        "line_count": 0,
                        "file_type": "text",
                        "is_npm_package": true,
                        "package_name": relative_path,
                        "package_version": "unknown",
                    })));
                }
            }
        }
        
        // 尝试读取源文件
        match read_source_file(project_root, relative_path) {
            Ok(content) => {
                let line_count = content.lines().count();
                return Ok(Json(json!({
                    "file_path": relative_path,
                    "content": content,
                    "line_count": line_count,
                    "file_type": detect_file_type(relative_path),
                    "is_npm_package": false,
                    "package_name": null,
                    "package_version": null,
                })));
            }
            Err(e) => {
                return Err((StatusCode::NOT_FOUND, Json(json!({
                    "error": format!("File not found: {} - {}", relative_path, e)
                }))));
            }
        }
    }
    
    // 没有组件路径，返回错误
    Err((StatusCode::BAD_REQUEST, Json(json!({
        "error": "No component path provided"
    }))))
}

/// 判断路径是否指向 npm 包
fn is_npm_package_path(path: &str) -> bool {
    // 不以 src/ 或 /src/ 开头，且不是绝对路径
    if path.starts_with("src/") || path.starts_with("/src/") || Path::new(path).is_absolute() {
        return false;
    }
    // 包含点号（文件扩展名）但不是典型的项目源文件扩展名
    // 更准确：排在 node_modules 目录中的才是
    true // 简单策略：一切非 src/ 开头的都是 npm 包
}

/// 文件内容 API
///
/// 路由: GET /api/file-content
pub async fn file_content_handler(
    State(state): State<ServerState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (cache, _, _) = state;
    let cache_lock = cache.lock().await;
    let project_root = &cache_lock.project_root;
    
    let file_path = params.get("path")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, Json(json!({"error": "Missing 'path' parameter"}))))?;
    
    let relative_path = file_path.trim_start_matches('/');
    
    match read_source_file(project_root, relative_path) {
        Ok(content) => {
            Ok(Json(json!({
                "file_path": relative_path,
                "content": content,
                "line_count": content.lines().count(),
                "file_type": detect_file_type(relative_path),
            })))
        }
        Err(e) => {
            Err((StatusCode::NOT_FOUND, Json(json!({
                "error": format!("File not found: {}", e)
            }))))
        }
    }
}

/// AI 编辑 API
///
/// 路由: POST /api/ai-edit
pub async fn ai_edit_handler(
    State(state): State<ServerState>,
    Json(req): Json<AiEditRequest>,
) -> Json<serde_json::Value> {
    let (cache, _, _) = state;
    let cache_lock = cache.lock().await;
    let project_root = cache_lock.project_root.clone();
    
    // 读取源文件
    let relative_path = req.file_path.trim_start_matches('/');
    let code = match read_source_file(&project_root, relative_path) {
        Ok(c) => c,
        Err(e) => {
            return Json(json!(AiEditResponse {
                modified_code: String::new(),
                original_code: String::new(),
                success: false,
                error: Some(format!("Failed to read file: {}", e)),
                simulated: false,
            }));
        }
    };
    
    // 尝试获取 AI 助手
    let ai_guard = match get_or_init_ai() {
        Ok(g) => g,
        Err(e) => {
            // AI 不可用，返回模拟修改
            info!("AI not available, returning simulated edit: {}", e);
            let simulated = simulate_edit(&code, &req.instruction);
            return Json(json!(AiEditResponse {
                modified_code: simulated,
                original_code: code,
                success: true,
                error: Some(format!("AI model not loaded: {}. Shown as simulated suggestion.", e)),
                simulated: true,
            }));
        }
    };
    
    let mut ai = match ai_guard.as_ref() {
        Some(_ai_ref) => {
            // 需要重新构建 AI，因为全局锁中只能获取 & 不能获取 &mut
            drop(ai_guard);
            match rebuild_ai() {
                Ok(ai) => ai,
                Err(e) => {
                    info!("AI rebuild failed, returning simulated edit: {}", e);
                    let simulated = simulate_edit(&code, &req.instruction);
                    return Json(json!(AiEditResponse {
                        modified_code: simulated,
                        original_code: code,
                        success: true,
                        error: Some(format!("AI error: {}. Shown as simulated suggestion.", e)),
                        simulated: true,
                    }));
                }
            }
        }
        None => {
            // AI 不可用，返回模拟修改
            let simulated = simulate_edit(&code, &req.instruction);
            return Json(json!(AiEditResponse {
                modified_code: simulated,
                original_code: code,
                success: true,
                error: Some("AI model not available. Shown as simulated suggestion.".to_string()),
                simulated: true,
            }));
        }
    };
    
    // 执行 AI 编辑
    match ai.edit_code(relative_path, &req.instruction, &code) {
        Ok(modified_code) => {
            Json(json!(AiEditResponse {
                modified_code,
                original_code: code,
                success: true,
                error: None,
                simulated: false,
            }))
        }
        Err(e) => {
            error!("AI edit failed: {}", e);
            let simulated = simulate_edit(&code, &req.instruction);
            Json(json!(AiEditResponse {
                modified_code: simulated,
                original_code: code,
                success: true,
                error: Some(format!("AI edit error: {}. Shown as simulated suggestion.", e)),
                simulated: true,
            }))
        }
    }
}

/// 重新构建 AI 助手
fn rebuild_ai() -> Result<iris_ai::AiAssistant> {
    let mut config = iris_ai::AiConfig::default();
    config.temperature = 0.15;
    config.top_p = 0.9;
    config.max_tokens = 2048;
    let assistant = iris_ai::AiAssistant::new(config).build()?;
    Ok(assistant)
}

/// 模拟编辑（AI 不可用时的后备方案）
fn simulate_edit(code: &str, instruction: &str) -> String {
    // 生成一个带注释的模拟修改
    let comment = format!(
        "/* === AI Suggestion (simulated) ===\n * Instruction: {}\n * =============================== */\n",
        instruction
    );
    format!("{}{}", comment, code)
}

/// 应用编辑 API
///
/// 路由: POST /api/apply-edit
pub async fn apply_edit_handler(
    State(state): State<ServerState>,
    Json(req): Json<ApplyEditRequest>,
) -> Json<serde_json::Value> {
    let (cache, _, _) = state;
    let cache_lock = cache.lock().await;
    let project_root = cache_lock.project_root.clone();
    
    let relative_path = req.file_path.trim_start_matches('/');
    let create_new = req.create_new.unwrap_or(false);
    
    // 确定写入路径
    let write_path = if let Some(ref output) = req.output_path {
        let out = output.trim_start_matches('/');
        if Path::new(out).is_absolute() {
            PathBuf::from(out)
        } else {
            project_root.join(out)
        }
    } else {
        project_root.join(relative_path)
    };
    
    // 检查文件是否存在
    let is_new_file = !write_path.exists();
    if is_new_file && !create_new {
        return Json(json!(ApplyEditResponse {
            success: false,
            written_path: write_path.to_string_lossy().to_string(),
            is_new_file: true,
            error: Some("File does not exist. Set create_new=true to create it.".to_string()),
        }));
    }
    
    // 写入文件
    match fs::write(&write_path, &req.new_content) {
        Ok(_) => {
            info!("Applied edit to: {}", write_path.display());
            Json(json!(ApplyEditResponse {
                success: true,
                written_path: write_path.to_string_lossy().to_string(),
                is_new_file,
                error: None,
            }))
        }
        Err(e) => {
            error!("Failed to write file: {}", e);
            Json(json!(ApplyEditResponse {
                success: false,
                written_path: write_path.to_string_lossy().to_string(),
                is_new_file,
                error: Some(format!("Failed to write file: {}", e)),
            }))
        }
    }
}

/// npm 包信息 API
///
/// 路由: GET /api/npm-package-info
pub async fn npm_package_info_handler(
    State(state): State<ServerState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (cache, _, _) = state;
    let cache_lock = cache.lock().await;
    let project_root = cache_lock.project_root.clone();
    
    let package_path = params.get("path")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, Json(json!({"error": "Missing 'path' parameter"}))))?;
    
    match read_npm_package(&project_root, package_path) {
        Ok((pkg_name, pkg_ver, content)) => {
            Ok(Json(json!({
                "package_name": pkg_name,
                "version": pkg_ver,
                "content": content,
                "line_count": content.lines().count(),
                "file_type": detect_file_type(&pkg_name),
            })))
        }
        Err(e) => {
            Err((StatusCode::NOT_FOUND, Json(json!({
                "error": format!("Package not found: {}", e)
            }))))
        }
    }
}

// ─── Inspector Overlay 注入 ─────────────────────────────────────

/// Inspector Overlay CSS
const INSPECTOR_OVERLAY_CSS: &str = r##"
#iris-inspector-overlay {
    all: initial;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: #e0e0e0;
    box-sizing: border-box;
}
#iris-inspector-overlay * {
    all: unset;
    display: revert;
    box-sizing: border-box;
}
.iris-io-container {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    pointer-events: none;
    z-index: 2147483647;
}
.iris-io-container.active {
    pointer-events: auto;
}
.iris-io-highlight {
    position: absolute;
    border: 2px solid #4fc3f7;
    background: rgba(79, 195, 247, 0.1);
    pointer-events: none;
    z-index: 2147483646;
    transition: all 0.15s ease;
}
.iris-io-tooltip {
    position: fixed;
    background: #1a1a2e;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 12px;
    color: #ccc;
    pointer-events: none;
    z-index: 2147483647;
    white-space: nowrap;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}
.iris-io-panel {
    position: fixed;
    top: 0;
    right: -480px;
    width: 480px;
    height: 100vh;
    background: #1a1a2e;
    border-left: 1px solid #333;
    display: flex;
    flex-direction: column;
    z-index: 2147483647;
    transition: right 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: -4px 0 24px rgba(0,0,0,0.4);
    pointer-events: auto;
}
.iris-io-panel.open {
    right: 0;
}
.iris-io-panel-header {
    padding: 16px 20px;
    border-bottom: 1px solid #333;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
}
.iris-io-panel-header h3 {
    font-size: 15px;
    font-weight: 600;
    color: #e0e0e0;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 8px;
}
.iris-io-panel-header .close-btn {
    background: none;
    border: 1px solid #444;
    color: #999;
    width: 28px;
    height: 28px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    pointer-events: auto;
}
.iris-io-panel-header .close-btn:hover {
    background: #333;
    color: #fff;
}
.iris-io-file-info {
    padding: 12px 20px;
    background: #16162a;
    border-bottom: 1px solid #333;
    flex-shrink: 0;
}
.iris-io-file-path {
    font-size: 13px;
    color: #8ab4f8;
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    word-break: break-all;
}
.iris-io-package-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    background: #2a2a4a;
    color: #bb86fc;
    margin-top: 4px;
}
.iris-io-code-area {
    flex: 1;
    overflow: auto;
    background: #0f0f23;
    padding: 0;
}
.iris-io-code-area pre {
    margin: 0;
    padding: 16px 20px;
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    line-height: 1.6;
    color: #e0e0e0;
    white-space: pre;
    tab-size: 2;
}
.iris-io-code-area .line-numbers {
    display: flex;
}
.iris-io-code-area .line-nums {
    color: #555;
    text-align: right;
    padding: 16px 12px 16px 20px;
    user-select: none;
    border-right: 1px solid #222;
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    line-height: 1.6;
}
.iris-io-code-area .line-code {
    flex: 1;
    padding: 16px 20px;
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    line-height: 1.6;
    color: #e0e0e0;
    white-space: pre;
    overflow-x: auto;
}
.iris-io-ai-section {
    border-top: 1px solid #333;
    flex-shrink: 0;
}
.iris-io-ai-input-area {
    padding: 12px 20px;
    background: #16162a;
}
.iris-io-ai-input {
    width: 100%;
    min-height: 60px;
    max-height: 120px;
    padding: 10px 12px;
    background: #0f0f23;
    border: 1px solid #333;
    border-radius: 6px;
    color: #e0e0e0;
    font-family: inherit;
    font-size: 13px;
    line-height: 1.5;
    resize: vertical;
    outline: none;
    pointer-events: auto;
}
.iris-io-ai-input:focus {
    border-color: #4fc3f7;
}
.iris-io-ai-input::placeholder {
    color: #666;
}
.iris-io-ai-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
    align-items: center;
}
.iris-io-btn {
    padding: 6px 16px;
    border: 1px solid #444;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.15s ease;
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 6px;
}
.iris-io-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}
.iris-io-btn-primary {
    background: #4fc3f7;
    color: #000;
    border-color: #4fc3f7;
}
.iris-io-btn-primary:hover:not(:disabled) {
    background: #29b6f6;
}
.iris-io-btn-primary:disabled {
    background: #333;
    color: #666;
    border-color: #333;
}
.iris-io-btn-success {
    background: #66bb6a;
    color: #000;
    border-color: #66bb6a;
}
.iris-io-btn-success:hover:not(:disabled) {
    background: #43a047;
}
.iris-io-btn-outline {
    background: transparent;
    color: #aaa;
}
.iris-io-btn-outline:hover:not(:disabled) {
    background: #222;
    color: #e0e0e0;
}
.iris-io-btn-danger {
    background: transparent;
    color: #ef5350;
    border-color: #ef5350;
}
.iris-io-btn-danger:hover:not(:disabled) {
    background: rgba(239,83,80,0.15);
}
.iris-io-spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: #fff;
    border-radius: 50%;
    animation: iris-io-spin 0.6s linear infinite;
}
@keyframes iris-io-spin {
    to { transform: rotate(360deg); }
}
.iris-io-status {
    font-size: 12px;
    color: #888;
    margin-left: auto;
}
.iris-io-status.loading { color: #4fc3f7; }
.iris-io-status.success { color: #66bb6a; }
.iris-io-status.error { color: #ef5350; }
.iris-io-diff-area {
    border-top: 1px solid #333;
    max-height: 200px;
    overflow: auto;
    background: #0f0f23;
    display: none;
}
.iris-io-diff-area.show {
    display: block;
}
.iris-io-diff-header {
    padding: 8px 20px;
    background: #16162a;
    font-size: 12px;
    color: #888;
    display: flex;
    gap: 16px;
    border-bottom: 1px solid #222;
}
.iris-io-diff-content {
    padding: 12px 20px;
    font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
    font-size: 13px;
    line-height: 1.6;
    white-space: pre;
}
.iris-io-diff-added {
    background: rgba(102, 187, 106, 0.15);
    color: #a5d6a7;
}
.iris-io-diff-removed {
    background: rgba(239, 83, 80, 0.15);
    color: #ef9a9a;
}
.iris-io-element-tag {
    display: inline-block;
    padding: 1px 6px;
    background: #2a2a4a;
    border-radius: 3px;
    font-family: 'SF Mono', monospace;
    font-size: 12px;
    color: #ffa726;
    margin-right: 4px;
}
.iris-io-element-class {
    color: #8ab4f8;
    font-family: 'SF Mono', monospace;
    font-size: 12px;
}
.iris-io-element-id {
    color: #ffa726;
    font-family: 'SF Mono', monospace;
    font-size: 12px;
}
.iris-io-loading-overlay {
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(15,15,35,0.85);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    gap: 12px;
    z-index: 10;
    pointer-events: auto;
}
.iris-io-loading-overlay .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(79,195,247,0.2);
    border-top-color: #4fc3f7;
    border-radius: 50%;
    animation: iris-io-spin 0.6s linear infinite;
}
.iris-io-loading-overlay .text {
    font-size: 13px;
    color: #888;
}
.iris-io-toggle-btn {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 2147483645;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: #1a1a2e;
    border: 1px solid #333;
    color: #888;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
    transition: all 0.2s;
    pointer-events: auto;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}
.iris-io-toggle-btn:hover {
    background: #2a2a4a;
    color: #4fc3f7;
}
.iris-io-toggle-btn.active {
    background: #4fc3f7;
    color: #000;
    border-color: #4fc3f7;
}
.iris-io-preview-frame {
    width: 100%;
    height: 100%;
    border: none;
    position: absolute;
    top: 0; left: 0;
}
.iris-io-preview-container {
    position: relative;
    width: 100%;
    flex: 1;
    display: none;
}
.iris-io-preview-container.show {
    display: block;
}
"##;

/// Inspector Overlay JavaScript
const INSPECTOR_OVERLAY_JS: &str = r##"
(function() {
    'use strict';
    
    // 状态
    let isInspecting = false;
    let currentPanel = null;
    let selectedElement = null;
    let currentSource = null;
    let aiResponse = null;
    
    // DOM 引用
    let container, highlight, tooltip, panel, toggleBtn;
    
    function init() {
        if (document.getElementById('iris-inspector-overlay')) return;
        
        const overlay = document.createElement('div');
        overlay.id = 'iris-inspector-overlay';
        overlay.innerHTML = `
            <div class="iris-io-container" id="iris-io-container">
                <div class="iris-io-highlight" id="iris-io-highlight"></div>
                <div class="iris-io-tooltip" id="iris-io-tooltip"></div>
                <div class="iris-io-panel" id="iris-io-panel">
                    <div class="iris-io-panel-header">
                        <h3>Inspector</h3>
                        <button class="close-btn" onclick="irisIO.closePanel()">x</button>
                    </div>
                    <div class="iris-io-file-info" id="iris-io-file-info">
                        <div class="iris-io-file-path">Click an element to inspect</div>
                    </div>
                    <div class="iris-io-code-area" id="iris-io-code-area">
                        <div class="iris-io-loading-overlay" id="iris-io-loading" style="display:none;">
                            <div class="spinner"></div>
                            <div class="text">Loading...</div>
                        </div>
                    </div>
                    <div class="iris-io-diff-area" id="iris-io-diff-area"></div>
                    <div class="iris-io-preview-container" id="iris-io-preview-container"></div>
                    <div class="iris-io-ai-section">
                        <div class="iris-io-ai-input-area">
                            <textarea class="iris-io-ai-input" id="iris-io-ai-input"
                                placeholder="Describe the changes you want (e.g., 'Change background to blue, add padding 20px, make text bold')"
                                rows="3"></textarea>
                            <div class="iris-io-ai-actions">
                                <button class="iris-io-btn iris-io-btn-primary" id="iris-io-ask-ai-btn"
                                    onclick="irisIO.askAI()">AI Edit</button>
                                <button class="iris-io-btn iris-io-btn-success" id="iris-io-apply-btn"
                                    onclick="irisIO.applyEdit()" style="display:none;">Apply to Source</button>
                                <button class="iris-io-btn iris-io-btn-outline" id="iris-io-preview-btn"
                                    onclick="irisIO.togglePreview()" style="display:none;">Preview</button>
                                <span class="iris-io-status" id="iris-io-status"></span>
                            </div>
                        </div>
                    </div>
                </div>
                <button class="iris-io-toggle-btn" id="iris-io-toggle-btn"
                    onclick="irisIO.toggleInspector()" title="Toggle Inspector (Ctrl+Shift+I)">AI</button>
            </div>
        `;
        document.body.appendChild(overlay);
        
        // 缓存 DOM 引用
        container = document.getElementById('iris-io-container');
        highlight = document.getElementById('iris-io-highlight');
        tooltip = document.getElementById('iris-io-tooltip');
        panel = document.getElementById('iris-io-panel');
        toggleBtn = document.getElementById('iris-io-toggle-btn');
    }
    
    // 公共 API
    window.irisIO = {
        init: init,
        
        toggleInspector: function() {
            isInspecting = !isInspecting;
            container.classList.toggle('active', isInspecting);
            toggleBtn.classList.toggle('active', isInspecting);
            if (isInspecting) {
                document.addEventListener('dblclick', onElementClick, true);
                document.addEventListener('contextmenu', onRightClick, true);
                document.addEventListener('mousemove', onHover, true);
                document.addEventListener('click', onSingleClick, true);
            } else {
                document.removeEventListener('dblclick', onElementClick, true);
                document.removeEventListener('contextmenu', onRightClick, true);
                document.removeEventListener('mousemove', onHover, true);
                document.removeEventListener('click', onSingleClick, true);
                hideHighlight();
                document.body.style.cursor = '';
            }
        },
        
        closePanel: function() {
            panel.classList.remove('open');
            selectedElement = null;
            aiResponse = null;
            document.getElementById('iris-io-diff-area').classList.remove('show');
            document.getElementById('iris-io-preview-container').classList.remove('show');
            document.getElementById('iris-io-apply-btn').style.display = 'none';
            document.getElementById('iris-io-preview-btn').style.display = 'none';
        },
        
        askAI: function() {
            const input = document.getElementById('iris-io-ai-input');
            const instruction = input.value.trim();
            if (!instruction) return;
            if (!currentSource) return;
            
            const status = document.getElementById('iris-io-status');
            status.textContent = 'AI thinking...';
            status.className = 'iris-io-status loading';
            document.getElementById('iris-io-ask-ai-btn').disabled = true;
            
            fetch('/api/ai-edit', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    file_path: currentSource.file_path,
                    instruction: instruction
                })
            })
            .then(r => r.json())
            .then(data => {
                aiResponse = data;
                if (data.success) {
                    status.textContent = data.simulated ? 'Simulated suggestion' : 'AI done';
                    status.className = 'iris-io-status success';
                    showDiff(data.original_code, data.modified_code);
                    document.getElementById('iris-io-apply-btn').style.display = '';
                    document.getElementById('iris-io-preview-btn').style.display = '';
                } else {
                    status.textContent = 'Error: ' + (data.error || 'Unknown error');
                    status.className = 'iris-io-status error';
                }
            })
            .catch(err => {
                status.textContent = 'Error: ' + err.message;
                status.className = 'iris-io-status error';
            })
            .finally(() => {
                document.getElementById('iris-io-ask-ai-btn').disabled = false;
            });
        },
        
        applyEdit: function() {
            if (!aiResponse || !aiResponse.success) return;
            
            const status = document.getElementById('iris-io-status');
            status.textContent = 'Applying...';
            status.className = 'iris-io-status loading';
            document.getElementById('iris-io-apply-btn').disabled = true;
            
            fetch('/api/apply-edit', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    file_path: currentSource.file_path,
                    new_content: aiResponse.modified_code,
                    create_new: false
                })
            })
            .then(r => r.json())
            .then(data => {
                if (data.success) {
                    status.textContent = data.is_new_file ? 'Created new file!' : 'Applied to source!';
                    status.className = 'iris-io-status success';
                    // 更新显示的代码
                    currentSource.content = aiResponse.modified_code;
                    displaySourceCode(currentSource);
                } else {
                    status.textContent = 'Error: ' + (data.error || 'Failed to apply');
                    status.className = 'iris-io-status error';
                }
            })
            .catch(err => {
                status.textContent = 'Error: ' + err.message;
                status.className = 'iris-io-status error';
            })
            .finally(() => {
                document.getElementById('iris-io-apply-btn').disabled = false;
            });
        },
        
        togglePreview: function() {
            const previewContainer = document.getElementById('iris-io-preview-container');
            previewContainer.classList.toggle('show');
            if (previewContainer.classList.contains('show')) {
                // 创建预览 iframe
                let iframe = previewContainer.querySelector('iframe');
                if (!iframe) {
                    iframe = document.createElement('iframe');
                    iframe.className = 'iris-io-preview-frame';
                    previewContainer.appendChild(iframe);
                }
                // 写入修改后的代码
                const doc = iframe.contentDocument || iframe.contentWindow.document;
                doc.open();
                doc.write('<html><head><style>body{margin:0;font-family:sans-serif;}</style></head><body>');
                doc.write('<div id="app" style="width:100vw;height:100vh;">');
                doc.write('<p style="padding:20px;color:#888;">Preview mode - modified code would render here</p>');
                doc.write('<pre style="padding:20px;font-size:12px;white-space:pre-wrap;">');
                doc.write(aiResponse ? aiResponse.modified_code.substring(0, 2000) : '');
                doc.write('</pre></div></body></html>');
                doc.close();
            }
        }
    };
    
    // ─── 事件处理 ─────────────────────────────────────────
    
    function onHover(e) {
        if (!isInspecting) return;
        const el = e.target;
        if (el === container || el === panel || el === toggleBtn || container.contains(el)) return;
        if (panel.contains(el)) return;
        
        const rect = el.getBoundingClientRect();
        highlight.style.display = 'block';
        highlight.style.left = rect.left + 'px';
        highlight.style.top = rect.top + 'px';
        highlight.style.width = rect.width + 'px';
        highlight.style.height = rect.height + 'px';
        
        // Tooltip
        const tag = el.tagName.toLowerCase();
        const id = el.id ? '#' + el.id : '';
        const classes = Array.from(el.classList).slice(0, 3).join('.');
        tooltip.textContent = tag + (classes ? '.' + classes : '') + id;
        tooltip.style.left = (e.clientX + 10) + 'px';
        tooltip.style.top = (e.clientY + 10) + 'px';
        tooltip.style.display = 'block';
        
        document.body.style.cursor = 'crosshair';
    }
    
    function onSingleClick(e) {
        if (!isInspecting) return;
        const el = e.target;
        if (el === toggleBtn || toggleBtn.contains(el)) return;
        e.preventDefault();
        e.stopPropagation();
    }
    
    function onElementClick(e) {
        if (!isInspecting) return;
        const el = e.target;
        if (el === panel || panel.contains(el) || el === container || el === toggleBtn || toggleBtn.contains(el)) return;
        
        e.preventDefault();
        e.stopPropagation();
        selectedElement = el;
        inspectElement(el);
    }
    
    function onRightClick(e) {
        if (!isInspecting) return;
        const el = e.target;
        if (el === panel || panel.contains(el) || el === container || el === toggleBtn || toggleBtn.contains(el)) return;
        
        e.preventDefault();
        e.stopPropagation();
        selectedElement = el;
        inspectElement(el);
    }
    
    function hideHighlight() {
        highlight.style.display = 'none';
        tooltip.style.display = 'none';
    }
    
    // ─── 元素检查 ─────────────────────────────────────────
    
    function inspectElement(el) {
        const tag = el.tagName.toLowerCase();
        const id = el.id || '';
        const classes = Array.from(el.classList);
        
        // 查找 data-iris-component
        let compEl = el.closest('[data-iris-component]');
        let componentPath = compEl ? compEl.getAttribute('data-iris-component') : null;
        
        // 如果没找到，尝试查找 vue 组件
        if (!componentPath) {
            componentPath = findVueComponent(el);
        }
        
        showLoading(true);
        panel.classList.add('open');
        
        // 发送到服务器查询源文件
        fetch('/api/element-source', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                tag: tag,
                id: id || null,
                classes: classes,
                component_path: componentPath
            })
        })
        .then(r => r.json())
        .then(data => {
            showLoading(false);
            if (data.error) {
                showError(data.error);
                return;
            }
            currentSource = data;
            displayFileInfo(tag, classes, id, data);
            displaySourceCode(data);
            resetAIState();
        })
        .catch(err => {
            showLoading(false);
            showError('Failed to fetch source: ' + err.message);
        });
    }
    
    function findVueComponent(el) {
        // 尝试从 Vue 内部结构获取组件信息
        try {
            // 查找 vnode 的 DOM 到组件映射
            let current = el;
            while (current) {
                // 检查 Vue devtools 数据
                if (current.__vueParentComponent) {
                    const instance = current.__vueParentComponent;
                    // Vue 3: instance.type.__file 包含源文件路径
                    if (instance.type && instance.type.__file) {
                        return instance.type.__file;
                    }
                    // 或者从 setup 作用域获取
                    if (instance.setupState && instance.type) {
                        const name = instance.type.name || instance.type.__name;
                        if (name) return name;
                    }
                }
                // 检查 __vue_component__
                if (current.__vue_component__) {
                    const comp = current.__vue_component__;
                    if (comp.__file) return comp.__file;
                }
                current = current.parentElement;
            }
        } catch (e) {
            // 忽略 Vue 内部访问错误
        }
        return null;
    }
    
    // ─── UI 更新 ─────────────────────────────────────────
    
    function displayFileInfo(tag, classes, id, data) {
        const info = document.getElementById('iris-io-file-info');
        let tagHtml = '<span class="iris-io-element-tag">&lt;' + tag + '&gt;</span>';
        if (classes.length > 0) {
            tagHtml += ' <span class="iris-io-element-class">.' + classes.slice(0,5).join('.') + '</span>';
        }
        if (id) {
            tagHtml += ' <span class="iris-io-element-id">#' + id + '</span>';
        }
        
        let pkgHtml = '';
        if (data.is_npm_package) {
            pkgHtml = '<div class="iris-io-package-badge">npm: ' + (data.package_name || '') + '@' + (data.package_version || '') + '</div>';
        }
        
        info.innerHTML = '' +
            '<div style="margin-bottom:4px;">' + tagHtml + '</div>' +
            '<div class="iris-io-file-path">' + (data.file_path || 'unknown') + '</div>' +
            pkgHtml;
    }
    
    function displaySourceCode(data) {
        const codeArea = document.getElementById('iris-io-code-area');
        const lines = (data.content || '').split('\n');
        const totalLines = lines.length;
        
        // 生成行号
        let lineNums = '';
        let lineCodes = '';
        const maxLen = String(totalLines).length;
        for (let i = 0; i < totalLines && i < 500; i++) {
            const lineNum = String(i + 1).padStart(maxLen, ' ');
            lineNums += lineNum + '\n';
            const line = escapeHtml(lines[i]);
            lineCodes += line + '\n';
        }
        
        const truncMsg = totalLines > 500 ? '<div style="padding:8px 20px;color:#888;font-size:12px;border-top:1px solid #222;">Showing first 500 lines of ' + totalLines + '</div>' : '';
        
        codeArea.innerHTML = '' +
            '<div class="line-numbers">' +
                '<div class="line-nums">' + lineNums + '</div>' +
                '<div class="line-code">' + lineCodes + '</div>' +
            '</div>' +
            truncMsg;
        
        // 重置 AI 区域
        document.getElementById('iris-io-diff-area').classList.remove('show');
        document.getElementById('iris-io-preview-container').classList.remove('show');
    }
    
    function showDiff(original, modified) {
        const diffArea = document.getElementById('iris-io-diff-area');
        const origLines = original.split('\n');
        const modLines = modified.split('\n');
        
        let html = '<div class="iris-io-diff-header">' +
            '<span>Original: ' + origLines.length + ' lines</span>' +
            '<span>Modified: ' + modLines.length + ' lines</span>' +
        '</div><div class="iris-io-diff-content">';
        
        // 简单行级 diff
        const maxLines = Math.max(origLines.length, modLines.length);
        for (let i = 0; i < maxLines && i < 300; i++) {
            const origLine = origLines[i] || '';
            const modLine = modLines[i] || '';
            if (origLine !== modLine) {
                if (origLine !== undefined) {
                    html += '<div class="iris-io-diff-removed">- ' + escapeHtml(origLine) + '</div>';
                }
                if (modLine !== undefined && modLine !== '') {
                    html += '<div class="iris-io-diff-added">+ ' + escapeHtml(modLine) + '</div>';
                }
            } else {
                html += '<div style="color:#666;">  ' + escapeHtml(origLine) + '</div>';
            }
        }
        if (maxLines >= 300) {
            html += '<div style="color:#888;padding-top:8px;">... diff truncated at 300 lines</div>';
        }
        
        html += '</div>';
        diffArea.innerHTML = html;
        diffArea.classList.add('show');
    }
    
    function resetAIState() {
        aiResponse = null;
        document.getElementById('iris-io-ai-input').value = '';
        document.getElementById('iris-io-apply-btn').style.display = 'none';
        document.getElementById('iris-io-preview-btn').style.display = 'none';
        document.getElementById('iris-io-status').textContent = '';
        document.getElementById('iris-io-status').className = 'iris-io-status';
    }
    
    function showLoading(show) {
        const loadingEl = document.getElementById('iris-io-loading');
        loadingEl.style.display = show ? 'flex' : 'none';
    }
    
    function showError(msg) {
        const codeArea = document.getElementById('iris-io-code-area');
        codeArea.innerHTML = '<div style="padding:20px;color:#ef5350;font-size:14px;">Error: ' + escapeHtml(msg) + '</div>';
    }
    
    function escapeHtml(str) {
        if (!str) return '';
        return str
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
    }
    
    // ─── 自动挂载 Vue 组件标注 ─────────────────────────
    
    function annotateComponents() {
        try {
            if (window.__iris_app) {
                const app = window.__iris_app;
                annotateInstance(app._instance, '');
            }
        } catch (e) {
            console.warn('[Iris IO] Annotate failed:', e);
        }
    }
    
    function annotateInstance(instance, parentPath) {
        if (!instance) return;
        try {
            const vnode = instance.vnode;
            const el = vnode.el;
            // 获取组件源文件路径
            let sourcePath = parentPath;
            if (instance.type && instance.type.__file) {
                sourcePath = instance.type.__file;
            } else if (instance.type && instance.type.__name) {
                sourcePath = instance.type.__name;
            }
            
            if (el && el.nodeType === 1 && sourcePath) {
                el.setAttribute('data-iris-component', sourcePath);
            }
            
            // 递归子组件
            const subTree = instance.subTree;
            if (subTree && subTree.children) {
                const children = Array.isArray(subTree.children) ? subTree.children : [subTree.children];
                children.forEach(child => {
                    if (child && child.component) {
                        annotateInstance(child.component, sourcePath);
                    }
                });
            }
        } catch (e) {
            // 忽略遍历中的错误
        }
    }
    
    // ─── 初始化 ─────────────────────────────────────────
    
    function waitForApp() {
        if (window.__iris_app) {
            annotateComponents();
            init();
            return;
        }
        // 监听 Vue app 注册
        const origDefine = Object.defineProperty;
        try {
            Object.defineProperty = function(obj, prop, desc) {
                const result = origDefine.call(this, obj, prop, desc);
                if (prop === '__iris_app' && obj === window) {
                    setTimeout(annotateComponents, 100);
                    init();
                }
                return result;
            };
        } catch (e) { }
        
        // 轮询等待
        let attempts = 0;
        const timer = setInterval(function() {
            attempts++;
            if (window.__iris_app) {
                clearInterval(timer);
                setTimeout(function() {
                    annotateComponents();
                }, 200);
                init();
            } else if (attempts > 60) {
                clearInterval(timer);
                // 即使没有 Vue app 也初始化，让用户手动使用
                init();
            }
        }, 500);
    }
    
    // 键盘快捷键: Ctrl+Shift+I 切换 Inspector
    document.addEventListener('keydown', function(e) {
        if (e.ctrlKey && e.shiftKey && (e.key === 'I' || e.key === 'i')) {
            e.preventDefault();
            if (window.irisIO) {
                window.irisIO.toggleInspector();
            }
        }
    });
    
    // 启动
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', waitForApp);
    } else {
        waitForApp();
    }
})();
"##;

/// 注入 Inspector Overlay 到 HTML 中
pub fn inject_inspector_overlay(html: &str) -> String {
    let overlay_html = format!(
        r##"
<style type="text/css">{}</style>
<script>{}</script>
"##,
        INSPECTOR_OVERLAY_CSS,
        INSPECTOR_OVERLAY_JS,
    );
    
    // 在 </body> 前注入
    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + overlay_html.len());
        result.push_str(&html[..pos]);
        result.push_str(&overlay_html);
        result.push_str(&html[pos..]);
        result
    } else {
        // 没有 </body>，追加到最后
        format!("{}\n{}", html, overlay_html)
    }
}
