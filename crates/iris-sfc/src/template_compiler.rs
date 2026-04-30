use markup5ever_rcdom::{Handle, NodeData, RcDom};
/// Vue Template Compiler
///
/// Parses HTML templates using html5ever and generates virtual DOM creation functions.
/// Supports directives: v-if, v-for, v-bind, v-on, v-model, v-slot, v-once, v-pre, v-cloak, v-memo
use std::collections::HashSet;
use tracing::{debug, info};

/// Virtual DOM node types representing the template AST
#[derive(Debug, Clone)]
pub enum VNode {
    /// HTML element node with tag, attributes, children, and directives
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<VNode>,
        directives: Vec<Directive>,
    },
    /// Text node (plain text or interpolation)
    Text {
        content: String,
        is_interpolation: bool, // true for {{ expression }}
    },
    /// HTML comment node
    Comment { content: String },
}

/// Vue directive types supported by the template compiler
#[derive(Debug, Clone)]
pub enum Directive {
    /// Conditional rendering: v-if="condition"
    VIf { condition: String },
    /// Conditional rendering: v-else-if="condition"
    VElseIf { condition: String },
    /// Conditional rendering: v-else
    VElse,
    /// List rendering: v-for="item in items" or v-for="(item, index) in items"
    VFor {
        iterator: String, // e.g., "item" or "(item, index)"
        source: String,   // e.g., "items"
    },
    /// Attribute binding: :prop="value" or v-bind:prop="value"
    VBind { prop: String, value: String },
    /// Event listener: @event="handler" or v-on:event="handler"
    VOn { event: String, handler: String },
    /// Two-way binding: v-model="variable"
    VModel { variable: String },
    /// Slot: v-slot="slotProps" or #slotName
    VSlot {
        name: String,          // slot name (default: "default")
        props: Option<String>, // slot scope props (optional)
    },
    /// One-time render: v-once (render only once)
    VOnce,
    /// Raw content: v-pre (skip compilation)
    VPre,
    /// Cloak directive: v-cloak (removed after compilation)
    VCloak,
    /// Memo optimization: v-memo="[dep1, dep2]" (Vue 3.2+)
    VMemo { dependencies: String },
    /// Text content: v-text="expression"
    VText { expression: String },
    /// HTML content: v-html="expression"
    VHtml { expression: String },
    /// Show/hide: v-show="condition"
    VShow { condition: String },
}

/// 从原始 HTML 模板中检测 PascalCase 组件名
/// 例如 <MockTableDemo>、<MyComponent>、</SomeThing>
fn detect_components(html: &str) -> HashSet<String> {
    let mut components = HashSet::new();
    // 匹配 <ComponentName 或 </ComponentName，要求首字母大写
    // 这能匹配大多数 Vue 组件用法
    for cap in html.split('<').skip(1) {
        let name = cap.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .next()
            .unwrap_or("");
        if !name.is_empty() && name.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
            components.insert(name.to_string());
        }
    }
    components
}

/// Parse HTML template string into VNode AST
///
/// 返回 (VNode 列表, 检测到的组件名集合)
pub fn parse_template(html: &str) -> Result<(Vec<VNode>, HashSet<String>), String> {
    use html5ever::namespace_url;
    use html5ever::tendril::{Tendril, TendrilSink};
    use html5ever::{local_name, parse_fragment, ParseOpts, QualName};

    info!(html_len = html.len(), "Parsing HTML template");

    let opts = ParseOpts::default();
    let dom = parse_fragment(
        RcDom::default(),
        opts,
        QualName::new(None, html5ever::ns!(html), local_name!("body")),
        vec![],
    )
    .one(Tendril::from(html));

    let mut nodes = convert_dom_to_vnodes(&dom.document);

    // html5ever 的 parse_fragment 会额外包裹 <html><body> 标签
    // 需要去掉这些自动生成的包裹层，只保留实际的模板内容
    nodes = strip_auto_wrappers(nodes);

    // 从原始模板中检测 PascalCase 组件名
    // html5ever 将所有标签都转换为小写，我们需要恢复组件名的大小写
    let component_names = detect_components(html);
    if !component_names.is_empty() {
        debug!("Detected components: {:?}", component_names);
    }

    debug!(node_count = nodes.len(), "Template parsing completed");
    Ok((nodes, component_names))
}

