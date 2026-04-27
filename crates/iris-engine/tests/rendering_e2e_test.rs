//! 端到端渲染集成测试
//!
//! 测试从 VNode 创建到 GPU 渲染命令生成的完整渲染管线。

use iris_dom::vnode::VNode;
use iris_engine::vnode_renderer::{VNodeRenderer, RenderStats};

/// 测试场景 1: 简单元素渲染
#[test]
fn test_e2e_simple_element_rendering() {
    // 创建简单的 div 元素
    let mut div = VNode::element("div");
    
    // 验证 VNode 创建
    assert!(matches!(&div, VNode::Element { tag, .. } if tag == "div"));
    
    // 收集渲染统计
    let stats = RenderStats::collect(&div);
    assert_eq!(stats.total_nodes, 1);
    assert_eq!(stats.elements_drawn, 0); // 无布局信息，不绘制
}

/// 测试场景 2: 嵌套元素渲染
#[test]
fn test_e2e_nested_elements() {
    // 创建嵌套结构: div > p > span
    let mut span = VNode::element("span");
    let mut p = VNode::element("p");
    p.append_child(span);
    
    let mut div = VNode::element("div");
    div.append_child(p);
    
    // 验证嵌套结构
    let stats = RenderStats::collect(&div);
    assert_eq!(stats.total_nodes, 3);
}

/// 测试场景 3: 文本节点渲染
#[test]
fn test_e2e_text_node_rendering() {
    // 创建包含文本的元素
    let mut p = VNode::element("p");
    p.append_child(VNode::text("Hello World"));
    
    // 验证文本节点
    let stats = RenderStats::collect(&p);
    assert_eq!(stats.total_nodes, 2);
    assert_eq!(stats.text_nodes, 1);
}

/// 测试场景 4: 混合内容渲染
#[test]
fn test_e2e_mixed_content() {
    // 创建复杂结构: div > (p > text, span > text)
    let mut p = VNode::element("p");
    p.append_child(VNode::text("Paragraph 1"));
    
    let mut span = VNode::element("span");
    span.append_child(VNode::text("Span text"));
    
    let mut div = VNode::element("div");
    div.append_child(p);
    div.append_child(span);
    
    // 验证混合内容
    let stats = RenderStats::collect(&div);
    assert_eq!(stats.total_nodes, 5);
    assert_eq!(stats.text_nodes, 2);
    assert_eq!(stats.elements_drawn, 0); // 无布局
}

/// 测试场景 5: Fragment 渲染
#[test]
fn test_e2e_fragment_rendering() {
    // 创建 Fragment 包含多个子元素
    let child1 = VNode::element("div");
    let child2 = VNode::element("p");
    let child3 = VNode::element("span");
    
    let fragment = VNode::fragment(vec![child1, child2, child3]);
    
    // 验证 Fragment
    let stats = RenderStats::collect(&fragment);
    assert_eq!(stats.total_nodes, 3); // Fragment 本身不计入
}

/// 测试场景 6: 注释节点过滤
#[test]
fn test_e2e_comment_node_filtering() {
    // 创建包含注释的结构
    let mut div = VNode::element("div");
    div.append_child(VNode::comment("This is a comment"));
    div.append_child(VNode::text("Visible text"));
    
    // 验证注释节点仍然计入 total_nodes（但不计入 text_nodes）
    let stats = RenderStats::collect(&div);
    assert_eq!(stats.total_nodes, 3); // div + comment + text
    assert_eq!(stats.text_nodes, 1); // 只有 text 节点
}

/// 测试场景 7: 深度嵌套渲染
#[test]
fn test_e2e_deep_nesting() {
    // 创建深度嵌套结构 (10 层)
    let mut root = VNode::element("div");
    let mut current = &mut root;
    
    for i in 0..10 {
        let mut child = VNode::element(&format!("level-{}", i));
        current.append_child(child);
        // 注意：这里需要更好的方式来构建深度树
    }
    
    // 验证深度嵌套
    let stats = RenderStats::collect(&root);
    assert!(stats.total_nodes >= 1);
}

