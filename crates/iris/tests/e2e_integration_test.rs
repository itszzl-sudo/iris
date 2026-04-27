//! Phase 7.1 端到端集成测试
//!
//! 测试完整的渲染管线：
//! 1. VNode 创建 → 树构建 → 渲染统计
//! 2. JavaScript 风格 DOM 操作
//! 3. SFC 组件渲染（条件渲染、循环渲染、组件嵌套）
//! 4. 真实 Web 应用结构

use iris_dom::vnode::VNode;
use iris_engine::vnode_renderer::RenderStats;

// ============================================
// 测试 1: VNode 基础操作
// ============================================

/// 测试 VNode 创建和基本结构
#[test]
fn test_vnode_creation_and_structure() {
    // 创建元素
    let div = VNode::element("div");
    assert_eq!(div.tag_name().unwrap(), "div");
    assert_eq!(div.children().len(), 0);

    // 添加子节点
    let mut container = VNode::element("div");
    container.append_child(VNode::text("Hello"));
    container.append_child(VNode::element("span"));

    assert_eq!(container.children().len(), 2);
    assert_eq!(container.children()[0].text_content().unwrap(), "Hello");
    assert_eq!(container.children()[1].tag_name().unwrap(), "span");
}

/// 测试属性设置和获取
#[test]
fn test_vnode_attributes() {
    let mut div = VNode::element("div");
    div.set_attr("class", "container");
    div.set_attr("id", "main");
    div.set_attr("data-value", "123");

    assert_eq!(div.get_attr("class").unwrap(), "container");
    assert_eq!(div.get_attr("id").unwrap(), "main");
    assert_eq!(div.get_attr("data-value").unwrap(), "123");
    assert!(div.get_attr("nonexistent").is_none());
}

/// 测试 Fragment 节点
#[test]
fn test_vnode_fragment() {
    let children = vec![
        VNode::element("div"),
        VNode::element("p"),
        VNode::element("span"),
    ];

    let fragment = VNode::fragment(children);
    assert_eq!(fragment.children().len(), 3);
}

// ============================================
// 测试 2: HTML → VNode → 渲染流程
// ============================================

/// 测试从 HTML 解析到 VNode 树构建的完整流程
#[test]
fn test_html_to_vnode_pipeline() {
    // 模拟 HTML: <div><p>Hello, World!</p><span>Iris Engine</span></div>
    let mut root = VNode::element("div");

    let mut child1 = VNode::element("p");
    child1.append_child(VNode::text("Hello, World!"));
    root.append_child(child1);

    let mut child2 = VNode::element("span");
    child2.append_child(VNode::text("Iris Engine"));
    root.append_child(child2);

    // 验证 DOM 树结构
    assert!(matches!(&root, VNode::Element { tag, .. } if tag == "div"));
    assert_eq!(root.children().len(), 2);

    // 收集渲染统计（模拟 Layout → GPU 流程）
    let stats = RenderStats::collect(&root);
    assert_eq!(stats.total_nodes, 5); // div + p + text + span + text
    assert_eq!(stats.text_nodes, 2);
}

/// 测试嵌套元素的完整布局流程
#[test]
fn test_nested_layout_pipeline() {
    // 创建深层嵌套: div > section > article > text
    let mut level1 = VNode::element("div");
    let mut level2 = VNode::element("section");
    let mut level3 = VNode::element("article");
    level3.append_child(VNode::text("Deep nested content"));
    level2.append_child(level3);
    level1.append_child(level2);

    // 验证嵌套深度
    assert_eq!(level1.children().len(), 1);
    assert_eq!(level1.children()[0].children().len(), 1);
    assert_eq!(level1.children()[0].children()[0].children().len(), 1);

    // 收集渲染统计
    let stats = RenderStats::collect(&level1);
    assert_eq!(stats.total_nodes, 4); // div + section + article + text
}

// ============================================
// 测试 3: JavaScript → DOM 操作
// ============================================

/// 测试通过 JavaScript API 操作 DOM
#[test]
fn test_js_dom_manipulation() {
    // 创建初始 DOM
    let mut container = VNode::element("div");

    // 模拟 JavaScript: document.createElement('p')
    let mut paragraph = VNode::element("p");
    paragraph.append_child(VNode::text("Hello"));

    // 模拟 JavaScript: container.appendChild(paragraph)
    container.append_child(paragraph);

    // 验证 DOM 操作
    assert_eq!(container.children().len(), 1);
    assert!(matches!(&container.children()[0], VNode::Element { tag, .. } if tag == "p"));

    // 模拟 JavaScript: 创建新元素并替换
    let mut new_paragraph = VNode::element("p");
    new_paragraph.set_attr("id", "my-paragraph");
    new_paragraph.set_attr("class", "text-primary");
    new_paragraph.append_child(VNode::text("Updated content"));

    // 模拟 JavaScript: container.replaceChild(newParagraph, oldParagraph)
    container.children_mut()[0] = new_paragraph;

    assert_eq!(container.children().len(), 1);
    // 直接访问文本节点
    if let VNode::Element { children, .. } = &container.children()[0] {
        if let VNode::Text { content } = &children[0] {
            assert_eq!(content, "Updated content");
        }
    }
}

