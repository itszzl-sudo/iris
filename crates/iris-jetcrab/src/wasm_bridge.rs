//! WASM 桥接模块
//!
//! 提供 WASM 模块加载和 Rust ↔ JavaScript FFI 支持。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// WASM 导出函数
#[derive(Debug, Clone)]
pub struct WasmExport {
    /// 函数名
    pub name: String,
    /// 参数数量
    pub params: u32,
    /// 返回值数量
    pub results: u32,
}

/// WASM 模块信息
#[derive(Debug, Clone)]
pub struct WasmModuleInfo {
    /// 模块名称
    pub name: String,
    /// 模块路径
    pub path: String,
    /// 导出函数列表
    pub exports: Vec<WasmExport>,
    /// 是否已实例化
    pub instantiated: bool,
}

/// WASM 实例
pub struct WasmInstance {
    /// 模块信息
    module_info: WasmModuleInfo,
    /// 内存指针（模拟）
    memory_ptr: usize,
    /// 导出函数缓存
    exported_functions: HashMap<String, Box<dyn Fn(&[u32]) -> Vec<u32> + Send + Sync>>,
}

impl WasmInstance {
    /// 调用导出函数
    pub fn call_export(&self, name: &str, args: &[u32]) -> Result<Vec<u32>, String> {
        if let Some(func) = self.exported_functions.get(name) {
            Ok(func(args))
        } else {
            Err(format!("Export function not found: {}", name))
        }
    }

    /// 获取内存指针
    pub fn memory_ptr(&self) -> usize {
        self.memory_ptr
    }

    /// 获取模块信息
    pub fn module_info(&self) -> &WasmModuleInfo {
        &self.module_info
    }
}

/// WASM 加载器
pub struct WasmLoader {
    /// 已加载的模块
    modules: HashMap<String, WasmModuleInfo>,
    /// 已实例化的模块
    instances: HashMap<String, Arc<Mutex<WasmInstance>>>,
}

impl WasmLoader {
    /// 创建新的 WASM 加载器
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    /// 加载 WASM 模块
    pub fn load_module(&mut self, name: &str, path: &str) -> Result<WasmModuleInfo, String> {
        info!("Loading WASm module: {} from {}", name, path);

        // 检查是否已加载
        if let Some(module) = self.modules.get(name) {
            debug!("WASM module cache hit: {}", name);
            return Ok(module.clone());
        }

        // 读取 WASM 文件
        let wasm_bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read WASM file: {}", e))?;

        // 解析 WASM 模块（实际应该使用 wasmtime 或其他 WASM 运行时）
        let exports = self.parse_wasm_exports(&wasm_bytes)?;

        let module_info = WasmModuleInfo {
            name: name.to_string(),
            path: path.to_string(),
            exports,
            instantiated: false,
        };

        // 缓存模块
        self.modules.insert(name.to_string(), module_info.clone());

        info!("WASM module loaded: {} ({} exports)", name, module_info.exports.len());

        Ok(module_info)
    }

    /// 解析 WASM 导出（模拟）
    fn parse_wasm_exports(&self, _wasm_bytes: &[u8]) -> Result<Vec<WasmExport>, String> {
        // 实际应该解析 WASM 二进制格式
        // 这里返回模拟数据
        Ok(vec![
            WasmExport {
                name: "add".to_string(),
                params: 2,
                results: 1,
            },
            WasmExport {
                name: "fibonacci".to_string(),
                params: 1,
                results: 1,
            },
        ])
    }

    /// 实例化 WASM 模块
    pub fn instantiate(&mut self, name: &str) -> Result<Arc<Mutex<WasmInstance>>, String> {
        info!("Instantiating WASM module: {}", name);

        // 检查是否已实例化
        if let Some(instance) = self.instances.get(name) {
            return Ok(instance.clone());
        }

        // 获取模块信息
        let module_info = self.modules.get(name)
            .ok_or_else(|| format!("WASM module not found: {}", name))?;

        // 创建实例
        let instance = WasmInstance {
            module_info: module_info.clone(),
            memory_ptr: 0x1000, // 模拟内存地址
            exported_functions: self.create_export_functions(&module_info.exports)?,
        };

        let instance = Arc::new(Mutex::new(instance));

        // 缓存实例
        self.instances.insert(name.to_string(), instance.clone());

        // 更新模块状态
        if let Some(module) = self.modules.get_mut(name) {
            module.instantiated = true;
        }

        Ok(instance)
    }