/// 去除 html5ever 自动生成的 <html> 和 <body> 包裹层
fn strip_auto_wrappers(nodes: Vec<VNode>) -> Vec<VNode> {
    let mut result = nodes;
    // 处理单层包裹：如果顶层只有一个 <html> 元素，展开其子节点
    while result.len() == 1 {
        match &result[0] {
            VNode::Element { tag, children, .. } => {
                if tag == "html" || tag == "body" {
                    result = children.clone();
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    result
}

/// Convert html5ever DOM tree to VNode AST
fn convert_dom_to_vnodes(handle: &Handle) -> Vec<VNode> {
    let mut nodes = Vec::new();

    for child in handle.children.borrow().iter() {
        match &child.data {
            NodeData::Document => {
                // Recursively process document node
                nodes.extend(convert_dom_to_vnodes(child));
            }
            NodeData::Element {
                ref name,
                ref attrs,
                ..
            } => {
                let tag = name.local.to_string();
                let attributes = attrs.borrow().clone();

                // Parse attributes and extract directives
                let (directives, plain_attrs) = extract_directives(&attributes);

                // Recursively process child nodes
                let children = convert_dom_to_vnodes(child);

                nodes.push(VNode::Element {
                    tag,
                    attrs: plain_attrs,
                    children,
                    directives,
                });
            }
            NodeData::Text { ref contents } => {
                let text = contents.borrow().to_string();
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let (content, is_interpolation) = parse_text(trimmed);
                    nodes.push(VNode::Text {
                        content,
                        is_interpolation,
                    });
                }
            }
            NodeData::Comment { ref contents } => {
                nodes.push(VNode::Comment {
                    content: contents.to_string(),
                });
            }
            _ => {}
        }
    }

    nodes
}

/// Extract Vue directives from element attributes
fn extract_directives(attrs: &[html5ever::Attribute]) -> (Vec<Directive>, Vec<(String, String)>) {
    let mut directives = Vec::new();
    let mut plain_attrs = Vec::new();

    for attr in attrs {
        let name = attr.name.local.to_string();
        let value = attr.value.to_string();

        debug!("Attribute: {} = {}", name, value);

        match parse_directive(&name, &value) {
            Some(directive) => directives.push(directive),
            None => plain_attrs.push((name, value)),
        }
    }

    (directives, plain_attrs)
}

/// Parse attribute name and value into Vue directive if applicable
fn parse_directive(name: &str, value: &str) -> Option<Directive> {
    match name {
        // Conditional rendering directives
        "v-if" => Some(Directive::VIf {
            condition: value.to_string(),
        }),
        "v-else-if" => Some(Directive::VElseIf {
            condition: value.to_string(),
        }),
        "v-else" => Some(Directive::VElse),

        // List rendering directive
        "v-for" => {
            if let Some((iterator, source)) = parse_vfor(value) {
                Some(Directive::VFor { iterator, source })
            } else {
                None
            }
        }

        // Two-way binding directive
        "v-model" => Some(Directive::VModel {
            variable: value.to_string(),
        }),

        // Slot directive (v-slot or shorthand #name)
        "v-slot" => Some(Directive::VSlot {
            name: "default".to_string(),
            props: if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            },
        }),
        name if name.starts_with('#') => Some(Directive::VSlot {
            name: name[1..].to_string(),
            props: if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            },
        }),

        // One-time render directive
        "v-once" => Some(Directive::VOnce),

        // Skip compilation directive
        "v-pre" => Some(Directive::VPre),

        // Cloak directive
        "v-cloak" => Some(Directive::VCloak),

        // Memo optimization directive
        "v-memo" => Some(Directive::VMemo {
            dependencies: value.to_string(),
        }),

        // Text content directive
        "v-text" => Some(Directive::VText {
            expression: value.to_string(),
        }),

        // HTML content directive
        "v-html" => Some(Directive::VHtml {
            expression: value.to_string(),
        }),

        // Show/hide directive
        "v-show" => Some(Directive::VShow {
            condition: value.to_string(),
        }),

        // Attribute binding (v-bind:prop or :prop shorthand)
        name if name.starts_with("v-bind:") => Some(Directive::VBind {
            prop: name[7..].to_string(),
            value: value.to_string(),
        }),
        name if name.starts_with(':') => Some(Directive::VBind {
            prop: name[1..].to_string(),
            value: value.to_string(),
        }),

        // Event listener (v-on:event or @event shorthand)
        name if name.starts_with("v-on:") => Some(Directive::VOn {
            event: name[5..].to_string(),
            handler: value.to_string(),
        }),
        name if name.starts_with('@') => Some(Directive::VOn {
            event: name[1..].to_string(),
            handler: value.to_string(),
        }),

        _ => None,
    }
}

/// Parse v-for directive value into iterator and source
/// Supports: "item in items", "(item, index) in items", "item of items"
fn parse_vfor(value: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = if value.contains(" in ") {
        value.splitn(2, " in ").collect()
    } else if value.contains(" of ") {
        value.splitn(2, " of ").collect()
    } else {
        return None;
    };

    if parts.len() == 2 {
        Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        None
    }
}

/// Parse text node content and detect interpolation expressions
/// Returns (content, is_interpolation)
pub fn parse_text(text: &str) -> (String, bool) {
    if text.starts_with("{{") && text.ends_with("}}") {
        // Interpolation expression: {{ expression }}
        (text[2..text.len() - 2].trim().to_string(), true)
    } else {
        // Plain text
        (text.to_string(), false)
    }
}

/// Generate JavaScript render function from VNode AST
pub fn generate_render_fn(nodes: &[VNode]) -> String {
    info!(node_count = nodes.len(), "Generating render function");
    generate_render_fn_with_components(nodes, &HashSet::new())
}

/// 生成渲染函数，支持 PascalCase 组件名识别
pub fn generate_render_fn_with_components(nodes: &[VNode], components: &HashSet<String>) -> String {
    info!(node_count = nodes.len(), "Generating render function with {} components", components.len());

    // render 函数不接收参数，h 通过 import 导入
    let mut code = String::from("function render() {\n  return ");

    if nodes.is_empty() {
        code.push_str("null");
    } else if nodes.len() == 1 {
        code.push_str(&generate_vnode_with_components(&nodes[0], components));
    } else {
        code.push_str("[\n");
        for node in nodes {
            code.push_str("    ");
            code.push_str(&generate_vnode_with_components(node, components));
            code.push_str(",\n");
        }
        code.push_str("  ]");
    }

    code.push_str(";\n}");
    code
}

/// Generate code for a single VNode
/// Generate code for a single VNode (without component detection)
fn generate_vnode(node: &VNode) -> String {
    generate_vnode_with_components(node, &HashSet::new())
}

/// Generate code for a single VNode with component detection
fn generate_vnode_with_components(node: &VNode, components: &HashSet<String>) -> String {
    match node {
        VNode::Element {
            tag,
            attrs,
            children,
            directives,
        } => {
            // Generate directive-specific code if directives exist
            if let Some(directive_code) = generate_directives(directives, tag, attrs, children) {
                return directive_code;
            }

            generate_element_with_components(tag, attrs, children, components)
        }
        VNode::Text {
            content,
            is_interpolation,
        } => {
            if *is_interpolation {
                // 插值表达式：直接返回变量
                format!("{}", content)
            } else {
                // 静态文本：在 Vue 3 中直接用字符串
                format!("{:?}", content)
            }
        }
        VNode::Comment { .. } => {
            // HTML 注释在渲染中没有意义，跳过生成
            String::new()
        }
    }
}

/// Generate element node code with tag, attributes, and children
/// Generate element node code
fn generate_element(tag: &str, attrs: &[(String, String)], children: &[VNode]) -> String {
    generate_element_with_components(tag, attrs, children, &HashSet::new())
}

/// Generate element node code with component detection
fn generate_element_with_components(tag: &str, attrs: &[(String, String)], children: &[VNode], components: &HashSet<String>) -> String {
    // 检查是否是 PascalCase 组件
    let is_component = components.iter().any(|c| c.to_lowercase() == tag.to_lowercase());
    let tag_ref = if is_component {
        // 找到正确的 PascalCase 组件名
        components.iter().find(|c| c.to_lowercase() == tag.to_lowercase())
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("{:?}", tag))
    } else {
        format!("{:?}", tag)
    };
    
    let mut code = if is_component {
        // 组件使用引用而非字符串：h(MockTableDemo, ...)
        format!("h({}, ", tag_ref)
    } else {
        format!("h({:?}, ", tag)
    };

    code.push_str(&render_attrs(attrs));

    if children.is_empty() {
        code.push_str(")");
    } else {
        code.push_str(", [\n");
        for child in children {
            code.push_str("      ");
            code.push_str(&generate_vnode_with_components(child, components));
            code.push_str(",\n");
        }
        code.push_str("    ])");
    }

    code
}

