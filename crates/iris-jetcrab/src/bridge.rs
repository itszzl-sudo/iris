//! JetCrab 与 Iris 核心模块的桥接层
//!
//! 负责将 JetCrab 运行时与 Iris 的 DOM、Layout、GPU 等模块连接。

use tracing::{debug, info};

/// JetCrab 桥接器
///
/// 提供 JetCrab 运行时与 Iris 核心模块之间的通信桥梁。
pub struct JetCrabBridge {
    /// 是否已初始化
    initialized: bool,
}

impl JetCrabBridge {
    /// 创建新的桥接器
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    /// 初始化桥接器
    pub fn init(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        info!("Initializing JetCrab bridge...");

        // TODO: 初始化桥接逻辑
        // 1. 注册 DOM API 到 JetCrab
        // 2. 注册 CSSOM API 到 JetCrab
        // 3. 注册事件系统到 JetCrab

        self.initialized = true;
        info!("JetCrab bridge initialized");
        Ok(())
    }

    /// 注册 DOM API
    pub fn register_dom_api(&self) -> Result<(), String> {
        if !self.initialized {
            return Err("Bridge not initialized".to_string());
        }

        debug!("Registering DOM API...");

        // TODO: 实现 DOM API 注册
        // - document.createElement
        // - document.getElementById
        // - element.appendChild
        // - 等等...

        Ok(())
    }

    /// 注册 CSSOM API
    pub fn register_cssom_api(&self) -> Result<(), String> {
        if !self.initialized {
            return Err("Bridge not initialized".to_string());
        }

        debug!("Registering CSSOM API...");

        // TODO: 实现 CSSOM API 注册
        // - document.styleSheets
        // - stylesheet.insertRule
        // - element.style
        // - 等等...

        Ok(())
    }

    /// 注册事件系统
    pub fn register_event_system(&self) -> Result<(), String> {
        if !self.initialized {
            return Err("Bridge not initialized".to_string());
        }

        debug!("Registering event system...");

        // TODO: 实现事件系统注册
        // - element.addEventListener
        // - element.removeEventListener
        // - element.dispatchEvent
        // - 等等...

        Ok(())
    }

    /// 执行 Vue SFC 编译后的代码
    pub fn execute_sfc_code(&self, code: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("Bridge not initialized".to_string());
        }

        debug!("Executing SFC compiled code ({} chars)", code.len());

        // TODO: 在 JetCrab 中执行代码
        // let runtime = JetCrabRuntime::new();
        // runtime.eval(code)

        Ok("undefined".to_string())
    }

    /// 检查桥接器是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 关闭桥接器
    pub fn shutdown(&mut self) -> Result<(), String> {
        if !self.initialized {
            return Ok(());
        }

        info!("Shutting down JetCrab bridge...");

        // TODO: 清理桥接逻辑

        self.initialized = false;
        info!("JetCrab bridge shut down");
        Ok(())
    }
}

impl Default for JetCrabBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for JetCrabBridge {
    fn drop(&mut self) {
        if self.initialized {
            let _ = self.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bridge() {
        let bridge = JetCrabBridge::new();
        assert!(!bridge.is_initialized());
    }

    #[test]
    fn test_init_bridge() {
        let mut bridge = JetCrabBridge::new();
        let result = bridge.init();
        assert!(result.is_ok());
        assert!(bridge.is_initialized());
    }

    #[test]
    fn test_double_init() {
        let mut bridge = JetCrabBridge::new();
        bridge.init().unwrap();
        let result = bridge.init(); // 第二次应该也成功
        assert!(result.is_ok());
    }

    #[test]
    fn test_shutdown() {
        let mut bridge = JetCrabBridge::new();
        bridge.init().unwrap();
        let result = bridge.shutdown();
        assert!(result.is_ok());
        assert!(!bridge.is_initialized());
    }
}
