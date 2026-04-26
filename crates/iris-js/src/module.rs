//! ESM 模块系统
//!
//! 实现 ES Module 的解析、加载和执行。

use crate::vm::{JsRuntime, JsValue};
use rquickjs::Result;
use std::collections::HashMap;

/// 模块状态
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleStatus {
    /// 未加载
    Unloaded,
    /// 加载中
    Loading,
    /// 已加载
    Loaded,
    /// 加载失败
    Error(String),
}

/// ES 模块
#[derive(Debug)]
pub struct EsModule {
    /// 模块标识符（路径或 URL）
    pub specifier: String,
    /// 模块代码
    pub code: String,
    /// 模块状态
    pub status: ModuleStatus,
    /// 导出值
    pub exports: HashMap<String, JsValue>,
    /// 依赖的模块
    pub dependencies: Vec<String>,
}

impl EsModule {
    /// 创建新模块
    pub fn new(specifier: &str, code: &str) -> Self {
        Self {
            specifier: specifier.to_string(),
            code: code.to_string(),
            status: ModuleStatus::Unloaded,
            exports: HashMap::new(),
            dependencies: Vec::new(),
        }
    }
}

/// 模块注册表
///
/// 管理所有已加载的模块。
///
/// # 示例
///
/// ```rust
/// use iris_js::module::ModuleRegistry;
///
/// let mut registry = ModuleRegistry::new();
/// registry.register("myModule", "export const name = 'test';");
/// ```
pub struct ModuleRegistry {
    /// 已注册的模块
    modules: HashMap<String, EsModule>,
    /// 模块解析器（路径解析）
    resolver: ModuleResolver,
}

impl ModuleRegistry {
    /// 创建新的模块注册表
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            resolver: ModuleResolver::new(),
        }
    }

    /// 注册模块（从源代码）
    pub fn register(&mut self, specifier: &str, code: &str) {
        let module = EsModule::new(specifier, code);
        self.modules.insert(specifier.to_string(), module);
    }

    /// 注册模块（从文件）
    pub fn register_from_file(&mut self, specifier: &str, path: &str) -> std::io::Result<()> {
        let code = std::fs::read_to_string(path)?;
        self.register(specifier, &code);
        Ok(())
    }

    /// 获取模块
    pub fn get_module(&self, specifier: &str) -> Option<&EsModule> {
        self.modules.get(specifier)
    }

    /// 获取模块的可变引用
    pub fn get_module_mut(&mut self, specifier: &str) -> Option<&mut EsModule> {
        self.modules.get_mut(specifier)
    }

    /// 解析模块路径
    pub fn resolve(&self, specifier: &str, base: &str) -> String {
        self.resolver.resolve(specifier, base)
    }

    /// 加载并执行模块
    pub fn load_and_execute(
        &mut self,
        runtime: &mut JsRuntime,
        specifier: &str,
    ) -> Result<HashMap<String, JsValue>> {
        // 检查模块是否存在
        if !self.modules.contains_key(specifier) {
            return Err(rquickjs::Error::new_loading(specifier));
        }

        // 执行模块代码
        let module = self.modules.get_mut(specifier).unwrap();
        module.status = ModuleStatus::Loading;

        match runtime.eval(&module.code) {
            Ok(_) => {
                module.status = ModuleStatus::Loaded;
                Ok(module.exports.clone())
            }
            Err(e) => {
                module.status = ModuleStatus::Error(e.to_string());
                Err(e)
            }
        }
    }

    /// 清空所有模块
    pub fn clear(&mut self) {
        self.modules.clear();
    }

    /// 获取模块数量
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

/// 模块解析器
///
/// 负责解析模块路径，支持相对路径和绝对路径。
pub struct ModuleResolver {
    /// 基础路径
    base_path: String,
}

impl ModuleResolver {
    /// 创建新的模块解析器
    pub fn new() -> Self {
        Self {
            base_path: String::from("/"),
        }
    }

    /// 设置基础路径
    pub fn set_base_path(&mut self, path: &str) {
        self.base_path = path.to_string();
    }

    /// 解析模块路径
    ///
    /// 支持以下格式：
    /// - 相对路径: `./module.js`, `../module.js`
    /// - 绝对路径: `/module.js`
    /// - 包名: `vue`, `@vue/runtime-core`
    pub fn resolve(&self, specifier: &str, base: &str) -> String {
        if specifier.starts_with("./") || specifier.starts_with("../") {
            // 相对路径
            self.resolve_relative_path(specifier, base)
        } else if specifier.starts_with('/') {
            // 绝对路径
            specifier.to_string()
        } else {
            // 包名或内置模块
            specifier.to_string()
        }
    }

    /// 解析相对路径
    fn resolve_relative_path(&self, specifier: &str, base: &str) -> String {
        // 简化实现：直接拼接
        // 实际应该使用路径解析库
        if base.ends_with('/') {
            format!("{}{}", base, specifier)
        } else {
            format!("{}/{}", base, specifier)
        }
    }
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_module() {
        let module = EsModule::new("test.js", "export const a = 1;");
        assert_eq!(module.specifier, "test.js");
        assert_eq!(module.status, ModuleStatus::Unloaded);
    }

    #[test]
    fn test_register_module() {
        let mut registry = ModuleRegistry::new();
        registry.register("test.js", "export const a = 1;");
        assert_eq!(registry.module_count(), 1);
    }

    #[test]
    fn test_get_module() {
        let mut registry = ModuleRegistry::new();
        registry.register("test.js", "export const a = 1;");

        let module = registry.get_module("test.js");
        assert!(module.is_some());
        assert_eq!(module.unwrap().specifier, "test.js");
    }

    #[test]
    fn test_resolve_relative_path() {
        let resolver = ModuleResolver::new();

        assert_eq!(
            resolver.resolve("./module.js", "/src"),
            "/src/./module.js"
        );
        assert_eq!(
            resolver.resolve("../module.js", "/src/components"),
            "/src/components/../module.js"
        );
    }

    #[test]
    fn test_resolve_absolute_path() {
        let resolver = ModuleResolver::new();
        assert_eq!(resolver.resolve("/module.js", "/src"), "/module.js");
    }

    #[test]
    fn test_resolve_package_name() {
        let resolver = ModuleResolver::new();
        assert_eq!(resolver.resolve("vue", "/src"), "vue");
        assert_eq!(
            resolver.resolve("@vue/runtime-core", "/src"),
            "@vue/runtime-core"
        );
    }

    #[test]
    fn test_load_and_execute_module() {
        let mut registry = ModuleRegistry::new();
        registry.register("math.js", "const a = 1 + 2;");

        let mut runtime = JsRuntime::new();
        let result = registry.load_and_execute(&mut runtime, "math.js");
        assert!(result.is_ok());

        let module = registry.get_module("math.js").unwrap();
        assert_eq!(module.status, ModuleStatus::Loaded);
    }

    #[test]
    fn test_load_nonexistent_module() {
        let mut registry = ModuleRegistry::new();
        let mut runtime = JsRuntime::new();
        let result = registry.load_and_execute(&mut runtime, "nonexistent.js");
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_modules() {
        let mut registry = ModuleRegistry::new();
        registry.register("test1.js", "");
        registry.register("test2.js", "");
        assert_eq!(registry.module_count(), 2);

        registry.clear();
        assert_eq!(registry.module_count(), 0);
    }
}
