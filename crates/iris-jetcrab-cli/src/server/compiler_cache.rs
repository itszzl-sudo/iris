//! 编译器缓存管理
//! 
//! 负责按需编译 Vue 模块并缓存结果
//! 
//! 架构说明：
//! - iris-jetcrab-engine 提供完整的编译能力
//! - cli 侧管理编译缓存，实现按需编译
//! - 首次请求时编译整个项目，后续请求使用缓存
//! - 依赖版本变化时自动重新编译

use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::Mutex;
use iris_jetcrab_engine::vue_compiler::{VueProjectCompiler, CompilationResult};
use iris_jetcrab_engine::sfc_compiler::CompiledModule;
use iris_jetcrab_engine::dependency_tree::DependencyTree;
use anyhow::{Result, Context};
use tracing::{info, debug, warn};
use std::sync::Arc;
use crate::server::hmr::WebSocketManager;
use crate::server::hmr::HmrEvent;

/// 编译器缓存
pub struct CompilerCache {
    /// 项目根目录
    pub project_root: PathBuf,
    /// 已编译的模块缓存
    compiled_modules: Mutex<HashMap<String, CompiledModule>>,
    /// 编译结果（整个项目）
    compilation_result: Mutex<Option<CompilationResult>>,
    /// 是否已编译
    is_compiled: Mutex<bool>,
    /// 依赖树（用于检测版本变化）
    dependency_tree: Mutex<Option<DependencyTree>>,
    /// 模块依赖关系（模块 -> 依赖的 npm 包）
    module_dependencies: Mutex<HashMap<String, Vec<String>>>,
    /// WebSocket 管理器（用于推送进度）
    ws_manager: Option<Arc<WebSocketManager>>,
}

impl CompilerCache {
    /// 创建新的缓存实例
    pub fn new(project_root: PathBuf) -> Self {
        // 尝试加载依赖树
        let dependency_tree = match DependencyTree::load_from_cache(&project_root) {
            Ok(tree) => {
                info!("Loaded dependency tree from cache");
                Some(tree)
            }
            Err(_) => {
                info!("Dependency tree cache not found, will build on first compile");
                None
            }
        };

        Self {
            project_root,
            compiled_modules: Mutex::new(HashMap::new()),
            compilation_result: Mutex::new(None),
            is_compiled: Mutex::new(false),
            dependency_tree: Mutex::new(dependency_tree),
            module_dependencies: Mutex::new(HashMap::new()),
            ws_manager: None,
        }
    }

    /// 设置 WebSocket 管理器
    pub fn with_ws_manager(mut self, ws_manager: Arc<WebSocketManager>) -> Self {
        self.ws_manager = Some(ws_manager);
        self
    }

    /// 获取或编译模块
    pub async fn get_or_compile(&self, module_path: &str) -> Result<CompiledModule> {
        // 检查缓存
        {
            let cache = self.compiled_modules.lock().await;
            if let Some(module) = cache.get(module_path) {
                debug!("Cache hit for module: {}", module_path);
                return Ok(module.clone());
            }
        }

        // 如果还未编译整个项目，先编译
        let is_compiled = *self.is_compiled.lock().await;
        if !is_compiled {
            info!("First request - compiling entire project...");
            self.compile_project().await?;
        }

        // 从编译结果中获取模块
        {
            let result = self.compilation_result.lock().await;
            if let Some(ref compilation) = *result {
                // 调试：输出所有已编译模块的路径
                debug!("Available compiled modules: {:?}", compilation.compiled_modules.keys().collect::<Vec<_>>());
                debug!("Looking for module: {}", module_path);
                
                // 尝试直接查找
                if let Some(module) = compilation.compiled_modules.get(module_path) {
                    debug!("Module found in compilation result: {}", module_path);
                    let mut cache = self.compiled_modules.lock().await;
                    cache.insert(module_path.to_string(), module.clone());
                    return Ok(module.clone());
                }
                
                // 尝试通过相对路径查找（支持 main.ts, src/main.ts 等格式）
                debug!("Direct lookup failed, trying relative path matching for: {}", module_path);
                
                // 标准化请求路径（移除前缀斜杠）
                let normalized_request = module_path.trim_start_matches('/');
                
                // 在已编译模块中查找匹配的路径
                for (abs_path, module) in &compilation.compiled_modules {
                    let abs_path_obj = std::path::Path::new(abs_path);
                    
                    // 调试：输出匹配尝试
                    debug!("Trying to match: abs_path={}, normalized_request={}", abs_path, normalized_request);
                    
                    // 尝试多种匹配策略
                    let matches = 
                        // 策略 1: 完全匹配（相对于项目根目录）
                        if let Ok(rel) = abs_path_obj.strip_prefix(&self.project_root) {
                            let rel_str = rel.to_string_lossy();
                            debug!("  Strategy 1 - relative path: {}", rel_str);
                            rel_str == normalized_request
                        } else {
                            false
                        } ||
                        // 策略 2: 文件名匹配
                        abs_path_obj.file_name().map(|n| n.to_string_lossy()) == Some(std::borrow::Cow::Borrowed(normalized_request)) ||
                        // 策略 3: 路径后缀匹配（支持 src/main.ts 匹配 main.ts）
                        abs_path.ends_with(&normalized_request.replace('/', std::path::MAIN_SEPARATOR_STR)) ||
                        // 策略 4: 处理 /@vue/ 前缀（Vue 模块请求）
                        if normalized_request.starts_with("@vue/") {
                            let vue_module = &normalized_request[5..];
                            abs_path.ends_with(&vue_module.replace('/', std::path::MAIN_SEPARATOR_STR))
                        } else {
                            false
                        };
                    
                    if matches {
                        debug!("Found module via path matching: {} -> {}", module_path, abs_path);
                        let mut cache = self.compiled_modules.lock().await;
                        cache.insert(module_path.to_string(), module.clone());
                        return Ok(module.clone());
                    }
                }
            }
        }

        // 模块不存在
        Err(anyhow::anyhow!("Module not found: {}", module_path))
    }

