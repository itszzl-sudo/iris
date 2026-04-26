//! JavaScript 运行时封装
//!
//! 基于 Boa JavaScript 引擎（纯 Rust 实现，无需系统依赖）。
//!
//! # 示例
//!
//! ```rust
//! use iris_js::vm::JsRuntime;
//!
//! let mut runtime = JsRuntime::new();
//! let result = runtime.eval("1 + 2").unwrap();
//! ```

use boa_engine::{Context, Source, JsValue, js_string, object::ObjectInitializer, property::Attribute};

/// JavaScript 运行时环境
///
/// 封装 Boa JavaScript 引擎，提供安全的执行环境。
///
/// # 示例
///
/// ```rust
/// use iris_js::vm::JsRuntime;
///
/// let mut runtime = JsRuntime::new();
/// let result = runtime.eval("2 * 3 + 4").unwrap();
/// ```
pub struct JsRuntime {
    /// JavaScript 上下文
    context: Context,
    /// 是否已初始化
    initialized: bool,
}

impl JsRuntime {
    /// 创建新的 JS 运行时
    ///
    /// 初始化 Boa JavaScript 引擎。
    pub fn new() -> Self {
        let context = Context::default();
        Self {
            context,
            initialized: false,
        }
    }

    /// 执行 JavaScript 代码
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_js::vm::JsRuntime;
    ///
    /// let mut runtime = JsRuntime::new();
    /// let result = runtime.eval("2 * 3 + 4").unwrap();
    /// ```
    pub fn eval(&mut self, code: &str) -> Result<JsValue, String> {
        self.context
            .eval(Source::from_bytes(code))
            .map_err(|e| format!("JS Error: {}", e))
    }

    /// 设置全局属性
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_js::vm::JsRuntime;
    /// use boa_engine::js_string;
    ///
    /// let mut runtime = JsRuntime::new();
    /// runtime.set_global("name", js_string!("Iris").into()).unwrap();
    /// ```
    pub fn set_global(&mut self, name: &str, value: JsValue) -> Result<(), String> {
        self.context
            .register_global_property(js_string!(name), value, Attribute::all())
            .map_err(|e| format!("Failed to set global property: {}", e))
    }

    /// 获取全局属性
    pub fn get_global(&mut self, name: &str) -> JsValue {
        self.context
            .global_object()
            .get(js_string!(name), &mut self.context)
            .unwrap_or(JsValue::undefined())
    }

    /// 注入 BOM API 到全局环境
    pub fn inject_bom(&mut self, inner_width: u32, inner_height: u32) -> Result<(), String> {
        // 注入 window 对象
        let window = ObjectInitializer::new(&mut self.context)
            .property(js_string!("innerWidth"), inner_width as f64, Attribute::all())
            .property(js_string!("innerHeight"), inner_height as f64, Attribute::all())
            .build();

        self.context
            .register_global_property(js_string!("window"), window.clone(), Attribute::all())
            .map_err(|e| format!("Failed to set window: {}", e))?;

        self.context
            .register_global_property(js_string!("self"), window, Attribute::all())
            .map_err(|e| format!("Failed to set self: {}", e))?;

        // 注入 console 对象
        let console_code = r#"(function(...args) { /* native log */ })"#;
        let log_func = self.eval(console_code)?;
        
        let console = ObjectInitializer::new(&mut self.context)
            .property(
                js_string!("log"),
                log_func,
                Attribute::all(),
            )
            .build();

        self.context
            .register_global_property(js_string!("console"), console, Attribute::all())
            .map_err(|e| format!("Failed to set console: {}", e))?;

        // 注入 document 对象（简化版）
        let document = ObjectInitializer::new(&mut self.context)
            .property(js_string!("title"), js_string!("Iris App"), Attribute::all())
            .build();

        self.context
            .register_global_property(js_string!("document"), document, Attribute::all())
            .map_err(|e| format!("Failed to set document: {}", e))?;

        Ok(())
    }

    /// 标记为已初始化
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// JavaScript 值类型转换辅助
///
/// 提供 Boa JsValue 到 Rust 类型的转换。
pub struct JsValueHelper;

impl JsValueHelper {
    /// 获取整数值
    pub fn as_int(value: &JsValue) -> Option<i32> {
        value.as_number().map(|n| n as i32)
    }

    /// 获取浮点数值
    pub fn as_float(value: &JsValue) -> Option<f64> {
        value.as_number()
    }

    /// 获取布尔值
    pub fn as_bool(value: &JsValue) -> bool {
        value.as_boolean().unwrap_or(false)
    }

    /// 获取字符串值
    pub fn as_str(value: &JsValue, context: &mut Context) -> Option<String> {
        value
            .to_string(context)
            .ok()
            .map(|s| s.to_std_string_escaped())
    }

