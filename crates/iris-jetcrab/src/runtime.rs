//! JetCrab Runtime 核心实现
//!
//! 提供 JavaScript 执行环境和生命周期管理。

use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};

/// JetCrab 运行时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// 是否启用严格模式
    pub strict_mode: bool,
    /// 最大执行时间（毫秒）
    pub max_execution_time_ms: u64,
    /// 是否启用 Source Map
    pub enable_source_map: bool,
    /// 内存限制（MB）
    pub memory_limit_mb: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            max_execution_time_ms: 5000,
            enable_source_map: false,
            memory_limit_mb: 512,
        }
    }
}

/// JetCrab 运行时
///
/// 封装 JetCrab 引擎，提供 JavaScript 执行环境。
///
/// # 示例
///
/// ```rust,ignore
/// use iris_jetcrab::JetCrabRuntime;
///
/// let mut runtime = JetCrabRuntime::new();
/// runtime.init().unwrap();
///
/// let result = runtime.eval("1 + 2");
/// assert_eq!(result, 3);
/// ```
pub struct JetCrabRuntime {
    /// 运行时配置
    config: RuntimeConfig,
    /// 是否已初始化
    initialized: bool,
    /// TODO: JetCrab 引擎实例
    // engine: jetcrab::Engine,
}

impl JetCrabRuntime {
    /// 创建新的 JetCrab 运行时
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            initialized: false,
        }
    }

    /// 使用自定义配置创建运行时
    pub fn with_config(config: RuntimeConfig) -> Self {
        Self {
            config,
            initialized: false,
        }
    }

    /// 初始化运行时
    ///
    /// # 错误
    ///
    /// 如果初始化失败，返回错误信息
    pub fn init(&mut self) -> Result<(), String> {
        if self.initialized {
            warn!("Runtime already initialized");
            return Ok(());
        }

        info!("Initializing JetCrab runtime...");
        debug!("Configuration: {:?}", self.config);

        // TODO: 初始化 JetCrab 引擎
        // self.engine = jetcrab::Engine::new()?;

        self.initialized = true;
        info!("JetCrab runtime initialized");
        Ok(())
    }

    /// 执行 JavaScript 代码
    ///
    /// # 参数
    ///
    /// * `code` - JavaScript 代码字符串
    ///
    /// # 返回值
    ///
    /// 执行结果
    ///
    /// # 错误
    ///
    /// 如果执行失败，返回错误信息
    pub fn eval(&self, code: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("Runtime not initialized. Call init() first.".to_string());
        }

        debug!("Evaluating JavaScript code ({} chars)", code.len());

        // TODO: 使用 JetCrab 引擎执行代码
        // let result = self.engine.eval(code)?;
        // Ok(result.to_string())

        // 临时实现
        Ok("undefined".to_string())
    }

    /// 执行 JavaScript 文件
    ///
    /// # 参数
    ///
    /// * `path` - 文件路径
    pub fn eval_file(&self, path: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("Runtime not initialized".to_string());
        }

        debug!("Evaluating file: {}", path);

        // 读取文件内容
        let code = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // 执行代码
        self.eval(&code)
    }

    /// 设置全局变量
    ///
    /// # 参数
    ///
    /// * `name` - 变量名
    /// * `value` - 变量值（JSON 格式）
    pub fn set_global(&self, name: &str, value: &str) -> Result<(), String> {
        if !self.initialized {
            return Err("Runtime not initialized".to_string());
        }

        debug!("Setting global variable: {} = {}", name, value);

        // TODO: 设置全局变量
        // self.engine.set_global(name, value)?;

        Ok(())
    }

    /// 获取全局变量
    pub fn get_global(&self, name: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("Runtime not initialized".to_string());
        }

        debug!("Getting global variable: {}", name);

        // TODO: 获取全局变量
        // let value = self.engine.get_global(name)?;
        // Ok(value.to_string())

        Ok("undefined".to_string())
    }

    /// 检查运行时是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 获取运行时配置
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// 关闭运行时
    pub fn shutdown(&mut self) -> Result<(), String> {
        if !self.initialized {
            return Ok(());
        }

        info!("Shutting down JetCrab runtime...");

        // TODO: 清理 JetCrab 引擎
        // self.engine.shutdown()?;

        self.initialized = false;
        info!("JetCrab runtime shut down");
        Ok(())
    }
}

impl Default for JetCrabRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for JetCrabRuntime {
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
    fn test_create_runtime() {
        let runtime = JetCrabRuntime::new();
        assert!(!runtime.is_initialized());
    }

    #[test]
    fn test_runtime_config() {
        let config = RuntimeConfig {
            strict_mode: false,
            max_execution_time_ms: 10000,
            enable_source_map: true,
            memory_limit_mb: 1024,
        };

        let runtime = JetCrabRuntime::with_config(config);
        assert!(!runtime.config().strict_mode);
        assert_eq!(runtime.config().max_execution_time_ms, 10000);
        assert!(runtime.config().enable_source_map);
        assert_eq!(runtime.config().memory_limit_mb, 1024);
    }

    #[test]
    fn test_eval_without_init() {
        let runtime = JetCrabRuntime::new();
        let result = runtime.eval("1 + 2");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not initialized"));
    }

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert!(config.strict_mode);
        assert_eq!(config.max_execution_time_ms, 5000);
        assert!(!config.enable_source_map);
        assert_eq!(config.memory_limit_mb, 512);
    }
}
