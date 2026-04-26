//! BOM API 模拟
//!
//! 模拟浏览器环境中的 window、document、location 等全局对象。

use crate::vnode::VNode;
use std::collections::HashMap;

/// Location 对象 (模拟 window.location)
#[derive(Debug, Clone)]
pub struct Location {
    /// 完整 URL
    pub href: String,
    /// 协议
    pub protocol: String,
    /// 主机名
    pub hostname: String,
    /// 端口
    pub port: String,
    /// 路径名
    pub pathname: String,
    /// 查询字符串
    pub search: String,
    /// 哈希
    pub hash: String,
}

impl Location {
    /// 创建 Location 对象
    pub fn new(url: &str) -> Self {
        Self {
            href: url.to_string(),
            protocol: "file://".to_string(),
            hostname: "localhost".to_string(),
            port: String::new(),
            pathname: "/".to_string(),
            search: String::new(),
            hash: String::new(),
        }
    }
}

/// Navigator 对象 (模拟 window.navigator)
#[derive(Debug, Clone)]
pub struct Navigator {
    /// 用户代理字符串
    pub user_agent: String,
    /// 平台
    pub platform: String,
    /// 语言
    pub language: String,
    /// 是否在线
    pub on_line: bool,
}

impl Default for Navigator {
    fn default() -> Self {
        Self {
            user_agent: "Iris/0.1.0".to_string(),
            platform: "unknown".to_string(),
            language: "zh-CN".to_string(),
            on_line: true,
        }
    }
}

/// History 对象 (模拟 window.history)
#[derive(Debug)]
pub struct History {
    /// 历史记录栈
    entries: Vec<String>,
    /// 当前位置索引
    current_index: usize,
}

impl History {
    /// 创建 History 对象
    pub fn new() -> Self {
        Self {
            entries: vec!["/".to_string()],
            current_index: 0,
        }
    }

    /// 历史记录长度
    pub fn length(&self) -> usize {
        self.entries.len()
    }

    /// 前进
    pub fn forward(&mut self) {
        if self.current_index + 1 < self.entries.len() {
            self.current_index += 1;
        }
    }

    /// 后退
    pub fn back(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
    }

    /// 推入新状态
    pub fn push_state(&mut self, url: &str) {
        // 删除当前索引之后的所有记录
        self.entries.truncate(self.current_index + 1);
        self.entries.push(url.to_string());
        self.current_index += 1;
    }

    /// 获取当前 URL
    pub fn current_url(&self) -> &str {
        &self.entries[self.current_index]
    }
}

/// Console 对象 (模拟 window.console)
pub struct Console;

impl Console {
    /// 输出日志
    pub fn log(&self, message: &str) {
        println!("[LOG] {}", message);
    }

    /// 输出警告
    pub fn warn(&self, message: &str) {
        println!("[WARN] {}", message);
    }

    /// 输出错误
    pub fn error(&self, message: &str) {
        eprintln!("[ERROR] {}", message);
    }

    /// 输出信息
    pub fn info(&self, message: &str) {
        println!("[INFO] {}", message);
    }

    /// 计时开始
    pub fn time(&self, label: &str) {
        println!("[TIME] {} started", label);
    }

    /// 计时结束
    pub fn time_end(&self, label: &str) {
        println!("[TIME] {} ended", label);
    }
}

/// Window 对象 (全局对象)
///
/// 模拟浏览器环境中的 window 对象。
///
/// # 示例
///
/// ```rust
/// use iris_dom::bom::Window;
///
/// let mut window = Window::new(800, 600);
/// assert_eq!(window.inner_width(), 800);
/// assert_eq!(window.inner_height(), 600);
/// ```
pub struct Window {
    /// 窗口宽度
    width: u32,
    /// 窗口高度
    height: u32,
    /// Location 对象
    pub location: Location,
    /// Navigator 对象
    pub navigator: Navigator,
    /// History 对象
    pub history: History,
    /// Console 对象
    pub console: Console,
    /// 全局存储
    storage: HashMap<String, String>,
}

impl Window {
    /// 创建 Window 对象
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            location: Location::new("/"),
            navigator: Navigator::default(),
            history: History::new(),
            console: Console,
            storage: HashMap::new(),
        }
    }

    /// 获取窗口内部宽度
    pub fn inner_width(&self) -> u32 {
        self.width
    }

    /// 获取窗口内部高度
    pub fn inner_height(&self) -> u32 {
        self.height
    }

    /// 调整窗口大小
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// 设置全局属性
    pub fn set_property(&mut self, key: &str, value: &str) {
        self.storage.insert(key.to_string(), value.to_string());
    }

    /// 获取全局属性
    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.storage.get(key)
    }
}

/// Document 对象 (模拟 document)
///
/// 提供 DOM 操作 API。
///
/// # 示例
///
/// ```rust
/// use iris_dom::bom::Document;
/// use iris_dom::vnode::VNode;
///
/// let mut doc = Document::new();
/// let element = doc.create_element("div");
/// assert!(element.is_element());
/// ```
pub struct Document {
    /// 根节点
    root: VNode,
}

impl Document {
    /// 创建 Document 对象
    pub fn new() -> Self {
        Self {
            root: VNode::element("html"),
        }
    }

    /// 创建元素节点
    pub fn create_element(&self, tag: &str) -> VNode {
        VNode::element(tag)
    }