    /// 创建导出函数（模拟）
    fn create_export_functions(
        &self,
        exports: &[WasmExport],
    ) -> Result<HashMap<String, Box<dyn Fn(&[u32]) -> Vec<u32> + Send + Sync>>, String> {
        let mut functions = HashMap::new();

        for export in exports {
            let name = export.name.clone();
            
            // 创建模拟函数
            let func: Box<dyn Fn(&[u32]) -> Vec<u32> + Send + Sync> = match name.as_str() {
                "add" => Box::new(|args: &[u32]| {
                    if args.len() == 2 {
                        vec![args[0] + args[1]]
                    } else {
                        vec![0]
                    }
                }),
                "fibonacci" => Box::new(|args: &[u32]| {
                    if args.len() == 1 {
                        vec![fibonacci(args[0])]
                    } else {
                        vec![0]
                    }
                }),
                _ => Box::new(|_| vec![]),
            };

            functions.insert(name, func);
        }

        Ok(functions)
    }

    /// 获取已加载的模块
    pub fn get_module(&self, name: &str) -> Option<WasmModuleInfo> {
        self.modules.get(name).cloned()
    }

    /// 获取已实例化的模块
    pub fn get_instance(&self, name: &str) -> Option<Arc<Mutex<WasmInstance>>> {
        self.instances.get(name).cloned()
    }

    /// 列出所有已加载的模块
    pub fn list_modules(&self) -> Vec<WasmModuleInfo> {
        self.modules.values().cloned().collect()
    }

    /// 卸载模块
    pub fn unload_module(&mut self, name: &str) -> Result<(), String> {
        self.modules.remove(name)
            .ok_or_else(|| format!("Module not found: {}", name))?;
        self.instances.remove(name);
        
        info!("WASM module unloaded: {}", name);
        Ok(())
    }

    /// 清除所有缓存
    pub fn clear_cache(&mut self) {
        self.modules.clear();
        self.instances.clear();
        info!("WASM cache cleared");
    }

    /// 获取加载的模块数量
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

impl Default for WasmLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Fibonacci 计算（用于测试）
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut a = 0;
            let mut b = 1;
            for _ in 2..=n {
                let temp = a + b;
                a = b;
                b = temp;
            }
            b
        }
    }
}

/// JavaScript FFI 桥
pub struct JsFFIBridge {
    /// 注册的 JavaScript 函数
    js_functions: HashMap<String, Box<dyn Fn(&[String]) -> String + Send + Sync>>,
}

impl JsFFIBridge {
    /// 创建新的 FFI 桥
    pub fn new() -> Self {
        Self {
            js_functions: HashMap::new(),
        }
    }

    /// 注册 JavaScript 函数
    pub fn register_js_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&[String]) -> String + Send + Sync + 'static,
    {
        self.js_functions.insert(name.to_string(), Box::new(func));
        debug!("Registered JS function: {}", name);
    }

    /// 调用 JavaScript 函数
    pub fn call_js_function(&self, name: &str, args: &[String]) -> Result<String, String> {
        if let Some(func) = self.js_functions.get(name) {
            Ok(func(args))
        } else {
            Err(format!("JavaScript function not found: {}", name))
        }
    }

    /// 移除 JavaScript 函数
    pub fn unregister_js_function(&mut self, name: &str) -> Option<Box<dyn Fn(&[String]) -> String + Send + Sync>> {
        self.js_functions.remove(name)
    }

    /// 获取已注册的函数列表
    pub fn list_functions(&self) -> Vec<String> {
        self.js_functions.keys().cloned().collect()
    }

    /// 清除所有注册的函数
    pub fn clear(&mut self) {
        self.js_functions.clear();
        info!("JS FFI bridge cleared");
    }
}

impl Default for JsFFIBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_loader() {
        let mut loader = WasmLoader::new();
        assert_eq!(loader.module_count(), 0);
    }

    #[test]
    fn test_fibonacci() {
        assert_eq!(fibonacci(0), 0);
        assert_eq!(fibonacci(1), 1);
        assert_eq!(fibonacci(2), 1);
        assert_eq!(fibonacci(10), 55);
    }

    #[test]
    fn test_js_ffi_bridge() {
        let mut bridge = JsFFIBridge::new();

        // 注册函数
        bridge.register_js_function("greet", |args: &[String]| {
            if args.is_empty() {
                "Hello!".to_string()
            } else {
                format!("Hello, {}!", args[0])
            }
        });

        // 调用函数
        let result = bridge.call_js_function("greet", &["World".to_string()]);
        assert_eq!(result, Ok("Hello, World!".to_string()));

        // 列出函数
        let functions = bridge.list_functions();
        assert!(functions.contains(&"greet".to_string()));
    }

    #[test]
    fn test_wasm_exports() {
        let export = WasmExport {
            name: "test".to_string(),
            params: 2,
            results: 1,
        };

        assert_eq!(export.name, "test");
        assert_eq!(export.params, 2);
        assert_eq!(export.results, 1);
    }
}