/// 测试复杂的 DOM 操作场景
#[test]
fn test_complex_dom_operations() {
    // 创建列表: <ul>
    let mut list = VNode::element("ul");

    // 添加列表项: <li>Item 1</li> ... <li>Item 5</li>
    for i in 1..=5 {
        let mut item = VNode::element("li");
        item.append_child(VNode::text(&format!("Item {}", i)));
        list.append_child(item);
    }

    assert_eq!(list.children().len(), 5);

    // 模拟 JavaScript: 删除第三个元素 (index 2)
    list.children_mut().remove(2);
    assert_eq!(list.children().len(), 4);

    // 模拟 JavaScript: 在开头插入新元素
    let mut new_first = VNode::element("li");
    new_first.append_child(VNode::text("New First Item"));
    list.children_mut().insert(0, new_first);

    assert_eq!(list.children().len(), 5);
    // 验证第一个子节点的内容
    if let VNode::Element { children, .. } = &list.children()[0] {
        if let VNode::Text { content } = &children[0] {
            assert_eq!(content, "New First Item");
        }
    }
}

// ============================================
// 测试 4: SFC → 编译 → 渲染
// ============================================

/// 测试 Vue SFC 组件的完整渲染流程
#[test]
fn test_sfc_compilation_to_render() {
    // 模拟编译后的 Vue 组件结构
    // <template>
    //   <div class="app">
    //     <h1>{{ title }}</h1>
    //     <p>{{ message }}</p>
    //   </div>
    // </template>

    let mut app_div = VNode::element("div");
    app_div.set_attr("class", "app");

    let mut h1 = VNode::element("h1");
    h1.append_child(VNode::text("Welcome to Iris"));
    app_div.append_child(h1);

    let mut p = VNode::element("p");
    p.append_child(VNode::text("Building amazing apps"));
    app_div.append_child(p);

    // 验证 VNode 结构
    let stats = RenderStats::collect(&app_div);
    assert_eq!(stats.total_nodes, 5); // div + h1 + text + p + text
    assert_eq!(stats.text_nodes, 2);

    // 验证属性设置
    if let VNode::Element { attrs, .. } = &app_div {
        assert_eq!(attrs.get("class").unwrap(), "app");
    }
}

/// 测试带有条件渲染的 SFC 组件 (v-if)
#[test]
fn test_sfc_conditional_rendering() {
    // 模拟 v-if="showMessage"
    let show_message = true;

    let mut container = VNode::element("div");

    if show_message {
        let mut message = VNode::element("p");
        message.append_child(VNode::text("This message is visible"));
        container.append_child(message);
    }

    let stats = RenderStats::collect(&container);
    assert_eq!(stats.total_nodes, 3); // div + p + text
    assert_eq!(stats.text_nodes, 1);

    // 测试条件为假 - 空的 div 仍计入 1 个节点
    let show_message_false = false;
    let mut container2 = VNode::element("div");

    if show_message_false {
        let mut message = VNode::element("p");
        message.append_child(VNode::text("This should not render"));
        container2.append_child(message);
    }

    let stats2 = RenderStats::collect(&container2);
    assert_eq!(stats2.total_nodes, 1); // only div
    assert_eq!(stats2.text_nodes, 0);

    // 测试条件为假
    let show_message_false = false;
    let mut container2 = VNode::element("div");

    if show_message_false {
        let mut message = VNode::element("p");
        message.append_child(VNode::text("This should not render"));
        container2.append_child(message);
    }

    let stats2 = RenderStats::collect(&container2);
    assert_eq!(stats2.total_nodes, 1); // only div
    assert_eq!(stats2.text_nodes, 0);
}

/// 测试带有循环渲染的 SFC 组件 (v-for)
#[test]
fn test_sfc_loop_rendering() {
    // 模拟 v-for="item in items"
    let items = vec!["Apple", "Banana", "Cherry"];

    let mut list = VNode::element("ul");

    for item in &items {
        let mut li = VNode::element("li");
        li.append_child(VNode::text(item));
        list.append_child(li);
    }

    let stats = RenderStats::collect(&list);
    assert_eq!(stats.total_nodes, 7); // ul + 3*(li + text)
    assert_eq!(stats.text_nodes, 3);
    assert_eq!(list.children().len(), 3);

    // 验证每个列表项的内容
    for (i, child) in list.children().iter().enumerate() {
        assert_eq!(child.tag_name().unwrap(), "li");
        assert_eq!(child.children()[0].text_content().unwrap(), items[i]);
    }
}