    /// 创建文本节点
    pub fn create_text_node(&self, text: &str) -> VNode {
        VNode::text(text)
    }

    /// 创建注释节点
    pub fn create_comment(&self, text: &str) -> VNode {
        VNode::comment(text)
    }

    /// 获取根节点
    pub fn root(&self) -> &VNode {
        &self.root
    }

    /// 获取 root 的可变引用
    pub fn root_mut(&mut self) -> &mut VNode {
        &mut self.root
    }

    /// 查询选择器 (简化实现)
    pub fn query_selector(&self, selector: &str) -> Option<&VNode> {
        // 这里应该实现完整的选择器匹配逻辑
        // 简化：仅支持基本查询
        self.query_selector_recursive(&self.root, selector)
    }

    fn query_selector_recursive<'a>(&'a self, node: &'a VNode, selector: &str) -> Option<&'a VNode> {
        // 检查当前节点
        if let VNode::Element { attrs, .. } = node {
            if selector.starts_with('#') {
                // ID 选择器
                let id = &selector[1..];
                if let Some(node_id) = attrs.get("id") {
                    if node_id == id {
                        return Some(node);
                    }
                }
            } else if selector.starts_with('.') {
                // Class 选择器
                let class = &selector[1..];
                if let Some(node_class) = attrs.get("class") {
                    if node_class.split_whitespace().any(|c| c == class) {
                        return Some(node);
                    }
                }
            } else {
                // 标签选择器
                if let VNode::Element { tag, .. } = node {
                    if tag == selector {
                        return Some(node);
                    }
                }
            }
        }

        // 递归查询子节点
        match node {
            VNode::Element { children, .. } | VNode::Fragment { children } => {
                for child in children {
                    if let Some(found) = self.query_selector_recursive(child, selector) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// 查询所有匹配选择器的节点
    pub fn query_selector_all(&self, selector: &str) -> Vec<&VNode> {
        let mut results = Vec::new();
        self.query_selector_all_recursive(&self.root, selector, &mut results);
        results
    }

    fn query_selector_all_recursive<'a>(
        &'a self,
        node: &'a VNode,
        selector: &str,
        results: &mut Vec<&'a VNode>,
    ) {
        if let Some(matched) = self.query_selector_recursive(node, selector) {
            results.push(matched);
        }

        match node {
            VNode::Element { children, .. } | VNode::Fragment { children } => {
                for child in children {
                    self.query_selector_all_recursive(child, selector, results);
                }
            }
            _ => {}
        }
    }

    /// 获取元素 by ID
    pub fn get_element_by_id(&self, id: &str) -> Option<&VNode> {
        self.query_selector(&format!("#{}", id))
    }

    /// 获取元素 by class
    pub fn get_elements_by_class_name(&self, class: &str) -> Vec<&VNode> {
        self.query_selector_all(&format!(".{}", class))
    }

    /// 获取元素 by tag name
    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<&VNode> {
        self.query_selector_all(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_creation() {
        let window = Window::new(800, 600);
        assert_eq!(window.inner_width(), 800);
        assert_eq!(window.inner_height(), 600);
    }

    #[test]
    fn test_window_resize() {
        let mut window = Window::new(800, 600);
        window.resize(1024, 768);
        assert_eq!(window.inner_width(), 1024);
        assert_eq!(window.inner_height(), 768);
    }

    #[test]
    fn test_window_storage() {
        let mut window = Window::new(800, 600);
        window.set_property("theme", "dark");
        assert_eq!(window.get_property("theme"), Some(&"dark".to_string()));
    }

    #[test]
    fn test_history_navigation() {
        let mut history = History::new();
        assert_eq!(history.length(), 1);

        history.push_state("/page1");
        history.push_state("/page2");
        assert_eq!(history.length(), 3);
        assert_eq!(history.current_url(), "/page2");

        history.back();
        assert_eq!(history.current_url(), "/page1");

        history.forward();
        assert_eq!(history.current_url(), "/page2");
    }

    #[test]
    fn test_document_create_element() {
        let doc = Document::new();
        let div = doc.create_element("div");
        assert!(div.is_element());
        assert_eq!(div.tag_name(), Some("div"));
    }

    #[test]
    fn test_document_query_selector() {
        let mut doc = Document::new();
        let mut body = doc.create_element("body");
        let mut div = doc.create_element("div");
        div.set_attr("id", "main");
        div.set_attr("class", "container");
        body.append_child(div);
        doc.root_mut().append_child(body);

        // 测试 ID 选择器
        let found = doc.query_selector("#main");
        assert!(found.is_some());

        // 测试 Class 选择器
        let found = doc.query_selector(".container");
        assert!(found.is_some());

        // 测试标签选择器
        let found = doc.query_selector("div");
        assert!(found.is_some());
    }

    #[test]
    fn test_document_get_element_by_id() {
        let mut doc = Document::new();
        let mut body = doc.create_element("body");
        let mut div = doc.create_element("div");
        div.set_attr("id", "test");
        body.append_child(div);
        doc.root_mut().append_child(body);

        let found = doc.get_element_by_id("test");
        assert!(found.is_some());
    }

    #[test]
    fn test_console_output() {
        let console = Console;
        // 这些只是打印，不会失败
        console.log("test log");
        console.warn("test warn");
        console.error("test error");
    }
}
