//! Vue 3 运行时预加载
//!
//! 将 Vue 3 runtime-core 和 runtime-dom 注入到 JS 环境。

use crate::vm::JsRuntime;
use iris_layout::vdom::{VElement, VNode, VTree};
use std::cell::RefCell;
use std::collections::HashMap;

/// Vue 运行时版本
pub const VUE_VERSION: &str = "3.4.21";

/// VNode 注册表 - 管理 JavaScript 创建的 VNode
pub struct VNodeRegistry {
    nodes: HashMap<u32, VNode>,
    next_id: u32,
}

impl VNodeRegistry {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 1,
        }
    }

    /// 创建元素 VNode 并返回 ID
    pub fn create_element(
        &mut self,
        tag: &str,
        props: Option<HashMap<String, String>>,
        children_ids: Vec<u32>,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        // 先收集子节点
        let children: Vec<VNode> = children_ids
            .iter()
            .filter_map(|child_id| self.nodes.remove(child_id))
            .collect();

        let vnode = VNode::Element(VElement {
            tag: tag.to_string(),
            attrs: props.unwrap_or_default(),
            children,
            key: None,
        });

        self.nodes.insert(id, vnode);
        id
    }

    /// 创建文本 VNode 并返回 ID
    pub fn create_text(&mut self, content: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let vnode = VNode::Text(content.to_string());
        self.nodes.insert(id, vnode);

        id
    }

    /// 创建注释 VNode 并返回 ID
    pub fn create_comment(&mut self, content: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let vnode = VNode::Comment(content.to_string());
        self.nodes.insert(id, vnode);

        id
    }

    /// 获取根 VNode 并构建完整树
    pub fn build_tree(&mut self, root_id: u32) -> Option<VTree> {
        if let Some(root_node) = self.nodes.remove(&root_id) {
            Some(VTree { root: root_node })
        } else {
            None
        }
    }
}

/// 全局 VNode 注册表实例（通过 RefCell 实现内部可变性）
thread_local! {
    static VNODE_REGISTRY: RefCell<VNodeRegistry> = RefCell::new(VNodeRegistry::new());
}

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

/// 注入 Render 辅助函数（h, text, comment）
///
/// 这些函数由 SFC 编译器生成的 render 函数调用，
/// 会创建 VNode 并注册到 Rust 端的 VNodeRegistry。
pub fn inject_render_helpers(runtime: &mut JsRuntime) -> std::result::Result<(), String> {
    let helpers_code = r#"
// Render helper functions for Vue 3 SFC

// 创建元素 VNode
// h(tag, props, children)
let __vnode_counter = 0;

function h(tag, props, children) {
    const id = ++__vnode_counter;
    
    // 处理参数
    if (typeof props === 'object' && !Array.isArray(props)) {
        // props 是对象
    } else if (Array.isArray(props)) {
        // props 实际是 children
        children = props;
        props = null;
    } else if (typeof props === 'string' || typeof props === 'number') {
        // props 实际是单个子节点
        children = [props];
        props = null;
    }
    
    // 存储 VNode 信息到全局映射表（使用字符串键）
    if (!globalThis.__vnode_map) {
        globalThis.__vnode_map = {};
    }
    
    globalThis.__vnode_map[String(id)] = {
        type: 'element',
        tag: tag,
        props: props,
        children: (children || []).map(c => String(c))
    };
    
    return id;
}

// 创建文本 VNode
function text(content) {
    const id = ++__vnode_counter;
    
    if (!globalThis.__vnode_map) {
        globalThis.__vnode_map = {};
    }
    
    globalThis.__vnode_map[String(id)] = {
        type: 'text',
        content: String(content)
    };
    
    return id;
}

// 创建注释 VNode
function comment(content) {
    const id = ++__vnode_counter;
    
    if (!globalThis.__vnode_map) {
        globalThis.__vnode_map = {};
    }
    
    globalThis.__vnode_map[String(id)] = {
        type: 'comment',
        content: String(content)
    };
    
    return id;
}

// 导出到全局
globalThis.h = h;
globalThis.text = text;
globalThis.comment = comment;
"#;

    runtime.eval(helpers_code).map_err(|e| e.to_string())?;
    Ok(())
}

/// 执行 render 函数并构建 VTree
///
/// # 参数
///
/// * `runtime` - JavaScript 运行时
/// * `render_fn` - SFC 编译后的 render 函数代码
///
/// # 返回
///
/// 返回构建的 VTree
pub fn execute_render_function(
    runtime: &mut JsRuntime,
    render_fn: &str,
) -> std::result::Result<VTree, String> {
    // 1. 清空 VNode 映射表
    runtime
        .eval("globalThis.__vnode_map = {}; globalThis.__vnode_counter = 0;")
        .map_err(|e| e.to_string())?;

    // 2. 执行 render 函数
    runtime.eval(render_fn).map_err(|e| e.to_string())?;

    // 3. 调用 render 函数获取根节点 ID
    let result = runtime
        .eval("render()")
        .map_err(|e| format!("Failed to execute render function: {}", e))?;

    let root_id = result
        .as_number()
        .ok_or("Render function did not return a vnode ID")? as u32;

    // 4. 从 JavaScript 获取 VNode 映射表
    let map_json = runtime
        .eval("JSON.stringify(globalThis.__vnode_map)")
        .map_err(|e| e.to_string())?;

    let map_str = {
        let js_string = map_json
            .as_string()
            .ok_or("Failed to get vnode map as string")?;
        
        // 直接使用 replace 移除转义
        let raw = format!("{:?}", js_string);
        raw.replace("\\\"", "\"")
            .trim_matches('"')
            .to_string()
    };

    // 5. 解析 VNode 映射表并构建 VTree
    build_vtree_from_map(&map_str, root_id)
}

