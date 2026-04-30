use wasm_bindgen::prelude::*;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;

/// 编译结果
#[derive(serde::Serialize)]
struct CompileResult {
    script: String,
    styles: Vec<StyleBlock>,
    deps: Vec<String>,
    source_map: Option<String>,
}

/// 样式块
#[derive(serde::Serialize)]
struct StyleBlock {
    code: String,
    scoped: bool,
    lang: String,
}

/// 编译 Vue SFC 源码
/// 返回 JSON 字符串
#[wasm_bindgen(js_name = compileSfc)]
pub fn compile_sfc(source: &str, filename: &str) -> Result<String, JsError> {
    let _ = filename;
    tracing::debug!("compileSfc: {} ({} bytes)", filename, source.len());

    let script = extract_script(source);
    let styles = extract_styles(source);
    let template = extract_template(source);

    // 简单模板到 render 函数转换
    let render_fn = if let Some(tpl) = &template {
        template_to_render(tpl)
    } else {
        String::new()
    };

    // 构建最终的脚本
    let final_script = if script.is_some() || !render_fn.is_empty() {
        let mut output = String::new();

        // script block
        if let Some(s) = &script {
            // inject render function
            if !render_fn.is_empty() {
                if s.contains("export default") {
                    output.push_str(&s.replace("export default {",
                        &format!("export default {{\n  render: {},", render_fn)));
                } else {
                    output.push_str(s);
                    output.push_str(&format!("\n\nconst __sfc__ = {{ render: {} }};\nexport default __sfc__;", render_fn));
                }
            } else {
                output.push_str(s);
            }
        } else if !render_fn.is_empty() {
            output.push_str(&format!(
                "export default {{ render: {} }};", render_fn));
        }

        output
    } else {
        source.to_string()
    };

    let result = CompileResult {
        script: final_script,
        styles,
        deps: Vec::new(),
        source_map: None,
    };

    serde_json::to_string(&result)
        .map_err(|e| JsError::new(&format!("JSON serialization failed: {}", e)))
}

/// 清除缓存
#[wasm_bindgen(js_name = clearCache)]
pub fn clear_cache() {
    tracing::debug!("Cache cleared");
}

/// 编译 SCSS
fn compile_scss(code: &str) -> String {
    match grass::from_string(code.to_string(), &grass::Options::default()) {
        Ok(css) => css,
        Err(e) => {
            tracing::warn!("SCSS compile error: {}", e);
            code.to_string()
        }
    }
}

fn extract_script(source: &str) -> Option<String> {
    let start = source.find("<script")?;
    let after_tag = &source[start..];
    let content_start = after_tag.find('>')? + start + 1;
    let remaining = &source[content_start..];
    let end = remaining.find("</script>")?;
    let script = remaining[..end].trim().to_string();

    // 检查是否有 setup 属性
    if after_tag.contains("setup") {
        Some(script)
    } else {
        Some(script)
    }
}

fn extract_template(source: &str) -> Option<String> {
    let start = source.find("<template")?;
    let after_tag = &source[start..];
    let content_start = after_tag.find('>')? + start + 1;
    let remaining = &source[content_start..];
    let end = remaining.find("</template>")?;
    Some(remaining[..end].trim().to_string())
}

fn extract_styles(source: &str) -> Vec<StyleBlock> {
    let mut styles = Vec::new();
    let mut search_from = 0;

    loop {
        let start = match source[search_from..].find("<style") {
            Some(s) => search_from + s,
            None => break,
        };

        let after_tag = &source[start..];
        let scoped = after_tag.contains("scoped");
        let lang = if after_tag.contains("lang=\"scss\"") || after_tag.contains("lang='scss'") {
            "scss".to_string()
        } else {
            "css".to_string()
        };

        let content_start = match after_tag.find('>') {
            Some(s) => start + s + 1,
            None => break,
        };

        let remaining = &source[content_start..];
        let end = match remaining.find("</style>") {
            Some(e) => e,
            None => break,
        };

        let code = remaining[..end].trim().to_string();

        // 编译 SCSS
        let final_code = if lang == "scss" {
            compile_scss(&code)
        } else {
            code
        };

        styles.push(StyleBlock {
            code: final_code,
            scoped,
            lang,
        });

        search_from = content_start + end + 8;
    }

    styles
}

/// 将 HTML 模板编译为 Vue 3 render 函数
fn template_to_render(template: &str) -> String {
    let inner = template.trim();
    if inner.is_empty() {
        return String::new();
    }

    // 使用 html5ever 解析 HTML
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut inner.as_bytes())
        .unwrap();

    // 获取 <body> 的直接子节点（即模板的实际内容）
    let body_children = get_body_children(&dom.document);
    if body_children.is_empty() {
        return String::new();
    }

    // 将每个顶级元素转换为 vnode
    let vnodes: Vec<String> = body_children
        .iter()
        .filter_map(|child| {
            let v = element_to_vnode(child, 1);
            if v.is_empty() { None } else { Some(v) }
        })
        .collect();

    if vnodes.is_empty() {
        return String::new();
    }

    if vnodes.len() == 1 {
        // 单根节点：直接返回
        format!(
            "function() {{ const h = arguments[0]; return {};\n}}",
            vnodes[0]
        )
    } else {
        // 多根节点：返回数组
        let joined = vnodes.join(&format!(",\n"));
        format!(
            "function() {{ const h = arguments[0]; return [\n{}];\n}}",
            joined
        )
    }
}