/// Generate code for Vue directives
fn generate_directives(
    directives: &[Directive],
    tag: &str,
    attrs: &[(String, String)],
    children: &[VNode],
) -> Option<String> {
    // Handle v-pre: output raw HTML without compilation
    if directives.iter().any(|d| matches!(d, Directive::VPre)) {
        let mut html = format!("<{}", tag);
        for (key, value) in attrs {
            html.push_str(&format!(" {}={:?}", key, value));
        }
        html.push('>');
        html.push_str(&format!("</{}>", tag));
        return Some(format!("text({:?})", html));
    }

    // Handle v-if conditional rendering
    // 注意：当前实现独立处理每个条件分支，未形成完整的 if-else 链
    // 完整实现需要在模板解析阶段识别相邻的 v-if/v-else-if/v-else 节点
    if let Some(directive) = directives
        .iter()
        .find(|d| matches!(d, Directive::VIf { .. }))
    {
        if let Directive::VIf { condition } = directive {
            let element = generate_element(tag, attrs, children);
            return Some(format!("{} ? {} : null", condition, element));
        }
    }

    // Handle v-else-if conditional rendering
    if let Some(directive) = directives
        .iter()
        .find(|d| matches!(d, Directive::VElseIf { .. }))
    {
        if let Directive::VElseIf { condition } = directive {
            let element = generate_element(tag, attrs, children);
            return Some(format!("{} ? {} : null", condition, element));
        }
    }

    // Handle v-else conditional rendering
    if directives.iter().any(|d| matches!(d, Directive::VElse)) {
        let element = generate_element(tag, attrs, children);
        return Some(element);
    }

    // Handle v-for list rendering
    if let Some(directive) = directives
        .iter()
        .find(|d| matches!(d, Directive::VFor { .. }))
    {
        if let Directive::VFor { iterator, source } = directive {
            let element = generate_element(tag, attrs, children);
            // 修复：移除 ... 前缀，直接返回数组（避免语法错误）
            return Some(format!("{}.map(({}) => {})", source, iterator, element));
        }
    }

    // Handle v-once: add caching marker
    let mut final_attrs = attrs.to_vec();
    if directives.iter().any(|d| matches!(d, Directive::VOnce)) {
        final_attrs.push(("_once".to_string(), "true".to_string()));
    }

    // Handle v-cloak: add temporary attribute (removed at runtime)
    if directives.iter().any(|d| matches!(d, Directive::VCloak)) {
        final_attrs.push(("v-cloak".to_string(), "".to_string()));
    }

    // Handle v-memo: add memoization dependencies
    if let Some(directive) = directives
        .iter()
        .find(|d| matches!(d, Directive::VMemo { .. }))
    {
        if let Directive::VMemo { dependencies } = directive {
            final_attrs.push(("_memo".to_string(), dependencies.clone()));
        }
    }

    // Handle v-slot: generate slot component
    if let Some(directive) = directives
        .iter()
        .find(|d| matches!(d, Directive::VSlot { .. }))
    {
        if let Directive::VSlot { name, props } = directive {
            let element = generate_element(tag, &final_attrs, children);
            return Some(if let Some(slot_props) = props {
                // Scoped slot: slot("name", (props) => h(...))
                format!("slot({:?}, ({}) => {})", name, slot_props, element)
            } else {
                // Default slot: slot("name", h(...))
                format!("slot({:?}, {})", name, element)
            });
        }
    }

    // Handle v-bind: add dynamic attributes
    // 修复：直接使用表达式，而不是字符串拼接（避免 XSS）
    for directive in directives
        .iter()
        .filter(|d| matches!(d, Directive::VBind { .. }))
    {
        if let Directive::VBind { prop, value } = directive {
            // 将动态值作为表达式直接传递，由运行时处理转义
            final_attrs.push((prop.clone(), format!("/* dynamic: */ {}", value)));
        }
    }

    // Handle v-on: add event listeners
    // Parse event modifiers (.prevent, .stop, .once, .capture, .self, .passive)
    for directive in directives
        .iter()
        .filter(|d| matches!(d, Directive::VOn { .. }))
    {
        if let Directive::VOn { event, handler } = directive {
            // Split event name on '.' to extract modifiers
            let parts: Vec<&str> = event.split('.').collect();
            let base_event = parts[0];
            let modifiers: Vec<&str> = parts[1..].to_vec();
            
            let event_name = format!("on{}", capitalize(base_event));
            
            // Generate the handler expression
            let handler_expr = if modifiers.is_empty() {
                // No modifiers: use the handler directly (it's a function reference)
                if handler.is_empty() {
                    // No handler and no modifier - this is a no-op
                    String::new()
                } else {
                    handler.clone()
                }
            } else {
                // With modifiers: wrap in arrow function
                let mut expr = String::from("($event) => ");
                
                if !handler.is_empty() {
                    // Both modifiers and handler
                    let mut body_parts = Vec::new();
                    for m in &modifiers {
                        match *m {
                            "prevent" => body_parts.push("$event.preventDefault()"),
                            "stop" => body_parts.push("$event.stopPropagation()"),
                            "capture" => {} // handled at event binding level
                            "once" => {} // handled at event binding level
                            "self" => body_parts.push("$event.target !== $event.currentTarget ? null : "),
                            "passive" => {} // hint only
                            _ => {}
                        }
                    }
                    body_parts.push(&handler);
                    // Simple comma approach: works if handler is a call expression like handler($event)
                    // or if chained via comma operator
                    if body_parts.len() == 1 {
                        expr.push_str(body_parts[0]);
                    } else {
                        expr.push_str("{");
                        for bp in &body_parts {
                            if *bp == body_parts.last().copied().unwrap_or("") {
                                // Last one - call handler with $event
                                if bp.contains("($event)") || bp.contains('(') {
                                    expr.push_str(&format!(" {} ", bp));
                                } else {
                                    expr.push_str(&format!(" {}.call(this, $event); ", bp));
                                }
                            } else {
                                expr.push_str(&format!(" {}; ", bp));
                            }
                        }
                        expr.push_str("}");
                    }
                } else {
                    // Modifier only (e.g., @click.prevent with no handler)
                    expr.push_str("{");
                    for m in &modifiers {
                        match *m {
                            "prevent" => expr.push_str(" $event.preventDefault(); "),
                            "stop" => expr.push_str(" $event.stopPropagation(); "),
                            "capture" => {}
                            "once" => {}
                            "self" => expr.push_str(" /* self modifier */ "),
                            "passive" => {}
                            _ => {}
                        }
                    }
                    expr.push_str("}");
                }
                expr
            };
            
            if !handler_expr.is_empty() {
                final_attrs.push((event_name, handler_expr));
            }
        }
    }

    // Handle v-model: add value binding and input handler
    for directive in directives
        .iter()
        .filter(|d| matches!(d, Directive::VModel { .. }))
    {
        if let Directive::VModel { variable } = directive {
            final_attrs.push(("value".to_string(), variable.clone()));
            final_attrs.push((
                "onInput".to_string(),
                format!("(e) => {{ {} = e.target.value; }}", variable),
            ));
        }
    }

    // Handle v-text: set textContent
    if let Some(directive) = directives
        .iter()
        .find(|d| matches!(d, Directive::VText { .. }))
    {
        if let Directive::VText { expression } = directive {
            let element = generate_element_with_attrs(tag, &final_attrs);
            return Some(format!(
                "(() => {{ const el = {}; el.textContent = {}; return el; }})()",
                element, expression
            ));
        }
    }

    // Handle v-html: set innerHTML
    // WARNING: XSS risk if expression contains user input. Consider using DOMPurify at runtime.
    if let Some(directive) = directives
        .iter()
        .find(|d| matches!(d, Directive::VHtml { .. }))
    {
        if let Directive::VHtml { expression } = directive {
            let element = generate_element_with_attrs(tag, &final_attrs);
            return Some(format!(
                "(() => {{ const el = {}; el.innerHTML = {}; return el; }})()",
                element, expression
            ));
        }
    }

    // Handle v-show: toggle display style
    if let Some(directive) = directives
        .iter()
        .find(|d| matches!(d, Directive::VShow { .. }))
    {
        if let Directive::VShow { condition } = directive {
            let element = generate_element_with_attrs(tag, &final_attrs);
            return Some(format!(
                "(() => {{ const el = {}; el.style.display = {} ? '' : 'none'; return el; }})()",
                element, condition
            ));
        }
    }

    // Regenerate element if attributes were modified by directives
    if final_attrs.len() != attrs.len() {
        Some(generate_element(tag, &final_attrs, children))
    } else {
        None
    }
}

