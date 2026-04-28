//! Iris Runtime - Vue 3 开发服务器 WASM 接口
//!
//! 提供基于 WebAssembly 的 Vue SFC 编译和热重载功能。
//!
//! # 示例
//!
//! ```javascript
//! import { IrisRuntime } from 'iris-runtime';
//!
//! const runtime = new IrisRuntime();
//! const compiled = runtime.compile_sfc(`
//!   <template>
//!     <h1>{{ message }}</h1>
//!   </template>
//!   <script>
//!     export default {
//!       data() { return { message: 'Hello!' } }
//!     }
//!   </script>
//! `, 'App.vue');
//! ```

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::debug;

mod compiler;
mod hmr;

/// Iris 运行时核心
///
/// 提供 Vue SFC 编译、模块解析和热更新功能。
#[wasm_bindgen]
pub struct IrisRuntime {
    /// 编译缓存
    compiled_modules: HashMap<String, CompiledModule>,
    /// 是否启用调试模式
    debug: bool,
}

#[wasm_bindgen]
impl IrisRuntime {
    /// 创建新的 IrisRuntime 实例
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // 初始化日志
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        Self {
            compiled_modules: HashMap::new(),
            debug: false,
        }
    }

    /// 设置调试模式
    #[wasm_bindgen(js_name = setDebug)]
    pub fn set_debug(&mut self, enabled: bool) {
        self.debug = enabled;
        debug!("Debug mode: {}", enabled);
    }

    /// 编译 Vue SFC 文件
    ///
    /// # 参数
    ///
    /// * `source` - Vue SFC 源码
    /// * `filename` - 文件名（用于 sourcemap 和错误提示）
    ///
    /// # 返回
    ///
    /// JSON 格式的编译结果：
    /// ```json
    /// {
    ///   "script": "export default { ... }",
    ///   "styles": [{"code": "...", "scoped": true}],
    ///   "deps": ["./components/Foo.vue"]
    /// }
    /// ```
    #[wasm_bindgen(js_name = compileSfc)]
    pub fn compile_sfc(&mut self, source: &str, filename: &str) -> Result<String, JsError> {
        debug!("Compiling SFC: {}", filename);

        let compiled = compiler::compile_sfc(source, filename)
            .map_err(|e| JsError::new(&format!("Compilation failed: {}", e)))?;

        // 缓存编译结果
        self.compiled_modules.insert(filename.to_string(), compiled.clone());

        serde_json::to_string(&compiled)
            .map_err(|e| JsError::new(&format!("Serialization failed: {}", e)))
    }

    /// 解析模块导入路径
    ///
    /// # 参数
    ///
    /// * `import_path` - 导入路径（例如：'./components/Foo.vue'）
    /// * `importer` - 导入者路径
    ///
    /// # 返回
    ///
    /// 解析后的绝对路径
    #[wasm_bindgen(js_name = resolveImport)]
    pub fn resolve_import(&self, import_path: &str, importer: &str) -> Result<String, JsError> {
        debug!("Resolving import: {} from {}", import_path, importer);

        compiler::resolve_module(import_path, importer)
            .map_err(|e| JsError::new(&format!("Module resolution failed: {}", e)))
    }

    /// 生成热更新补丁
    ///
    /// # 参数
    ///
    /// * `old_source` - 旧源码
    /// * `new_source` - 新源码
    /// * `filename` - 文件名
    ///
    /// # 返回
    ///
    /// JSON 格式的 HMR patch：
    /// ```json
    /// {
    ///   "type": "vue-reload",
    ///   "path": "App.vue",
    ///   "timestamp": 1234567890,
    ///   "changes": [...]
    /// }
    /// ```
    #[wasm_bindgen(js_name = generateHmrPatch)]
    pub fn generate_hmr_patch(
        &mut self,
        old_source: &str,
        new_source: &str,
        filename: &str,
    ) -> Result<String, JsError> {
        debug!("Generating HMR patch for: {}", filename);

        let patch = hmr::generate_patch(old_source, new_source, filename)
            .map_err(|e| JsError::new(&format!("HMR patch generation failed: {}", e)))?;

        serde_json::to_string(&patch)
            .map_err(|e| JsError::new(&format!("Serialization failed: {}", e)))
    }

    /// 获取已编译模块的信息
    #[wasm_bindgen(js_name = getCompiledModule)]
    pub fn get_compiled_module(&self, filename: &str) -> Result<String, JsError> {
        self.compiled_modules
            .get(filename)
            .ok_or_else(|| JsError::new(&format!("Module not found: {}", filename)))
            .and_then(|module| {
                serde_json::to_string(module)
                    .map_err(|e| JsError::new(&format!("Serialization failed: {}", e)))
            })
    }

    /// 清除编译缓存
    #[wasm_bindgen(js_name = clearCache)]
    pub fn clear_cache(&mut self) {
        debug!("Clearing compilation cache");
        self.compiled_modules.clear();
    }

    /// 获取编译缓存大小
    #[wasm_bindgen(js_name = getCacheSize)]
    pub fn get_cache_size(&self) -> usize {
        self.compiled_modules.len()
    }

    /// 获取版本信息
    #[wasm_bindgen]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

impl Default for IrisRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// 编译后的模块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledModule {
    /// 转换后的 JavaScript 代码
    pub script: String,
    /// 样式块列表
    pub styles: Vec<StyleBlock>,
    /// 依赖列表
    pub deps: Vec<String>,
}

/// 样式块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleBlock {
    /// CSS 代码
    pub code: String,
    /// 是否启用作用域
    pub scoped: bool,
}
