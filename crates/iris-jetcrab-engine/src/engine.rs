//! JetCrab Engine 核心编排器
//!
//! 负责协调 Vue 项目的加载、编译、执行和渲染

use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use tracing::{info, debug, warn, error};

use crate::ProjectScanner;
use crate::ProjectInfo;
use crate::ModuleGraph;
use crate::HMRManager;

/// 引擎配置
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Vue 项目根目录
    pub project_root: PathBuf,
    /// 是否启用 HMR
    pub hmr_enabled: bool,
    /// 是否启用调试模式
    pub debug: bool,
    /// 忽略的文件/目录
    pub ignore_patterns: Vec<String>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            hmr_enabled: true,
            debug: false,
            ignore_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                ".DS_Store".to_string(),
            ],
        }
    }
}

/// JetCrab Engine 主结构体
///
/// 提供完整的 Vue 项目运行时编排能力
pub struct JetCrabEngine {
    /// 引擎配置
    config: EngineConfig,
    /// 项目信息
    project_info: Option<ProjectInfo>,
    /// 模块依赖图
    module_graph: ModuleGraph,
    /// HMR 管理器
    hmr_manager: Option<HMRManager>,
    /// 是否已初始化
    initialized: bool,
}

impl JetCrabEngine {
    /// 创建新的引擎实例
    pub fn new() -> Self {
        Self {
            config: EngineConfig::default(),
            project_info: None,
            module_graph: ModuleGraph::new(),
            hmr_manager: None,
            initialized: false,
        }
    }

    /// 使用自定义配置创建引擎实例
    pub fn with_config(config: EngineConfig) -> Self {
        Self {
            config,
            project_info: None,
            module_graph: ModuleGraph::new(),
            hmr_manager: None,
            initialized: false,
        }
    }