/// 测试带有组件嵌套的 SFC
#[test]
fn test_sfc_component_nesting() {
    // 模拟组件嵌套：
    // <App>
    //   <Header />
    //   <Main>
    //     <Card />
    //     <Card />
    //   </Main>
    //   <Footer />
    // </App>

    let mut app = VNode::element("div");

    // Header
    let mut header = VNode::element("header");
    header.append_child(VNode::text("Header"));
    app.append_child(header);

    // Main with Cards
    let mut main = VNode::element("main");
    for i in 1..=2 {
        let mut card = VNode::element("div");
        card.set_attr("class", "card");
        card.append_child(VNode::text(&format!("Card {}", i)));
        main.append_child(card);
    }
    app.append_child(main);

    // Footer
    let mut footer = VNode::element("footer");
    footer.append_child(VNode::text("Footer"));
    app.append_child(footer);

    let stats = RenderStats::collect(&app);
    assert_eq!(stats.total_nodes, 10);
    // div(app) + header + text + main + 2*(div + text) + footer + text
}

// ============================================
// 测试 5: 完整集成场景
// ============================================

/// 测试真实的 Web 应用结构
#[test]
fn test_realistic_web_app() {
    // 创建一个真实的网页结构
    let mut html = VNode::element("html");

    // <head>
    let mut head = VNode::element("head");
    let mut title = VNode::element("title");
    title.append_child(VNode::text("My Iris App"));
    head.append_child(title);
    html.append_child(head);

    // <body>
    let mut body = VNode::element("body");

    // <nav>
    let mut nav = VNode::element("nav");
    let mut nav_links = VNode::element("ul");
    for link in &["Home", "About", "Contact"] {
        let mut li = VNode::element("li");
        let mut a = VNode::element("a");
        a.append_child(VNode::text(link));
        li.append_child(a);
        nav_links.append_child(li);
    }
    nav.append_child(nav_links);
    body.append_child(nav);

    // <main>
    let mut main = VNode::element("main");
    let mut article = VNode::element("article");
    let mut h1 = VNode::element("h1");
    h1.append_child(VNode::text("Welcome"));
    article.append_child(h1);

    let mut p1 = VNode::element("p");
    p1.append_child(VNode::text("This is a paragraph."));
    article.append_child(p1);

    main.append_child(article);
    body.append_child(main);

    // <footer>
    let mut footer = VNode::element("footer");
    footer.append_child(VNode::text("© 2026 Iris Engine"));
    body.append_child(footer);

    html.append_child(body);

    // 验证完整结构
    let stats = RenderStats::collect(&html);
    assert!(stats.total_nodes > 20);
    assert!(stats.text_nodes > 5);
}

/// 测试表单元素的渲染
#[test]
fn test_form_elements_rendering() {
    let mut form = VNode::element("form");
    form.set_attr("action", "/submit");
    form.set_attr("method", "post");

    let mut input_group = VNode::element("div");
    let mut label1 = VNode::element("label");
    label1.append_child(VNode::text("Name:"));
    input_group.append_child(label1);

    let mut input = VNode::element("input");
    input.set_attr("type", "text");
    input.set_attr("name", "username");
    input_group.append_child(input);

    form.append_child(input_group);

    let mut button = VNode::element("button");
    button.set_attr("type", "submit");
    button.append_child(VNode::text("Submit"));
    form.append_child(button);

    let stats = RenderStats::collect(&form);
    assert_eq!(stats.total_nodes, 7);
}

/// 测试表格元素的渲染
#[test]
fn test_table_rendering() {
    let mut table = VNode::element("table");

    // Table header
    let mut thead = VNode::element("thead");
    let mut header_row = VNode::element("tr");
    for header in &["Name", "Age", "City"] {
        let mut th = VNode::element("th");
        th.append_child(VNode::text(header));
        header_row.append_child(th);
    }
    thead.append_child(header_row);
    table.append_child(thead);

    // Table body
    let mut tbody = VNode::element("tbody");

    // Row 1
    let mut row1 = VNode::element("tr");
    for cell in &["Alice", "30", "New York"] {
        let mut td = VNode::element("td");
        td.append_child(VNode::text(cell));
        row1.append_child(td);
    }
    tbody.append_child(row1);

    // Row 2
    let mut row2 = VNode::element("tr");
    for cell in &["Bob", "25", "London"] {
        let mut td = VNode::element("td");
        td.append_child(VNode::text(cell));
        row2.append_child(td);
    }
    tbody.append_child(row2);

    table.append_child(tbody);

    let stats = RenderStats::collect(&table);
    assert_eq!(stats.total_nodes, 24);
    // table + thead + tr + 3*th + 3*text + tbody + 2*tr + 6*td + 6*text
}

/// 测试大型 DOM 树的性能
#[test]
fn test_large_dom_tree_performance() {
    // 创建一个包含 100 个节点的树
    let mut root = VNode::element("div");

    for i in 0..10 {
        let mut section = VNode::element("section");
        for j in 0..10 {
            let mut article = VNode::element("article");
            article.append_child(VNode::text(&format!("Article {}-{}", i, j)));
            section.append_child(article);
        }
        root.append_child(section);
    }

    let stats = RenderStats::collect(&root);
    // 1 root + 10 sections + 100 articles + 100 texts = 211
    assert_eq!(stats.total_nodes, 211);
    assert_eq!(stats.text_nodes, 100);
}