/// 从 JSON 映射表构建 VTree
fn build_vtree_from_map(map_json: &str, root_id: u32) -> std::result::Result<VTree, String> {
    use serde_json::Value;

    let map: HashMap<String, Value> =
        serde_json::from_str(map_json).map_err(|e| format!("Failed to parse vnode map: {}", e))?;

    fn build_node(
        id: u32,
        map: &HashMap<String, Value>,
    ) -> std::result::Result<VNode, String> {
        let node_data = map
            .get(&id.to_string())
            .ok_or(format!("VNode {} not found in map", id))?;

        let node_type = node_data
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("VNode missing type field")?;

        match node_type {
            "element" => {
                let tag = node_data
                    .get("tag")
                    .and_then(|v| v.as_str())
                    .ok_or("Element VNode missing tag")?
                    .to_string();

                let attrs = if let Some(props) = node_data.get("props").and_then(|v| v.as_object())
                {
                    props
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                } else {
                    HashMap::new()
                };

                let children = if let Some(children_array) =
                    node_data.get("children").and_then(|v| v.as_array())
                {
                    children_array
                        .iter()
                        .filter_map(|v| v.as_str().and_then(|s| s.parse::<u32>().ok()))
                        .map(|child_id| build_node(child_id, map))
                        .collect::<std::result::Result<Vec<_>, _>>()?
                } else {
                    Vec::new()
                };

                Ok(VNode::Element(VElement {
                    tag,
                    attrs,
                    children,
                    key: None,
                }))
            }
            "text" => {
                let content = node_data
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Text VNode missing content")?
                    .to_string();

                Ok(VNode::Text(content))
            }
            "comment" => {
                let content = node_data
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Comment VNode missing content")?
                    .to_string();

                Ok(VNode::Comment(content))
            }
            _ => Err(format!("Unknown VNode type: {}", node_type)),
        }
    }

    let root_node = build_node(root_id, &map)?;

    Ok(VTree { root: root_node })
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

    #[test]
    fn test_inject_render_helpers() {
        let mut runtime = JsRuntime::new();
        let result = inject_render_helpers(&mut runtime);
        assert!(result.is_ok());

        // 验证 helper 函数存在
        let result = runtime.eval("typeof h === 'function' && typeof text === 'function' && typeof comment === 'function'");
        assert!(result.is_ok());
        assert!(result.unwrap().as_boolean().unwrap_or(false));
    }

    #[test]
    fn test_execute_simple_render_function() {
        let mut runtime = JsRuntime::new();
        inject_render_helpers(&mut runtime).unwrap();

        // 定义一个简单的 render 函数
        let render_code = r#"
function render() {
    return h('div', { class: 'container' }, [
        h('h1', null, [text('Hello, Iris!')]),
        h('p', null, [text('This is a test')])
    ]);
}
"#;

        let result = execute_render_function(&mut runtime, render_code);
        if let Err(ref e) = result {
            eprintln!("Execute render error: {:?}", e);
        }
        assert!(result.is_ok());

        let vtree = result.unwrap();
        
        // 验证 VTree 已创建
        match &vtree.root {
            VNode::Element(elem) => {
                eprintln!("Element tag: {}, children count: {}", elem.tag, elem.children.len());
                assert_eq!(elem.tag, "div");
                assert_eq!(elem.attrs.get("class"), Some(&"container".to_string()));
                assert_eq!(elem.children.len(), 2);
            }
            _ => panic!("Expected Element node"),
        }
    }

    #[test]
    fn test_execute_render_with_text_nodes() {
        let mut runtime = JsRuntime::new();
        inject_render_helpers(&mut runtime).unwrap();

        let render_code = r#"
function render() {
    return text('Simple text content');
}
"#;

        let result = execute_render_function(&mut runtime, render_code);
        assert!(result.is_ok());

        let vtree = result.unwrap();
        
        match &vtree.root {
            VNode::Text(content) => {
                assert_eq!(content, "Simple text content");
            }
            _ => panic!("Expected Text node"),
        }
    }

    #[test]
    fn test_vnode_registry_build_tree() {
        let mut registry = VNodeRegistry::new();

        // 创建子节点
        let text_id = registry.create_text("Hello");
        let child_id = registry.create_element("span", None, vec![text_id]);
        
        // 创建根节点
        let root_id = registry.create_element(
            "div",
            Some(vec![("class".to_string(), "container".to_string())].into_iter().collect()),
            vec![child_id],
        );

        // 构建树
        let vtree = registry.build_tree(root_id);
        assert!(vtree.is_some());

        let vtree = vtree.unwrap();
        
        match &vtree.root {
            VNode::Element(elem) => {
                assert_eq!(elem.tag, "div");
                assert_eq!(elem.children.len(), 1);
                
                // 验证子节点
                if let VNode::Element(child_elem) = &elem.children[0] {
                    assert_eq!(child_elem.tag, "span");
                    assert_eq!(child_elem.children.len(), 1);
                    
                    // 验证文本节点
                    if let VNode::Text(content) = &child_elem.children[0] {
                        assert_eq!(content, "Hello");
                    } else {
                        panic!("Expected Text node");
                    }
                } else {
                    panic!("Expected Element node");
                }
            }
            _ => panic!("Expected Element node"),
        }
    }
}