    /// 初始化引擎
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            warn!("Engine already initialized");
            return Ok(());
        }

        info!("Initializing JetCrab Engine...");

        // 1. 初始化日志系统
        if self.config.debug {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .init();
        }

        // 2. 初始化共享核心层
        iris_core::init();
        iris_gpu::init();
        iris_layout::init();
        iris_dom::init();
        iris_sfc::init();

        // 3. 初始化 JetCrab 运行时
        iris_jetcrab::init();

        // 4. 初始化 HMR（如果启用）
        if self.config.hmr_enabled {
            self.hmr_manager = Some(HMRManager::new());
            info!("HMR enabled");
        }

        self.initialized = true;
        info!("JetCrab Engine initialized successfully");

        Ok(())
    }

    /// 加载 Vue 项目
    pub async fn load_project<P: AsRef<Path>>(&mut self, project_path: P) -> Result<()> {
        if !self.initialized {
            error!("Engine not initialized. Call initialize() first.");
            anyhow::bail!("Engine not initialized");
        }

        let project_path = project_path.as_ref();
        info!("Loading Vue project from: {:?}", project_path);

        // 1. 扫描项目目录
        let scanner = ProjectScanner::new(project_path.to_path_buf());
        self.project_info = Some(scanner.scan()
            .context("Failed to scan project directory")?);

        debug!("Project info: {:?}", self.project_info);

        // 2. 解析 index.html
        let project_info = self.project_info.as_ref().unwrap();
        self.parse_index_html(&project_info.index_html_path)?;

        // 3. 构建模块依赖图
        self.build_module_graph().await?;

        info!("Vue project loaded successfully");

        Ok(())
    }

    /// 解析 index.html 文件
    fn parse_index_html(&self, index_path: &Path) -> Result<()> {
        info!("Parsing index.html: {:?}", index_path);

        let content = std::fs::read_to_string(index_path)
            .context("Failed to read index.html")?;

        // TODO: 使用 html5ever 解析 HTML
        // 提取 <script> 标签的 src 属性
        // 识别入口文件（通常是 /src/main.js 或 /src/main.ts）

        debug!("index.html content length: {} bytes", content.len());

        Ok(())
    }

    /// 构建模块依赖图
    async fn build_module_graph(&mut self) -> Result<()> {
        info!("Building module dependency graph...");

        let project_info = self.project_info.as_ref().unwrap();
        let src_dir = &project_info.src_dir;

        // 1. 扫描所有 .vue 和 .js/.ts 文件
        let files = self.scan_source_files(src_dir)?;
        debug!("Found {} source files", files.len());

        // 2. 解析每个文件的依赖
        for file_path in &files {
            self.parse_file_dependencies(file_path).await?;
        }

        // 3. 检测循环依赖
        if let Some(cycles) = self.module_graph.detect_cycles() {
            warn!("Circular dependencies detected: {:?}", cycles);
        }

        info!("Module graph built with {} modules", self.module_graph.len());

        Ok(())
    }

    /// 扫描源文件
    fn scan_source_files(&self, src_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if !src_dir.exists() {
            warn!("Source directory does not exist: {:?}", src_dir);
            return Ok(files);
        }

        for entry in walkdir::WalkDir::new(src_dir)
            .into_iter()
            .filter_entry(|e| !self.should_ignore(e))
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str());
                match ext {
                    Some("vue") | Some("js") | Some("ts") | Some("jsx") | Some("tsx") => {
                        files.push(path.to_path_buf());
                    }
                    _ => {}
                }
            }
        }

        Ok(files)
    }

    /// 检查是否应该忽略该文件/目录
    fn should_ignore(&self, entry: &walkdir::DirEntry) -> bool {
        let name = entry.file_name().to_string_lossy();
        self.config.ignore_patterns.iter().any(|pattern| {
            name.contains(pattern.as_str())
        })
    }

    /// 解析文件依赖
    async fn parse_file_dependencies(&mut self, file_path: &Path) -> Result<()> {
        debug!("Parsing dependencies in: {:?}", file_path);

        let content = std::fs::read_to_string(file_path)
            .context("Failed to read file")?;

        let ext = file_path.extension().and_then(|e| e.to_str());

        match ext {
            Some("vue") => {
                // 编译 Vue SFC
                let compiled = iris_sfc::compile_from_string(
                    &file_path.to_string_lossy(),
                    &content
                )?;

                // 添加到模块图（SFCModule 没有 dependencies 字段，使用空列表）
                self.module_graph.add_module(
                    file_path.to_string_lossy().to_string(),
                    vec![], // TODO: 从 compiled 中提取依赖
                );
            }
            Some("js") | Some("ts") | Some("jsx") | Some("tsx") => {
                // 解析 JavaScript/TypeScript 依赖
                let dependencies = self.parse_js_dependencies(&content);
                
                self.module_graph.add_module(
                    file_path.to_string_lossy().to_string(),
                    dependencies,
                );
            }
            _ => {}
        }

        Ok(())
    }

    /// 解析 JavaScript/TypeScript 文件的依赖
    fn parse_js_dependencies(&self, content: &str) -> Vec<String> {
        let mut dependencies = Vec::new();

        // 简单的 import 解析（可以使用更完善的 AST 解析）
        for line in content.lines() {
            let line = line.trim();
            
            // 匹配 import ... from '...'
            if line.starts_with("import ") && line.contains(" from ") {
                if let Some(start) = line.find('\'') {
                    if let Some(end) = line[start+1..].find('\'') {
                        let dep = &line[start+1..start+1+end];
                        dependencies.push(dep.to_string());
                    }
                }
            }
            
            // 匹配 require('...')
            if line.contains("require(") {
                if let Some(start) = line.find('\'') {
                    if let Some(end) = line[start+1..].find('\'') {
                        let dep = &line[start+1..start+1+end];
                        dependencies.push(dep.to_string());
                    }
                }
            }
        }

        dependencies
    }

    /// 运行 Vue 应用
    pub async fn run(&mut self) -> Result<()> {
        if !self.initialized {
            error!("Engine not initialized");
            anyhow::bail!("Engine not initialized");
        }

        if self.project_info.is_none() {
            error!("No project loaded. Call load_project() first.");
            anyhow::bail!("No project loaded");
        }

        info!("Starting Vue application...");

        // 1. 获取入口文件
        let entry_file = &self.project_info.as_ref().unwrap().entry_file;
        info!("Entry file: {:?}", entry_file);

        // 2. 创建 JetCrab 运行时
        let mut runtime = iris_jetcrab::JetCrabRuntime::new();
        runtime.init().map_err(|e| anyhow::anyhow!("Failed to init runtime: {}", e))?;

        // 3. 加载并执行入口文件
        let entry_content = std::fs::read_to_string(entry_file)
            .context("Failed to read entry file")?;

        runtime.eval(&entry_content).map_err(|e| anyhow::anyhow!("Failed to eval entry file: {}", e))?;

        // 4. 启动渲染循环（使用 iris-gpu）
        // TODO: 实现渲染循环

        info!("Vue application started successfully");

        Ok(())
    }

    /// 设置项目根目录
    pub fn set_project_root<P: AsRef<Path>>(&mut self, path: P) {
        self.config.project_root = path.as_ref().to_path_buf();
    }

    /// 启用/禁用 HMR
    pub fn enable_hmr(&mut self, enabled: bool) {
        self.config.hmr_enabled = enabled;
    }

    /// 获取项目信息
    pub fn project_info(&self) -> Option<&ProjectInfo> {
        self.project_info.as_ref()
    }

    /// 获取模块依赖图
    pub fn module_graph(&self) -> &ModuleGraph {
        &self.module_graph
    }
}

impl Default for JetCrabEngine {
    fn default() -> Self {
        Self::new()
    }
}