    /// 判断是否为 undefined
    pub fn is_undefined(value: &JsValue) -> bool {
        value.is_undefined()
    }

    /// 判断是否为 null
    pub fn is_null(value: &JsValue) -> bool {
        value.is_null()
    }

    /// 判断是否为数字
    pub fn is_number(value: &JsValue) -> bool {
        value.is_number()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_runtime() {
        let runtime = JsRuntime::new();
        assert!(!runtime.is_initialized());
    }

    #[test]
    fn test_eval_simple() {
        let mut runtime = JsRuntime::new();
        let result = runtime.eval("1 + 2").unwrap();
        assert_eq!(JsValueHelper::as_int(&result), Some(3));
    }

    #[test]
    fn test_eval_expression() {
        let mut runtime = JsRuntime::new();
        let result = runtime.eval("2 * 3 + 4 * 5").unwrap();
        assert_eq!(JsValueHelper::as_int(&result), Some(26));
    }

    #[test]
    fn test_eval_string() {
        let mut runtime = JsRuntime::new();
        let result = runtime.eval("'Hello' + ' ' + 'World'").unwrap();
        let mut ctx = Context::default();
        assert_eq!(
            JsValueHelper::as_str(&result, &mut ctx),
            Some("Hello World".to_string())
        );
    }

    #[test]
    fn test_eval_boolean() {
        let mut runtime = JsRuntime::new();
        let result = runtime.eval("true && false").unwrap();
        assert!(!JsValueHelper::as_bool(&result));
    }

    #[test]
    fn test_set_get_global() {
        let mut runtime = JsRuntime::new();
        runtime.set_global("myVar", JsValue::from(42)).unwrap();
        let value = runtime.get_global("myVar");
        assert_eq!(JsValueHelper::as_int(&value), Some(42));
    }

    #[test]
    fn test_eval_variable() {
        let mut runtime = JsRuntime::new();
        runtime.eval("var x = 10;").unwrap();
        runtime.eval("var y = 20;").unwrap();
        let result = runtime.eval("x + y").unwrap();
        assert_eq!(JsValueHelper::as_int(&result), Some(30));
    }

    #[test]
    fn test_eval_function() {
        let mut runtime = JsRuntime::new();
        runtime.eval("function add(a, b) { return a + b; }").unwrap();
        let result = runtime.eval("add(3, 4)").unwrap();
        assert_eq!(JsValueHelper::as_int(&result), Some(7));
    }

    #[test]
    fn test_eval_arrow_function() {
        let mut runtime = JsRuntime::new();
        runtime.eval("const multiply = (a, b) => a * b;").unwrap();
        let result = runtime.eval("multiply(3, 4)").unwrap();
        assert_eq!(JsValueHelper::as_int(&result), Some(12));
    }

    #[test]
    fn test_eval_object() {
        let mut runtime = JsRuntime::new();
        runtime.eval("const obj = { name: 'Iris', version: '0.1.0' };").unwrap();
        let result = runtime.eval("obj.name").unwrap();
        let mut ctx = Context::default();
        assert_eq!(
            JsValueHelper::as_str(&result, &mut ctx),
            Some("Iris".to_string())
        );
    }

    #[test]
    fn test_eval_array() {
        let mut runtime = JsRuntime::new();
        runtime.eval("const arr = [1, 2, 3, 4, 5];").unwrap();
        let result = runtime.eval("arr.length").unwrap();
        assert_eq!(JsValueHelper::as_int(&result), Some(5));
    }

    #[test]
    fn test_js_value_helper() {
        assert!(JsValueHelper::is_undefined(&JsValue::undefined()));
        assert!(JsValueHelper::is_null(&JsValue::null()));
        assert!(JsValueHelper::is_number(&JsValue::from(42)));
        assert!(JsValueHelper::is_number(&JsValue::from(3.14)));
        assert!(!JsValueHelper::is_number(&JsValue::from(js_string!("test"))));
    }

    #[test]
    fn test_inject_bom() {
        let mut runtime = JsRuntime::new();
        runtime.inject_bom(800, 600).unwrap();

        let width = runtime.eval("window.innerWidth").unwrap();
        assert_eq!(JsValueHelper::as_int(&width), Some(800));

        let height = runtime.eval("window.innerHeight").unwrap();
        assert_eq!(JsValueHelper::as_int(&height), Some(600));
    }

    #[test]
    fn test_mark_initialized() {
        let mut runtime = JsRuntime::new();
        assert!(!runtime.is_initialized());

        runtime.mark_initialized();
        assert!(runtime.is_initialized());
    }
}