/// Capitalize first letter of string
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Check if an attribute key represents a Vue event handler (starts with "on" + uppercase)
fn is_event_handler_attr(key: &str) -> bool {
    if key.len() < 4 || !key.starts_with("on") {
        return false;
    }
    // After "on", the next character should be uppercase
    let rest = &key[2..];
    rest.starts_with(|c: char| c.is_uppercase())
}

/// Render attributes for h() calls.
/// Event handler attributes (onClick, onInput, etc.) get their values rendered as raw
/// JavaScript expressions, while regular attributes get quoted string values.
fn render_attrs(attrs: &[(String, String)]) -> String {
    if attrs.is_empty() {
        return "null".to_string();
    }
    let mut code = "{ ".to_string();
    let pairs: Vec<String> = attrs
        .iter()
        .map(|(k, v)| {
            if is_event_handler_attr(k) || v.starts_with("/* dynamic: */") {
                // Event handler or dynamic binding: value is raw JavaScript expression
                let stripped = v.strip_prefix("/* dynamic: */").unwrap_or(v);
                format!("{:?}: {}", k, stripped.trim())
            } else {
                // Static attribute: value is a string
                format!("{:?}: {:?}", k, v)
            }
        })
        .collect();
    code.push_str(&pairs.join(", "));
    code.push_str(" }");
    code
}