/// 从解析后的 DOM 中获取 <body> 的直接子节点
fn get_body_children(handle: &Handle) -> Vec<Handle> {
    let doc_children = handle.children.borrow();
    for child in doc_children.iter() {
        if let NodeData::Element { ref name, .. } = child.data {
            if name.local.as_ref() == "html" {
                let html_children = child.children.borrow();
                for html_child in html_children.iter() {
                    if let NodeData::Element { ref name, .. } = html_child.data {
                        if name.local.as_ref() == "body" {
                            return html_child.children.borrow().clone();
                        }
                    }
                }
            }
        }
    }
    Vec::new()
}

/// JavaScript 字符串字面量转义
fn escape_js_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('\'');
    for ch in s.chars() {
        match ch {
            '\'' => result.push_str("\\'"),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x00'..='\x1f' => {
                for b in ch.encode_utf8(&mut [0u8; 4])[..ch.len_utf8()].as_bytes().iter() {
                    result.push_str(&format!("\\x{:02x}", b));
                }
            }
            c => result.push(c),
        }
    }
    result.push('\'');
    result
}

/// 判断文本节点是否只包含空白
fn is_whitespace_only(s: &str) -> bool {
    s.chars().all(|c| c.is_whitespace())
}

/// 将属性列表转换为 JS 对象字符串
fn attrs_to_object(attrs: &[html5ever::Attribute]) -> String {
    if attrs.is_empty() {
        return "null".to_string();
    }

    let mut entries: Vec<String> = Vec::new();

    for attr in attrs.iter() {
        let name = attr.name.local.as_ref();
        let value_str = attr.value.to_string();

        if name.starts_with(':') || name.starts_with("v-bind:") {
            // :prop="expr" 或 v-bind:prop="expr" -> prop: expr (JS 表达式，不引号)
            let prop = name.trim_start_matches(':').trim_start_matches("v-bind:");
            if prop.contains('-') {
                entries.push(format!("'{}': {}", prop, value_str));
            } else {
                entries.push(format!("{}: {}", prop, value_str));
            }
        } else if name.starts_with('@') || name.starts_with("v-on:") {
            // @click="handler" -> onClick: handler (camelCase)
            let event_part = name.trim_start_matches('@').trim_start_matches("v-on:");
            let mut js_name = String::from("on");
            if let Some(c) = event_part.chars().next() {
                js_name.push(c.to_uppercase().next().unwrap_or(c));
                js_name.push_str(&event_part[1..]);
            }
            // 修饰符（如 .enter, .prevent）导致 key 含点号，需引号
            if js_name.contains('.') {
                entries.push(format!("'{}': {}", js_name, value_str));
            } else {
                entries.push(format!("{}: {}", js_name, value_str));
            }
        } else if name.starts_with("v-") {
            // v-if, v-show, v-for, v-model 等 — render 函数 props 中无效，跳过
            continue;
        } else {
            // 静态属性
            let escaped = escape_js_string(&value_str);
            if name.contains('-') {
                entries.push(format!("'{}': {}", name, escaped));
            } else {
                entries.push(format!("{}: {}", name, escaped));
            }
        }
    }

    if entries.is_empty() {
        "null".to_string()
    } else {
        format!("{{ {} }}", entries.join(", "))
    }
}

/// 递归将 DOM 节点转换为 h() 调用
fn element_to_vnode(node: &Handle, indent: usize) -> String {
    match &node.data {
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let tag = name.local.as_ref();
            let attrs_obj = attrs_to_object(&attrs.borrow());
            let children = node.children.borrow();

            // 收集非空白子节点
            let vnodes: Vec<String> = children
                .iter()
                .filter_map(|child| {
                    let v = element_to_vnode(child, indent + 1);
                    if v.is_empty() { None } else { Some(v) }
                })
                .collect();

            if vnodes.is_empty() {
                // 无子节点
                format!("h('{}', {})", tag, attrs_obj)
            } else if vnodes.len() == 1
                && matches!(&children[0].data, NodeData::Text { .. })
            {
                // 单文本子节点
                format!("h('{}', {}, {})", tag, attrs_obj, vnodes[0])
            } else {
                // 多个子节点（或元素子节点）
                let pad = "  ".repeat(indent);
                let inner_pad = "  ".repeat(indent + 1);
                let inner = vnodes.join(&format!(",\n{}", inner_pad));
                format!(
                    "h('{}', {}, [\n{}  {}\n{}])",
                    tag, attrs_obj, pad, inner, pad
                )
            }
        }
        NodeData::Text { ref contents } => {
            let text = contents.borrow();
            if is_whitespace_only(&text) {
                String::new()
            } else {
                escape_js_string(&text.trim())
            }
        }
        _ => String::new(),
    }
}
