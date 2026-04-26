//! HTML 解析器
//!
//! 基于 html5ever 实现 HTML 字符串解析，生成 DOM 树。

use crate::dom::{DOMNode, DOMTree};
use html5ever::tendril::TendrilSink;
use html5ever::parse_document;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// 解析 HTML 字符串，生成 DOM 树
///
/// # 示例
///
/// ```rust
/// use iris_layout::html::parse_html;
///
/// let html = r#"
///     <div class="container">
///         <h1>Title</h1>
///         <p>Hello World</p>
///     </div>
/// "#;
///
/// let dom_tree = parse_html(html);
/// assert!(dom_tree.root().is_element());
/// ```
pub fn parse_html(html: &str) -> DOMTree {
    // 使用 html5ever 解析 HTML
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .expect("Failed to parse HTML");

    // 转换为我们的 DOM 树
    let root = convert_handle(&dom.document);
    DOMTree::new(root)
}

/// 将 html5ever 的 Handle 转换为我们的 DOMNode
fn convert_handle(handle: &Handle) -> DOMNode {
    match &handle.data {
        NodeData::Document => {
            // 文档节点，取第一个子节点作为根
            let children = handle.children.borrow();
            if let Some(first_child) = children.first() {
                convert_handle(first_child)
            } else {
                DOMNode::new_element("html")
            }
        }
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let tag_name = name.local.to_string();
            let mut node = DOMNode::new_element(&tag_name);
            
            // 转换属性
            let attrs_borrowed = attrs.borrow();
            for attr in attrs_borrowed.iter() {
                let key = attr.name.local.to_string();
                let value = attr.value.to_string();
                node.set_attribute(&key, &value);
            }
            
            // 递归处理子节点
            let children = handle.children.borrow();
            for child in children.iter() {
                let child_node = convert_handle(child);
                node.append_child(child_node);
            }
            
            node
        }
        NodeData::Text { ref contents } => {
            let text = contents.borrow().to_string();
            DOMNode::new_text(&text)
        }
        NodeData::Comment { ref contents } => {
            DOMNode::new_comment(contents)
        }
        NodeData::Doctype { .. } => {
            DOMNode::new_element("!doctype")
        }
        NodeData::ProcessingInstruction { .. } => {
            DOMNode::new_text("")
        }
    }
}

/// 从文件读取并解析 HTML
///
/// # 注意
///
/// 此函数是同步的，不适合在生产环境中使用。
/// 应该使用异步 I/O。
pub fn parse_html_file(path: &str) -> Result<DOMTree, std::io::Error> {
    let html = std::fs::read_to_string(path)?;
    Ok(parse_html(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let html = r#"<div>Hello</div>"#;
        let dom = parse_html(html);
        
        assert!(dom.root().is_element());
    }

    #[test]
    fn test_parse_with_attributes() {
        let html = r#"<div class="container" id="main">Content</div>"#;
        let dom = parse_html(html);
        
        // html5ever 会自动添加 <html> 和 <body>，所以需要查询
        let div = dom.query_selector("#main");
        assert!(div.is_some());
        assert_eq!(div.unwrap().class(), Some(&"container".to_string()));
    }

    #[test]
    fn test_parse_nested_elements() {
        let html = r#"
            <div>
                <p>Paragraph 1</p>
                <p>Paragraph 2</p>
            </div>
        "#;
        let dom = parse_html(html);
        
        assert!(dom.root().is_element());
        assert_eq!(dom.root().children.len(), 2);
    }

    #[test]
    fn test_parse_complex_html() {
        let html = r#"
            <!DOCTYPE html>
            <html>
                <head>
                    <title>Test</title>
                </head>
                <body>
                    <div class="container">
                        <h1>Title</h1>
                        <p>Content</p>
                    </div>
                </body>
            </html>
        "#;
        let dom = parse_html(html);
        
        // 应该能正确解析
        assert!(dom.root().is_element());
    }

    #[test]
    fn test_query_after_parse() {
        let html = r#"
            <div>
                <p id="test">Hello</p>
            </div>
        "#;
        let dom = parse_html(html);
        
        let found = dom.query_selector("#test");
        assert!(found.is_some());
        // 文本节点可能被包裹，所以检查包含关系
        let text = found.unwrap().collect_text();
        assert!(text.contains("Hello"));
    }
}
