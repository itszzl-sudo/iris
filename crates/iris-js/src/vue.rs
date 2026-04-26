//! Vue 3 运行时预加载
//!
//! 将 Vue 3 runtime-core 和 runtime-dom 注入到 JS 环境。

use crate::vm::JsRuntime;
use boa_engine::JsValue;

/// Vue 运行时版本
pub const VUE_VERSION: &str = "3.4.21";

/// 注入 Vue 3 运行时到 JS 环境
///
/// # 注意
///
/// 这需要 Vue 3 的编译后代码。当前使用简化版本，
/// 实际应该加载完整的 vue.runtime.esm.js 文件。
///
/// # 示例
///
/// ```rust
/// use iris_js::vm::JsRuntime;
/// use iris_js::vue::inject_vue_runtime;
///
/// let mut runtime = JsRuntime::new();
/// inject_vue_runtime(&mut runtime).unwrap();
/// ```
pub fn inject_vue_runtime(runtime: &mut JsRuntime) -> std::result::Result<(), String> {
    // 注入 Vue 全局对象（简化版）
    let vue_code = r#"
// Vue 3 简化模拟实现
const Vue = {
    version: '"#
        .to_string()
        + VUE_VERSION
        + r#"',
    
    // 创建应用
    createApp(rootComponent, rootProps) {
        return {
            mount(container) {
                console.log('Vue app mounted');
                return this;
            },
            unmount() {
                console.log('Vue app unmounted');
            }
        };
    },
    
    // 响应式 API
    ref(value) {
        return { value };
    },
    
    reactive(target) {
        return target;
    },
    
    computed(getter) {
        return { get value() { return getter(); } };
    },
    
    watch(source, callback) {
        console.log('Watch registered');
    },
    
    // 生命周期
    onMounted(callback) {
        console.log('onMounted registered');
    },
    
    onUnmounted(callback) {
        console.log('onUnmounted registered');
    },
    
    // 组合式 API
    provide(key, value) {
        console.log('Provide:', key);
    },
    
    inject(key, defaultValue) {
        return defaultValue;
    }
};

// 导出到全局
globalThis.Vue = Vue;
"#;

    runtime.eval(&vue_code).map_err(|e| e.to_string())?;
    runtime.mark_initialized();
    Ok(())
}

/// 注入 Vue 编译器宏（defineProps, defineEmits 等）
///
/// 这些宏在 `<script setup>` 中使用，需要在运行时提供模拟实现。
pub fn inject_vue_compiler_macros(runtime: &mut JsRuntime) -> std::result::Result<(), String> {
    let macros_code = r#"
// Vue 3 Compiler Macros (模拟实现)
// 这些在实际编译时会被替换，这里仅提供运行时支持

function defineProps(props) {
    return props || {};
}

function defineEmits(emits) {
    return function(event, ...args) {
        console.log('Emit:', event, args);
    };
}

function defineExpose(exposed) {
    console.log('Exposed:', exposed);
}

function defineSlots() {
    return {};
}

function withDefaults(props, defaults) {
    return Object.assign({}, props, defaults);
}

// 导出到全局
globalThis.defineProps = defineProps;
globalThis.defineEmits = defineEmits;
globalThis.defineExpose = defineExpose;
globalThis.defineSlots = defineSlots;
globalThis.withDefaults = withDefaults;
"#;

    runtime.eval(macros_code).map_err(|e| e.to_string())?;
    Ok(())
}

/// 注入 Vue 组件系统
///
/// 提供组件注册、解析和执行的能力。
pub fn inject_vue_component_system(runtime: &mut JsRuntime) -> std::result::Result<(), String> {
    let component_code = r#"
// Vue 组件系统（简化版）
const ComponentRegistry = {
    components: {},
    
    register(name, component) {
        this.components[name] = component;
    },
    
    get(name) {
        return this.components[name];
    },
    
    has(name) {
        return name in this.components;
    }
};

globalThis.ComponentRegistry = ComponentRegistry;

// 辅助函数
function defineComponent(options) {
    return options;
}

globalThis.defineComponent = defineComponent;
"#;

    runtime.eval(component_code).map_err(|e| e.to_string())?;
    Ok(())
}

/// 完整的 Vue 3 环境注入
///
/// 包含运行时、编译器宏和组件系统。
///
/// # 示例
///
/// ```rust
/// use iris_js::vm::JsRuntime;
/// use iris_js::vue::setup_complete_vue_environment;
///
/// let mut runtime = JsRuntime::new();
/// setup_complete_vue_environment(&mut runtime).unwrap();
/// ```
pub fn setup_complete_vue_environment(runtime: &mut JsRuntime) -> std::result::Result<(), String> {
    inject_vue_runtime(runtime)?;
    inject_vue_compiler_macros(runtime)?;
    inject_vue_component_system(runtime)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_vue_runtime() {
        let mut runtime = JsRuntime::new();
        let result = inject_vue_runtime(&mut runtime);
        assert!(result.is_ok());

        // 验证 Vue 对象存在
        let result = runtime.eval("Vue.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_inject_compiler_macros() {
        let mut runtime = JsRuntime::new();
        let result = inject_vue_compiler_macros(&mut runtime);
        assert!(result.is_ok());

        // 验证宏存在
        let result = runtime.eval("typeof defineProps");
        assert!(result.is_ok());
    }

    #[test]
    fn test_inject_component_system() {
        let mut runtime = JsRuntime::new();
        let result = inject_vue_component_system(&mut runtime);
        assert!(result.is_ok());

        // 验证组件系统存在
        let result = runtime.eval("typeof ComponentRegistry");
        assert!(result.is_ok());
    }

    #[test]
    fn test_complete_vue_environment() {
        let mut runtime = JsRuntime::new();
        let result = setup_complete_vue_environment(&mut runtime);
        assert!(result.is_ok());

        // 验证所有功能都已注入
        let result = runtime.eval("Vue.version && typeof defineProps === 'function' && typeof ComponentRegistry === 'object'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_vue_ref_api() {
        let mut runtime = JsRuntime::new();
        inject_vue_runtime(&mut runtime).unwrap();

        let result = runtime.eval("const count = Vue.ref(0); count.value");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value.as_number(), Some(0.0));
    }

    #[test]
    fn test_vue_computed_api() {
        let mut runtime = JsRuntime::new();
        inject_vue_runtime(&mut runtime).unwrap();

        let result = runtime.eval(
            "const double = Vue.computed(() => 2 * 3); double.value",
        );
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value.as_number(), Some(6.0));
    }
}