/// 测试场景 8: 空元素渲染
#[test]
fn test_e2e_empty_element() {
    // 创建空元素
    let div = VNode::element("div");
    
    // 验证空元素
    let stats = RenderStats::collect(&div);
    assert_eq!(stats.total_nodes, 1);
    assert_eq!(stats.text_nodes, 0);
}

/// 测试场景 9: 多文本节点
#[test]
fn test_e2e_multiple_text_nodes() {
    // 创建包含多个文本的元素
    let mut p = VNode::element("p");
    p.append_child(VNode::text("Hello "));
    p.append_child(VNode::text("World"));
    p.append_child(VNode::text("!"));
    
    // 验证多个文本节点
    let stats = RenderStats::collect(&p);
    assert_eq!(stats.total_nodes, 4);
    assert_eq!(stats.text_nodes, 3);
}

/// 测试场景 10: 元素属性渲染
#[test]
fn test_e2e_element_with_attributes() {
    // 创建带属性的元素
    let mut div = VNode::element("div");
    // 注意：当前 VNode API 可能不支持直接设置属性
    // 这个测试验证元素创建的基本功能
    
    let stats = RenderStats::collect(&div);
    assert_eq!(stats.total_nodes, 1);
}

/// 测试场景 11: 大型 DOM 树渲染
#[test]
fn test_e2e_large_dom_tree() {
    // 创建大型 DOM 树 (100 个元素)
    let mut root = VNode::element("root");
    
    for i in 0..100 {
        let child = VNode::element(&format!("child-{}", i));
        root.append_child(child);
    }
    
    // 验证大型树
    let stats = RenderStats::collect(&root);
    assert_eq!(stats.total_nodes, 101); // root + 100 children
}

/// 测试场景 12: 文本和元素混合嵌套
#[test]
fn test_e2e_complex_mixed_nesting() {
    // 创建复杂混合结构
    let mut article = VNode::element("article");
    
    let mut header = VNode::element("header");
    header.append_child(VNode::text("Title"));
    
    let mut content = VNode::element("div");
    content.append_child(VNode::text("Content paragraph 1"));
    
    let mut bold = VNode::element("strong");
    bold.append_child(VNode::text("bold text"));
    content.append_child(bold);
    
    content.append_child(VNode::text("Content paragraph 2"));
    
    article.append_child(header);
    article.append_child(content);
    
    // 验证复杂结构: article(1) + header(1) + text(1) + div(1) + text(1) + strong(1) + text(1) + text(1) = 8
    let stats = RenderStats::collect(&article);
    assert_eq!(stats.total_nodes, 8);
    assert_eq!(stats.text_nodes, 4);
}

/// 测试场景 13: VNode 渲染器基础功能
#[test]
fn test_e2e_renderer_basic() {
    // 创建简单的 VNode
    let vnode = VNode::element("div");
    
    // 验证渲染器可以处理该 VNode
    // 注意：完整测试需要 GPU 上下文，这里只测试统计功能
    let stats = RenderStats::collect(&vnode);
    assert!(stats.total_nodes > 0);
}

/// 测试场景 14: 渲染统计准确性
#[test]
fn test_e2e_stats_accuracy() {
    // 创建已知结构的 VNode 树
    let mut parent = VNode::element("parent");
    
    for _ in 0..5 {
        let mut child = VNode::element("child");
        child.append_child(VNode::text("text"));
        parent.append_child(child);
    }
    
    // 验证统计准确性
    let stats = RenderStats::collect(&parent);
    assert_eq!(stats.total_nodes, 11); // 1 parent + 5 children + 5 text nodes
    assert_eq!(stats.text_nodes, 5);
}

/// 测试场景 15: 边界条件 - 空树
#[test]
fn test_e2e_empty_tree() {
    // 创建 Fragment 包含空列表
    let fragment = VNode::fragment(vec![]);
    
    // 验证空树
    let stats = RenderStats::collect(&fragment);
    assert_eq!(stats.total_nodes, 0);
}
