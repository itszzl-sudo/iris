//! HTTP 服务器核心

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use colored::Colorize;
use std::net::SocketAddr;
use tracing::{info, warn};
use crate::server::routes;
use crate::server::ai_inspector;
use crate::server::compiler_cache::CompilerCache;
use crate::server::hmr::HMRManager;
use crate::utils;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 启动开发服务器
pub async fn start(root: String, port: u16, open: bool, enable_hmr: bool, debug: bool) -> Result<()> {
    // 初始化日志
    use tracing::Level;
    let log_level = if debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    // 显示 Banner
    println!("{}", "🦀 Iris JetCrab CLI".bright_cyan().bold());
    println!("{}", "Vue Development Server (Runtime On-Demand Compilation)".bright_black());
    println!();

    // 找到项目根目录
    let project_root = utils::find_project_root(std::path::Path::new(&root))?;
    println!("{} {}", "📁 Project:".bright_blue(), project_root.display().to_string().bright_white());

    // 检测 Vue 项目
    if !utils::is_vue_project(&project_root) {
        println!("{}", "❌ Error: Not a Vue project".bright_red().bold());
        return Ok(());
    }
    println!("{} {}", "✅ Vue:".bright_green(), "Project detected".bright_white());

    // 创建 HMR 管理器
    let mut hmr_manager = HMRManager::new(project_root.clone(), enable_hmr);
    let ws_manager = hmr_manager.ws_manager();
    
    // 创建编译器缓存（按需编译的核心）
    let cache = Arc::new(Mutex::new(
        CompilerCache::new(project_root.clone())
            .with_ws_manager(ws_manager.clone())
    ));
    
    // 启动文件监听
    if enable_hmr {
        hmr_manager.start_watching(cache.clone()).await?;
    }

    println!();

    // 构建路由
    let app = Router::new()
        // 主页
        .route("/", get(routes::index_handler))
        // 源文件编译（/src/*path）- 返回可执行的 JavaScript 模块
        .route("/src/*path", get(routes::source_file_handler))
        // Vue 模块按需编译（API 接口）
        .route("/@vue/*path", get(routes::vue_module_handler))
        // npm 包服务（/@npm/*path）
        .route("/@npm/*path", get(routes::npm_package_handler))
        // 静态资源
        .route("/assets/*path", get(routes::static_handler))
        // 项目信息 API
        .route("/api/project-info", get(routes::project_info_handler))
        // 依赖问题扫描 API
        .route("/api/dependency-issues", get(routes::dependency_issues_handler))
        // 依赖问题解决 API
        .route("/api/resolve-dependencies", post(routes::resolve_dependencies_handler))
        // 依赖问题解决页面
        .route("/resolve.html", get(routes::resolve_page_handler))
        // HMR WebSocket
        .route("/@hmr", get(routes::hmr_handler))
        // AI Inspector API
        .route("/api/element-source", post(ai_inspector::element_source_handler))
        .route("/api/file-content", get(ai_inspector::file_content_handler))
        .route("/api/ai-edit", post(ai_inspector::ai_edit_handler))
        .route("/api/apply-edit", post(ai_inspector::apply_edit_handler))
        .route("/api/npm-package-info", get(ai_inspector::npm_package_info_handler))
        // 通用文件服务 fallback（favicon.ico、图片占位等）
        .fallback(routes::fallback_handler)
        .with_state((cache, enable_hmr, ws_manager));

    // 添加 CORS
    let app = app.layer(CorsLayer::permissive());

    // 启动服务器
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("{} {}", "🌐 Server:".bright_blue(), format!("http://localhost:{}", port).bright_white().bold());
    
    if open {
        println!("{} {}", "🔗 Opening:".bright_blue(), "Browser".bright_white());
        if let Err(e) = open::that(format!("http://localhost:{}", port)) {
            warn!("Failed to open browser: {}", e);
        }
    }

    println!();
    println!("{}", "✨ Ready!".bright_green().bold());
    println!("  Compilation: On-demand (runtime)");
    println!("  HMR: {}", if enable_hmr { "Enabled".bright_green().to_string() } else { "Disabled".bright_yellow().to_string() });
    if enable_hmr {
        println!("  Watching: src/");
    }
    println!("  Press Ctrl+C to stop");
    println!();

    info!("Starting HTTP server on {}", addr);

    // 启动 HTTP 服务器
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
