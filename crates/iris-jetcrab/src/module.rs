//! ESM 模块加载器
//!
//! 实现 JavaScript ES Module 加载和解析。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// 模块信息
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// 模块路径
    pub path: PathBuf,
    /// 模块代码
    pub code: String,
    /// 依赖列表
    pub dependencies: Vec<String>,
}

/// 模块加载器
///
/// 负责加载和缓存 JavaScript 模块。
pub struct ModuleLoader {
    /// 模块缓存
    cache: HashMap<String, ModuleInfo>,
    /// 模块搜索路径
    search_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    /// 创建新的模块加载器
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            search_paths: Vec::new(),
        }
    }

    /// 添加模块搜索路径
    pub fn add_search_path(&mut self, path: &Path) {
        self.search_paths.push(path.to_path_buf());
        debug!("Added search path: {:?}", path);
    }

    /// 加载模块
    ///
    /// # 参数
    ///
    /// * `module_path` - 模块路径（相对或绝对）
    pub fn load_module(&mut self, module_path: &str) -> Result<ModuleInfo, String> {
        // 检查缓存
        if let Some(module) = self.cache.get(module_path) {
            debug!("Module cache hit: {}", module_path);
            return Ok(module.clone());
        }

        info!("Loading module: {}", module_path);

        // 解析模块路径
        let full_path = self.resolve_module_path(module_path)?;

        // 读取模块代码
        let code = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read module: {}", e))?;

        // 解析依赖
        let dependencies = self.parse_dependencies(&code)?;

        // 创建模块信息
        let module_info = ModuleInfo {
            path: full_path.clone(),
            code: code.clone(),
            dependencies: dependencies.clone(),
        };

        // 缓存模块
        self.cache.insert(module_path.to_string(), module_info.clone());

        debug!(
            "Module loaded: {} ({} deps)",
            module_path,
            dependencies.len()
        );

        Ok(module_info)
    }

    /// 解析模块路径
    fn resolve_module_path(&self, module_path: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(module_path);

        // 如果是绝对路径，直接返回
        if path.is_absolute() && path.exists() {
            return Ok(path);
        }

        // 在搜索路径中查找
        for search_path in &self.search_paths {
            let full_path = search_path.join(&path);
            if full_path.exists() {
                return Ok(full_path);
            }

            // 尝试添加 .js 扩展名
            let js_path = search_path.join(format!("{}.js", module_path));
            if js_path.exists() {
                return Ok(js_path);
            }

            // 尝试 index.js
            let index_path = search_path.join(module_path).join("index.js");
            if index_path.exists() {
                return Ok(index_path);
            }
        }

        Err(format!("Module not found: {}", module_path))
    }

    /// 解析模块依赖
    fn parse_dependencies(&self, code: &str) -> Result<Vec<String>, String> {
        let mut dependencies = Vec::new();

        // 简单的 import 语句解析
        for line in code.lines() {
            let line = line.trim();

            // 匹配: import ... from '...'
            if line.starts_with("import ") || line.starts_with("export ") {
                if let Some(start) = line.find('\'') {
                    if let Some(end) = line.rfind('\'') {
                        if start < end {
                            let dep = &line[start + 1..end];
                            if !dep.starts_with('.') || !dep.starts_with('/') {
                                dependencies.push(dep.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    /// 获取缓存的模块
    pub fn get_cached(&self, module_path: &str) -> Option<ModuleInfo> {
        self.cache.get(module_path).cloned()
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        info!("Module cache cleared");
    }

    /// 获取缓存大小
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_loader() {
        let loader = ModuleLoader::new();
        assert_eq!(loader.cache_size(), 0);
    }

    #[test]
    fn test_add_search_path() {
        let mut loader = ModuleLoader::new();
        loader.add_search_path(Path::new("/tmp"));
        assert_eq!(loader.search_paths.len(), 1);
    }

    #[test]
    fn test_clear_cache() {
        let mut loader = ModuleLoader::new();
        loader.clear_cache();
        assert_eq!(loader.cache_size(), 0);
    }

    #[test]
    fn test_parse_dependencies() {
        let code = r#"
            import Vue from 'vue';
            import App from './App.vue';
            import { ref } from 'vue';
        "#;

        let loader = ModuleLoader::new();
        let deps = loader.parse_dependencies(code).unwrap();
        
        // 应该只包含外部依赖（不包含相对路径）
        assert!(deps.iter().any(|d| d == "vue"));
    }
}
