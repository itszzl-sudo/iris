//! JavaScript 运行时封装
//!
//! 提供脚本执行环境和值转换。
//!
//! # 注意
//!
//! QuickJS 集成需要系统安装 QuickJS 库。
//! 当前使用简化实现，后续会集成完整的 QuickJS 绑定。

use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// JavaScript 运行时环境
///
/// 封装 QuickJS Runtime 和 Context，提供安全的执行环境。
///
/// # 示例
///
/// ```rust
/// use iris_js::vm::JsRuntime;
///
/// let mut runtime = JsRuntime::new();
/// let result = runtime.eval("1 + 2");
/// assert_eq!(result.unwrap().as_int(), Some(3));
/// ```
pub struct JsRuntime {
    /// QuickJS 运行时
    runtime: Runtime,
    /// QuickJS 上下文
    context: Context,
    /// 是否已初始化
    initialized: bool,
}

impl JsRuntime {
    /// 创建新的 JS 运行时
    ///
    /// 初始化 QuickJS Runtime 和 Context。
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create QuickJS runtime");
        let context = Context::full(&runtime).expect("Failed to create context");

        Self {
            runtime,
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
    /// let result = runtime.eval("2 * 3 + 4");
    /// ```
    pub fn eval(&mut self, code: &str) -> Result<JsValue> {
        self.context.with(|ctx| {
            let result: Value = ctx.eval(code)?;
            JsValue::from_js_value(&ctx, result)
        })
    }

    /// 执行 JavaScript 代码（带文件名，用于错误报告）
    pub fn eval_with_filename(&mut self, code: &str, filename: &str) -> Result<JsValue> {
        self.context.with(|ctx| {
            let result: Value = ctx.eval(code)?;
            JsValue::from_js_value(&ctx, result)
        })
    }

    /// 设置全局属性
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_js::vm::JsRuntime;
    ///
    /// let mut runtime = JsRuntime::new();
    /// runtime.set_global("name", "Iris");
    /// ```
    pub fn set_global(&mut self, name: &str, value: JsValue) -> Result<()> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            let js_value = value.to_js_value(&ctx)?;
            globals.set(name, js_value)?;
            Ok(())
        })
    }

    /// 获取全局属性
    pub fn get_global(&self, name: &str) -> Result<Option<JsValue>> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            if let Ok(value) = globals.get::<_, Value>(name) {
                Ok(Some(JsValue::from_js_value(&ctx, value)))
            } else {
                Ok(None)
            }
        })
    }

    /// 注入 BOM API 到全局环境
    pub fn inject_bom(&mut self, window: &Window, document: &Document, console: &Console) -> Result<()> {
        self.context.with(|ctx| {
            let globals = ctx.globals();

            // 注入 console 对象
            let console_obj = Object::new(ctx.clone())?;
            console_obj.set("log", ctx.wrap_callback(|_ctx, _this, args: rquickjs::Rest<rquickjs::Value>| {
                let msgs: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
                println!("[JS Console.log] {}", msgs.join(" "));
                Ok(())
            }).expect("Failed to create callback"))?;
            console_obj.set("warn", ctx.wrap_callback(|_ctx, _this, args: rquickjs::Rest<rquickjs::Value>| {
                let msgs: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
                println!("[JS Console.warn] {}", msgs.join(" "));
                Ok(())
            }).expect("Failed to create callback"))?;
            console_obj.set("error", ctx.wrap_callback(|_ctx, _this, args: rquickjs::Rest<rquickjs::Value>| {
                let msgs: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
                eprintln!("[JS Console.error] {}", msgs.join(" "));
                Ok(())
            }).expect("Failed to create callback"))?;

            globals.set("console", console_obj)?;

            // 注入 window 对象
            let window_obj = Object::new(ctx.clone())?;
            window_obj.set("innerWidth", window.inner_width() as i32)?;
            window_obj.set("innerHeight", window.inner_height() as i32)?;
            globals.set("window", window_obj.clone())?;
            globals.set("self", window_obj)?;

            // 注入 document 对象（简化版）
            let document_obj = Object::new(ctx.clone())?;
            document_obj.set("title", "Iris App")?;
            globals.set("document", document_obj)?;

            Ok(())
        })
    }

    /// 标记为已初始化
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 触发垃圾回收
    pub fn run_gc(&mut self) {
        self.runtime.run_gc();
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// JavaScript 值类型
///
/// 封装 QuickJS 值，提供类型安全的访问。
#[derive(Debug, Clone)]
pub enum JsValue {
    /// undefined
    Undefined,
    /// null
    Null,
    /// 布尔值
    Bool(bool),
    /// 整数
    Int(i32),
    /// 浮点数
    Float(f64),
    /// 字符串
    String(String),
    /// 对象
    Object(Vec<(String, JsValue)>),
    /// 数组
    Array(Vec<JsValue>),
    /// 函数（暂不支持）
    Function,
}

impl JsValue {
    /// 从 QuickJS Value 转换
    pub fn from_js_value(ctx: &Ctx, value: Value) -> Self {
        if value.is_undefined() {
            JsValue::Undefined
        } else if value.is_null() {
            JsValue::Null
        } else if value.is_bool() {
            JsValue::Bool(value.as_bool().unwrap_or(false))
        } else if value.is_int() {
            JsValue::Int(value.as_int().unwrap_or(0))
        } else if value.is_float() {
            JsValue::Float(value.as_float().unwrap_or(0.0))
        } else if value.is_string() {
            let s: String = value.as_string().unwrap().to_string().unwrap_or_default();
            JsValue::String(s)
        } else if value.is_array() {
            // 简化处理，实际应该遍历数组
            JsValue::Array(vec![])
        } else if value.is_object() {
            JsValue::Object(vec![])
        } else {
            JsValue::Undefined
        }
    }

    /// 转换为 QuickJS Value
    pub fn to_js_value(&self, ctx: &Ctx) -> Result<Value> {
        match self {
            JsValue::Undefined => Ok(Value::new_undefined(ctx.clone())),
            JsValue::Null => Ok(Value::new_null(ctx.clone())),
            JsValue::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
            JsValue::Int(i) => Ok(Value::new_int(ctx.clone(), *i)),
            JsValue::Float(f) => Ok(Value::new_float(ctx.clone(), *f)),
            JsValue::String(s) => Ok(Value::new_string(ctx.clone(), s.clone())),
            JsValue::Object(_) => Ok(Value::new_object(ctx.clone())),
            JsValue::Array(_) => {
                let arr = rquickjs::Array::new(ctx.clone())?;
                Ok(arr.into_value())
            }
            JsValue::Function => Ok(Value::new_object(ctx.clone())),
        }
    }

    /// 获取整数值
    pub fn as_int(&self) -> Option<i32> {
        match self {
            JsValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// 获取浮点数值
    pub fn as_float(&self) -> Option<f64> {
        match self {
            JsValue::Float(f) => Some(*f),
            JsValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// 获取布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// 获取字符串值
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// 判断是否为 undefined
    pub fn is_undefined(&self) -> bool {
        matches!(self, JsValue::Undefined)
    }

    /// 判断是否为 null
    pub fn is_null(&self) -> bool {
        matches!(self, JsValue::Null)
    }

    /// 判断是否为数字
    pub fn is_number(&self) -> bool {
        matches!(self, JsValue::Int(_) | JsValue::Float(_))
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
        assert_eq!(result.as_int(), Some(3));
    }

    #[test]
    fn test_eval_expression() {
        let mut runtime = JsRuntime::new();
        let result = runtime.eval("2 * 3 + 4 * 5").unwrap();
        assert_eq!(result.as_int(), Some(26));
    }

    #[test]
    fn test_eval_string() {
        let mut runtime = JsRuntime::new();
        let result = runtime.eval("'Hello' + ' ' + 'World'").unwrap();
        assert_eq!(result.as_str(), Some("Hello World"));
    }

    #[test]
    fn test_eval_boolean() {
        let mut runtime = JsRuntime::new();
        let result = runtime.eval("true && false").unwrap();
        assert_eq!(result.as_bool(), Some(false));
    }

    #[test]
    fn test_set_get_global() {
        let mut runtime = JsRuntime::new();
        runtime.set_global("myVar", JsValue::Int(42)).unwrap();
        let value = runtime.get_global("myVar").unwrap();
        assert!(value.is_some());
        assert_eq!(value.unwrap().as_int(), Some(42));
    }

    #[test]
    fn test_eval_variable() {
        let mut runtime = JsRuntime::new();
        runtime.eval("var x = 10;").unwrap();
        runtime.eval("var y = 20;").unwrap();
        let result = runtime.eval("x + y").unwrap();
        assert_eq!(result.as_int(), Some(30));
    }

    #[test]
    fn test_eval_function() {
        let mut runtime = JsRuntime::new();
        runtime.eval("function add(a, b) { return a + b; }").unwrap();
        let result = runtime.eval("add(3, 4)").unwrap();
        assert_eq!(result.as_int(), Some(7));
    }

    #[test]
    fn test_gc() {
        let mut runtime = JsRuntime::new();
        runtime.eval("var a = new Array(1000);").unwrap();
        runtime.run_gc();
        // GC 应该不会崩溃
    }

    #[test]
    fn test_js_value_types() {
        assert!(JsValue::Undefined.is_undefined());
        assert!(JsValue::Null.is_null());
        assert!(JsValue::Int(42).is_number());
        assert!(JsValue::Float(3.14).is_number());
        assert!(!JsValue::String("test".to_string()).is_number());
    }
}
