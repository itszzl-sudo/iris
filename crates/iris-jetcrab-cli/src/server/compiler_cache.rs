//! 编译器缓存管理
//!
//! 负责按需编译 Vue 模块并缓存结果
//!
//! 架构说明：
//! - iris-jetcrab-engine 提供完整的编译能力
//! - cli 侧管理编译缓存，实现真正的按需编译
//! - 每个模块在首次请求时才编译自身，不编译无关模块
//! - 浏览器端原生 ESM 负责按需加载每个依赖模块

use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::Mutex;
use iris_jetcrab_engine::vue_compiler::VueProjectCompiler;
use iris_jetcrab_engine::sfc_compiler::CompiledModule;
use anyhow::Result;
use tracing::{info, debug};

/// 编译器缓存
pub struct CompilerCache {
    /// 项目根目录
    pub project_root: PathBuf,
    /// 已编译的模块缓存 (key: 请求路径, value: 编译结果)
    compiled_modules: Mutex<HashMap<String, CompiledModule>>,
}

impl CompilerCache {
    /// 创建新的缓存实例
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            compiled_modules: Mutex::new(HashMap::new()),
        }
    }

    /// 设置 WebSocket 管理器（兼容接口）
    pub fn with_ws_manager(self, _ws_manager: std::sync::Arc<crate::server::hmr::WebSocketManager>) -> Self {
        self
    }

    /// 获取或编译模块（真正的按需编译）
    ///
    /// 缓存未命中时，只编译请求的单个文件，不编译整个项目
    pub async fn get_or_compile(&self, module_path: &str) -> Result<CompiledModule> {
        // 1. 检查缓存
        {
            let cache = self.compiled_modules.lock().await;
            if let Some(module) = cache.get(module_path) {
                debug!("Cache hit for module: {}", module_path);
                return Ok(module.clone());
            }
        }

        debug!("Cache miss, compiling module on-demand: {}", module_path);

        // 2. 解析文件路径
        let file_path = self.resolve_module_path(module_path)?;

        // 3. 按需编译单个文件
        let mut compiler = VueProjectCompiler::new(self.project_root.clone());
        let compiled = compiler.compile_file(&file_path.to_string_lossy())?;

        debug!(
            "Module compiled: {} (script: {} bytes, styles: {})",
            module_path,
            compiled.script.len(),
            compiled.styles.len()
        );

        // 4. 缓存并返回
        let mut cache = self.compiled_modules.lock().await;
        cache.insert(module_path.to_string(), compiled.clone());

        Ok(compiled)
    }

    /// 解析模块路径为文件系统上的绝对路径
    fn resolve_module_path(&self, module_path: &str) -> Result<PathBuf> {
        // 标准化请求路径（移除前缀斜杠）
        let normalized = module_path.trim_start_matches('/');

        // 如果已经是绝对路径，直接使用
        let path = std::path::Path::new(normalized);
        if path.is_absolute() && path.exists() {
            return Ok(path.to_path_buf());
        }

        // 相对于项目 src 目录查找
        let src_path = self.project_root.join("src").join(normalized);
        if src_path.exists() {
            return Ok(src_path);
        }

        // 相对于项目根目录查找
        let root_path = self.project_root.join(normalized);
        if root_path.exists() {
            return Ok(root_path);
        }

        // 尝试添加常见扩展名
        if !normalized.contains('.') {
            let extensions = [".ts", ".tsx", ".vue", ".js", ".jsx", ".mjs"];
            for ext in &extensions {
                let try_src = self.project_root.join("src").join(format!("{}{}", normalized, ext));
                if try_src.exists() {
                    return Ok(try_src);
                }
                let try_root = self.project_root.join(format!("{}{}", normalized, ext));
                if try_root.exists() {
                    return Ok(try_root);
                }
            }
        }

        // 目录索引文件解析（如 import from './components' → ./components/index.ts）
        let candidate_dir = self.project_root.join("src").join(normalized);
        if candidate_dir.is_dir() {
            let index_files = ["index.ts", "index.js", "index.tsx", "index.jsx", "index.mjs"];
            for index_file in &index_files {
                let index_path = candidate_dir.join(index_file);
                if index_path.exists() {
                    return Ok(index_path);
                }
            }
        }

        Err(anyhow::anyhow!("Module not found: {}", module_path))
    }

    /// 清除指定模块的缓存
    pub async fn invalidate(&self, module_path: &str) {
        let mut cache = self.compiled_modules.lock().await;
        if cache.remove(module_path).is_some() {
            debug!("Invalidated cached module: {}", module_path);
        } else {
            debug!("Module not in cache (no-op): {}", module_path);
        }
    }

    /// 批量使缓存失效：根据变更的文件系统路径移除对应的已缓存模块
    ///
    /// 遍历变更的文件路径，推导出对应的缓存 key 并移除
    /// 返回已移除的模块数量
    pub async fn invalidate_modules(&self, changed_paths: &[PathBuf]) -> usize {
        let mut cache = self.compiled_modules.lock().await;
        let mut removed = 0usize;

        for changed_path in changed_paths {
            // 从变更的文件系统路径推导缓存 key
            // 文件路径: <project_root>/src/App.vue
            // 缓存 key: /src/App.vue
            if let Ok(relative) = changed_path.strip_prefix(&self.project_root) {
                let relative_str = relative.to_string_lossy().replace('\\', "/");
                let cache_key = format!("/{}", relative_str);
                if cache.remove(&cache_key).is_some() {
                    removed += 1;
                    debug!("HMR invalidated module: {}", cache_key);
                }

                // 也尝试不带扩展名的 key（某些 import 路径不带扩展名）
                if let Some(stem) = changed_path.file_stem() {
                    if let Some(parent) = relative.parent() {
                        let parent_str = parent.to_string_lossy().replace('\\', "/");
                        let key_no_ext = format!("/{}/{}", parent_str, stem.to_string_lossy());
                        if key_no_ext != cache_key && cache.remove(&key_no_ext).is_some() {
                            removed += 1;
                            debug!("HMR invalidated module (no-ext): {}", key_no_ext);
                        }
                    }
                }
            }
        }

        removed
    }

    /// 获取缓存模块数量
    pub async fn cached_count(&self) -> usize {
        self.compiled_modules.lock().await.len()
    }

    /// 清除所有缓存（用于 HMR 完整刷新）
    /// 浏览器会重新请求各模块，触发按需编译
    pub async fn rebuild(&self) -> Result<()> {
        info!("Clearing compiled module cache (on-demand rebuild)");
        self.compiled_modules.lock().await.clear();
        Ok(())
    }

    /// 获取缓存统计
    pub async fn stats(&self) -> (usize, bool) {
        let count = self.compiled_modules.lock().await.len();
        (count, true)
    }

    /// 将项目相对路径转换为模块缓存 key
    ///
    /// 输入："src/styles/main.css" 或 "src/App.vue"
    /// 输出："/src/styles/main.css" 或 "/src/App.vue"
    pub fn get_module_path(&self, relative_path: &str) -> Result<String> {
        let path = relative_path.trim_start_matches('/');
        let normalized = path.replace('\\', "/");

        // 构建绝对路径检查文件是否存在
        let abs_path = self.project_root.join(&normalized);
        if !abs_path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", abs_path.display()));
        }

        // 返回缓存 key
        Ok(format!("/{}", normalized))
    }
}
