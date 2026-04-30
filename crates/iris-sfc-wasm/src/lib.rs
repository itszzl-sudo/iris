use wasm_bindgen::prelude::*;

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

/// 简单的模板到 render 函数（占位实现）
fn template_to_render(template: &str) -> String {
    let inner = template.trim();
    if inner.is_empty() {
        return String::new();
    }

    // 生成一个简单的 render 函数
    let mut html = String::new();
    let mut buf = String::new();
    let mut depth: i32 = 0;
    let mut in_tag = false;
    let mut tag_name = String::new();
    let mut attrs = String::new();
    let mut in_attrs = false;
    let mut self_closing = false;

    for ch in inner.chars() {
        match ch {
            '<' => {
                if !buf.trim().is_empty() {
                    // 文本节点
                    let text = buf.trim();
                    if !text.is_empty() {
                        if depth > 0 {
                            html.push_str(&"  ".repeat(depth as usize));
                        }
                        html.push_str(&format!("h('{}', {{}}, '{}'),\n", "span", text));
                    }
                    buf.clear();
                }
                in_tag = true;
                tag_name.clear();
                attrs.clear();
                in_attrs = false;
                self_closing = false;
            }
            '>' if in_tag => {
                in_tag = false;
                if self_closing || tag_name.starts_with('/') {
                    depth -= 1;
                } else {
                    if depth > 0 {
                        html.push_str(&"  ".repeat(depth as usize));
                    }
                    html.push_str(&format!("h('{}', {}),\n", tag_name, attrs));
                    if !self_closing {
                        depth += 1;
                    }
                }
                buf.clear();
            }
            '/' if in_tag && tag_name.is_empty() => {
                // comment or end tag
                // peek
                continue;
            }
            ' ' if in_tag && tag_name.is_empty() => {}
            ' ' if in_tag && !tag_name.is_empty() && !in_attrs => {
                in_attrs = true;
            }
            '/' if in_tag && in_attrs => {
                self_closing = true;
            }
            _ if in_tag => {
                if in_attrs {
                    attrs.push(ch);
                } else {
                    tag_name.push(ch);
                }
            }
            _ => {
                buf.push(ch);
            }
        }
    }

    format!("function() {{ const h = arguments[0]; return [\n{}] }}", html)
}