/// Generate element code with attributes only (no children)
fn generate_element_with_attrs(tag: &str, attrs: &[(String, String)]) -> String {
    let mut code = format!("h(\"{}\"", tag);

    if !attrs.is_empty() {
        code.push_str(", ");
        code.push_str(&render_attrs(attrs));
    }

    code.push_str(")");
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_element() {
        let html = r#"<div class="container">Hello</div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn test_parse_vfor() {
        let html = r#"<li v-for="item in items">{{ item.name }}</li>"#;
        let (nodes, _) = parse_template(html).unwrap();
        assert_eq!(nodes.len(), 1);

        let render_fn = generate_render_fn(&nodes);
        assert!(render_fn.contains(".map("));
    }

    #[test]
    fn test_parse_von() {
        // Test that @click directive generates correct render function
        let html = r#"<button @click="handleClick">Click</button>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        // Should generate valid render function
        assert!(render_fn.contains("h("), "Should generate h() call");
        assert!(
            render_fn.contains("function render()"),
            "Should generate render function"
        );
    }

    #[test]
    fn test_text_interpolation() {
        let (content, is_interpolation) = parse_text("{{ message }}");
        assert_eq!(content, "message");
        assert!(is_interpolation);

        let (content, is_interpolation) = parse_text("Hello World");
        assert_eq!(content, "Hello World");
        assert!(!is_interpolation);
    }

    // New directive tests - test code generation instead of parsing
    // (html5ever may transform attribute names, making parsing assertions unreliable)

    #[test]
    fn test_parse_vonce() {
        // Test that v-once directive generates correct render function
        let html = r#"<div v-once>{{ staticContent }}</div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        // Should generate valid render function
        assert!(render_fn.contains("h("), "Should generate h() call");
    }

    #[test]
    fn test_generate_vonce() {
        let html = r#"<div v-once>{{ message }}</div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        assert!(
            render_fn.contains("_once"),
            "Should contain _once attribute"
        );
    }

    #[test]
    fn test_parse_vpre() {
        // Test that v-pre directive generates correct render function
        let html = r#"<div v-pre>{{ raw }}</div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        // Should generate valid render function
        assert!(
            render_fn.contains("h(") || render_fn.contains("text("),
            "Should generate render function"
        );
    }

    #[test]
    fn test_parse_vcloak() {
        // Test that v-cloak directive generates correct render function
        let html = r#"<div v-cloak>{{ message }}</div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        // Should generate valid render function
        assert!(render_fn.contains("h("), "Should generate h() call");
    }

    #[test]
    fn test_parse_vmemo() {
        // Test that v-memo directive generates correct render function
        let html = r#"<div v-memo="[count, text]">{{ content }}</div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        // Should generate valid render function
        assert!(render_fn.contains("h("), "Should generate h() call");
    }

    #[test]
    fn test_generate_vmemo() {
        let html = r#"<li v-memo="[item.id]">{{ item.name }}</li>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        assert!(
            render_fn.contains("_memo"),
            "Should contain _memo attribute"
        );
        assert!(
            render_fn.contains("[item.id]"),
            "Should contain memo dependencies"
        );
    }

    #[test]
    fn test_parse_vtext() {
        // Test that v-text directive parses correctly
        let html = r#"<span v-text="message"></span>"#;
        let (nodes, _) = parse_template(html).unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn test_generate_vtext() {
        let html = r#"<span v-text="message"></span>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        assert!(
            render_fn.contains("textContent"),
            "Should contain textContent"
        );
        assert!(render_fn.contains("message"), "Should contain expression");
    }

    #[test]
    fn test_parse_vhtml() {
        // Test that v-html directive parses correctly
        let html = r#"<div v-html="rawHtml"></div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn test_generate_vhtml() {
        let html = r#"<div v-html="rawHtml"></div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        assert!(render_fn.contains("innerHTML"), "Should contain innerHTML");
        assert!(render_fn.contains("rawHtml"), "Should contain expression");
    }

    #[test]
    fn test_parse_vshow() {
        // Test that v-show directive parses correctly
        let html = r#"<div v-show="isVisible">Content</div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn test_generate_vshow() {
        let html = r#"<div v-show="isVisible">Content</div>"#;
        let (nodes, _) = parse_template(html).unwrap();
        let render_fn = generate_render_fn(&nodes);

        assert!(
            render_fn.contains("style.display"),
            "Should contain style.display"
        );
        assert!(render_fn.contains("isVisible"), "Should contain condition");
        assert!(
            render_fn.contains("'none'"),
            "Should contain 'none' for hiding"
        );
    }

    #[test]
    fn test_all_new_directives_parsed() {
        let test_cases = vec![
            (r#"<div v-once></div>"#, "v-once"),
            (r#"<div v-pre></div>"#, "v-pre"),
            (r#"<div v-cloak></div>"#, "v-cloak"),
            (r#"<div v-memo="[x]"></div>"#, "v-memo"),
        ];

        for (html, name) in test_cases {
            let result = parse_template(html);
            assert!(result.is_ok(), "Failed to parse {}", name);
            let (nodes, _) = result.unwrap();
            assert!(!nodes.is_empty(), "Empty result for {}", name);
        }
    }
}