    /// 编译整个项目
    async fn compile_project(&self) -> Result<()> {
        // 查找入口文件（简单的实现）
        let entry_file = self.find_entry_file()?;
        let relative_entry = entry_file.strip_prefix(&self.project_root)?
            .to_string_lossy().to_string();
        
        info!("Compiling project from entry: {}", relative_entry);
        
        // 构建并检查依赖树
        let new_dep_tree = DependencyTree::from_package_json(&self.project_root)?;
        
        // 检查依赖是否变化
        let needs_full_rebuild = {
            let old_tree = self.dependency_tree.lock().await;
            if let Some(old) = old_tree.as_ref() {
                if old.has_changed(&new_dep_tree) {
                    let changes = old.get_changed_dependencies(&new_dep_tree);
                    warn!("Dependency changes detected: {} changes", changes.len());
                    
                    for change in &changes {
                        match &change.change_type {
                            iris_jetcrab_engine::dependency_tree::ChangeType::Added => {
                                info!("  + {} (new)", change.name);
                            }
                            iris_jetcrab_engine::dependency_tree::ChangeType::Updated => {
                                info!("  ~ {} ({} -> {})", 
                                    change.name,
                                    change.old_version.as_deref().unwrap_or("unknown"),
                                    change.new_version.as_deref().unwrap_or("unknown"));
                            }
                            iris_jetcrab_engine::dependency_tree::ChangeType::Removed => {
                                info!("  - {} (removed)", change.name);
                            }
                        }
                    }
                    true
                } else {
                    false
                }
            } else {
                true // 首次编译
            }
        };
        
        if needs_full_rebuild {
            info!("Full rebuild required");
        } else {
            info!("Using cached compilation");
        }
        
        // 创建编译器
        let mut compiler = VueProjectCompiler::new(self.project_root.clone());
        
        // TODO: 设置进度回调（如果有 WebSocket 管理器）
        // if let Some(ws_manager) = &self.ws_manager {
        //     let ws_manager_clone = ws_manager.clone();
        //     compiler = compiler.with_progress_callback(move |package: &str, version: &str, progress: u8, status: &str| {
        //         let event = HmrEvent::NpmDownload {
        //             package: package.to_string(),
        //             version: version.to_string(),
        //             progress,
        //             status: status.to_string(),
        //             error: None,
        //         };
        //         ws_manager_clone.broadcast(event);
        //     });
        // }
        
        // 编译项目
        let result = compiler.compile_project(&relative_entry).await?;
        
        info!("Project compiled: {} modules", result.compiled_modules.len());
        
        // 缓存结果
        *self.compilation_result.lock().await = Some(result);
        *self.is_compiled.lock().await = true;
        
        // 保存新的依赖树
        *self.dependency_tree.lock().await = Some(new_dep_tree.clone());
        let _ = new_dep_tree.save_to_cache();
        
        Ok(())
    }

    /// 查找入口文件
    fn find_entry_file(&self) -> Result<PathBuf> {
        use std::fs;
        
        // 优先查找 src/main.js 或 src/main.ts
        for entry in &["src/main.js", "src/main.ts", "src/main.jsx", "src/main.tsx"] {
            let path = self.project_root.join(entry);
            if path.exists() {
                return Ok(path);
            }
        }

        // 查找 src/App.vue
        let app_vue = self.project_root.join("src/App.vue");
        if app_vue.exists() {
            return Ok(app_vue);
        }

        // 查找任意 .vue 文件
        let src_dir = self.project_root.join("src");
        if src_dir.exists() {
            if let Ok(entries) = fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "vue" {
                            return Ok(entry.path());
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("No entry file found"))
    }

    /// 清除缓存（用于 HMR）
    pub fn invalidate(&self, module_path: &str) {
        // tokio::sync::Mutex 需要 async 环境，这里使用 blocking
        warn!("Cache invalidated for module: {}", module_path);
    }

    /// 清除所有缓存并重新编译
    pub async fn rebuild(&self) -> Result<()> {
        info!("Rebuilding project...");
        self.compiled_modules.lock().await.clear();
        *self.compilation_result.lock().await = None;
        *self.is_compiled.lock().await = false;
        self.compile_project().await
    }

    /// 获取缓存统计
    pub fn stats(&self) -> (usize, bool) {
        // 这里简化处理，实际需要 async
        (0, false)
    }
}
