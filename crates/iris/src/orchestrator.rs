//! Iris Runtime Orchestrator
//!
//! 负责将 iris-sfc、iris-js、iris-dom、iris-layout 和 iris-gpu 连接在一起，
//! 形成完整的 Vue 3 运行时。
//!
//! # 架构流程
//!
//! ```text
//! 1. 初始化阶段
//!    → iris-js::init()
//!    → iris-dom::init()
//!
//! 2. 编译阶段
//!    iris-sfc::compile("App.vue")
//!    → 输出 JS 代码 + 样式
//!
//! 3. 执行阶段
//!    iris-js::eval(js_code)
//!    → Vue 创建组件实例
//!    → 生成虚拟 DOM
//!
//! 4. 渲染循环
//!    loop {
//!      处理事件 (iris-dom)
//!      更新虚拟 DOM (iris-js)
//!      GPU 渲染 (iris-gpu)
//!    }
//! ```

use iris_dom::vnode::VNode;
use iris_js::{
    module::ModuleRegistry,
    vue::setup_complete_vue_environment,
    vm::JsRuntime,
};
use iris_sfc::SfcModule;
use std::path::Path;
use tracing::{debug, info};

/// 运行时编排器
///
/// 管理整个 Iris 运行时的生命周期，协调各模块的初始化和交互。
pub struct RuntimeOrchestrator {
    /// JavaScript 运行时
    js_runtime: JsRuntime,
    /// 模块注册表
    module_registry: ModuleRegistry,
    /// 当前根虚拟 DOM 节点
    root_vnode: Option<VNode>,
    /// 是否已初始化
    initialized: bool,
}

impl RuntimeOrchestrator {
    /// 创建新的运行时编排器
    pub fn new() -> Self {
        Self {
            js_runtime: JsRuntime::new(),
            module_registry: ModuleRegistry::new(),
            root_vnode: None,
            initialized: false,
        }
    }

    /// 初始化运行时环境
    ///
    /// 按照正确的顺序初始化所有子模块。
    pub fn initialize(&mut self) -> Result<(), String> {
        info!("Initializing Iris runtime...");

        // 1. 初始化 JavaScript 运行时并注入 Vue
        debug!("Initializing JS runtime with Vue...");
        setup_complete_vue_environment(&mut self.js_runtime)
            .map_err(|e| format!("Failed to setup Vue environment: {}", e))?;

        // 2. 注入 BOM API
        debug!("Injecting BOM API...");
        self.js_runtime
            .inject_bom(1280, 720)
            .map_err(|e| format!("Failed to inject BOM: {}", e))?;

        self.initialized = true;
        info!("Iris runtime initialized successfully");
        Ok(())
    }

    /// 编译并加载 Vue SFC 组件
    ///
    /// 将 .vue 文件编译为 JavaScript，然后在 JS 运行时中执行。
    pub fn load_vue_app<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        if !self.initialized {
            return Err("Runtime not initialized. Call initialize() first.".to_string());
        }

        let path = path.as_ref();
        info!(path = ?path, "Loading Vue application...");

        // 1. 编译 SFC
        let sfc_module = self.compile_sfc(path)?;
        info!(name = %sfc_module.name, "SFC compiled successfully");

        // 2. 在 JS 运行时中执行
        self.execute_sfc_module(&sfc_module)?;
        info!("SFC module executed");

        // 3. 创建虚拟 DOM（简化版）
        self.root_vnode = Some(VNode::element("div"));
        info!("Virtual DOM created");

        Ok(())
    }

    /// 编译 SFC 模块
    fn compile_sfc(&self, path: &Path) -> Result<SfcModule, String> {
        iris_sfc::compile(path).map_err(|e| {
            format!("Failed to compile SFC {}: {}", path.display(), e)
        })
    }

    /// 在 JS 运行时中执行 SFC 模块
    fn execute_sfc_module(&mut self, sfc_module: &SfcModule) -> Result<(), String> {
        // 1. 注册模块
        self.module_registry
            .register(&sfc_module.name, &sfc_module.script);

        // 2. 注入样式
        for (index, style_block) in sfc_module.styles.iter().enumerate() {
            debug!(style_index = index, scoped = style_block.scoped, "Injecting style...");
        }

        // 3. 执行脚本
        let js_code = &sfc_module.script;
        debug!(script_len = js_code.len(), "Executing SFC script...");

        self.js_runtime.eval(js_code).map_err(|e| {
            format!("Failed to execute SFC script: {}", e)
        })?;

        Ok(())
    }

    /// 获取当前虚拟 DOM
    pub fn root_vnode(&self) -> Option<&VNode> {
        self.root_vnode.as_ref()
    }

    /// 获取 JS 运行时的可变引用
    pub fn js_runtime(&mut self) -> &mut JsRuntime {
        &mut self.js_runtime
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl Default for RuntimeOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_orchestrator() {
        let orchestrator = RuntimeOrchestrator::new();
        assert!(!orchestrator.is_initialized());
    }

    #[test]
    fn test_initialize() {
        let mut orchestrator = RuntimeOrchestrator::new();
        assert!(orchestrator.initialize().is_ok());
        assert!(orchestrator.is_initialized());
    }

    #[test]
    fn test_load_without_initialize() {
        let mut orchestrator = RuntimeOrchestrator::new();
        let result = orchestrator.load_vue_app("test.vue");
        assert!(result.is_err());
    }
}
