//! CSSStyleSheet - CSSOM 样式表对象
//!
//! 实现 Web 标准的 CSSStyleSheet 接口，提供样式表的完整操作 API。

use std::sync::{Arc, Mutex};
use crate::css::{parse_stylesheet, CSSRule as InternalCSSRule};
use crate::cssrule::{CSSStyleRule};
use crate::cssrulelist::CSSRuleList;

/// CSSStyleSheet 对象
///
/// 表示一个 CSS 样式表，提供对样式表规则的完整操作能力。
/// 对标 Web API: `CSSStyleSheet`
///
/// # 示例
///
/// ```rust
/// use iris_cssom::stylesheet::CSSStyleSheet;
///
/// let mut sheet = CSSStyleSheet::new();
/// sheet.insert_rule(".container { color: red; }", 0).unwrap();
///
/// assert_eq!(sheet.css_rules().lock().unwrap().length(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct CSSStyleSheet {
    /// 样式表是否禁用
    disabled: bool,
    /// 样式表来源 URL
    href: Option<String>,
    /// 拥有此样式表的 DOM 元素（标识）
    owner_node: Option<String>,
    /// 规则列表
    css_rules: Arc<Mutex<CSSRuleList>>,
    /// 内部样式表（用于与 iris-layout 集成）
    internal_stylesheet: Arc<Mutex<InternalStylesheetWrapper>>,
}

/// 内部样式表包装器
#[derive(Debug, Clone)]
struct InternalStylesheetWrapper {
    pub rules: Vec<InternalCSSRule>,
}

impl InternalStylesheetWrapper {
    fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }
}

impl CSSStyleSheet {
    /// 创建空的样式表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::stylesheet::CSSStyleSheet;
    ///
    /// let sheet = CSSStyleSheet::new();
    /// assert!(!sheet.disabled());
    /// assert_eq!(sheet.css_rules().lock().unwrap().length(), 0);
/// ```
    pub fn new() -> Self {
        Self {
            disabled: false,
            href: None,
            owner_node: None,
            css_rules: Arc::new(Mutex::new(CSSRuleList::new())),
            internal_stylesheet: Arc::new(Mutex::new(InternalStylesheetWrapper::new())),
        }
    }

    /// 从 CSS 文本创建样式表
    ///
    /// # 参数
    ///
    /// * `css_text` - CSS 文本内容
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::stylesheet::CSSStyleSheet;
    ///
    /// let css = r#"
    ///     .container { padding: 20px; }
    ///     #header { background: blue; }
    /// "#;
    /// let sheet = CSSStyleSheet::from_css(css);
    /// assert!(sheet.css_rules().lock().unwrap().length() > 0);
    /// ```
    pub fn from_css(css_text: &str) -> Self {
        let sheet = Self::new();
        
        // 解析 CSS
        let internal_stylesheet = parse_stylesheet(css_text);
        
        // 转换为 CSSOM 规则
        for rule in &internal_stylesheet.rules {
            let style_rule = CSSStyleRule::from_internal(rule);
            sheet.css_rules.lock().unwrap().append_rule(
                Arc::new(Mutex::new(style_rule))
            );
        }
        
        // 保存内部样式表
        sheet.internal_stylesheet.lock().unwrap().rules = internal_stylesheet.rules;
        
        sheet
    }

    /// 获取样式表是否禁用
    pub fn disabled(&self) -> bool {
        self.disabled
    }

    /// 设置样式表禁用状态
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// 获取样式表 URL
    pub fn href(&self) -> Option<&str> {
        self.href.as_deref()
    }

    /// 设置样式表 URL
    pub fn set_href(&mut self, href: &str) {
        self.href = Some(href.to_string());
    }

    /// 获取拥有此样式表的 DOM 元素标识
    pub fn owner_node(&self) -> Option<&str> {
        self.owner_node.as_deref()
    }

    /// 设置拥有此样式表的 DOM 元素标识
    pub fn set_owner_node(&mut self, node_id: &str) {
        self.owner_node = Some(node_id.to_string());
    }

    /// 获取规则列表（实时更新的 live list）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::stylesheet::CSSStyleSheet;
    ///
    /// let mut sheet = CSSStyleSheet::new();
    /// sheet.insert_rule(".class { color: red; }", 0).unwrap();
    ///
    /// let rules = sheet.css_rules();
    /// assert_eq!(rules.lock().unwrap().length(), 1);
    /// ```
    pub fn css_rules(&self) -> Arc<Mutex<CSSRuleList>> {
        Arc::clone(&self.css_rules)
    }

    /// 插入一条新规则
    ///
    /// # 参数
    ///
    /// * `rule` - CSS 规则文本（如 ".class { color: red; }"）
    /// * `index` - 插入位置（从 0 开始）
    ///
    /// # 返回值
    ///
    /// 成功返回插入位置的索引，失败返回错误
    ///
    /// # 错误
    ///
    /// * 如果索引超出范围，返回 `Err`
    /// * 如果 CSS 规则语法错误，返回 `Err`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::stylesheet::CSSStyleSheet;
    ///
    /// let mut sheet = CSSStyleSheet::new();
    /// let index = sheet.insert_rule(".container { color: red; }", 0);
    /// assert!(index.is_ok());
    /// assert_eq!(index.unwrap(), 0);
    /// ```
    pub fn insert_rule(&mut self, rule: &str, index: usize) -> Result<u32, String> {
        if self.disabled {
            return Err("Cannot insert rule into a disabled stylesheet".to_string());
        }

        // 解析单条规则
        let temp_stylesheet = parse_stylesheet(rule);
        if temp_stylesheet.rules.is_empty() {
            return Err("Failed to parse CSS rule".to_string());
        }

        let internal_rule = temp_stylesheet.rules.into_iter().next().unwrap();
        let style_rule = CSSStyleRule::from_internal(&internal_rule);
        let rule_arc = Arc::new(Mutex::new(style_rule));

        // 插入到规则列表
        let mut rules = self.css_rules.lock().unwrap();
        if index > rules.length() as usize {
            return Err("Index out of bounds".to_string());
        }
        
        rules.insert_rule(rule_arc, index);
        
        // 更新内部样式表
        self.internal_stylesheet.lock().unwrap().rules.insert(index, internal_rule);

        Ok(index as u32)
    }

    /// 删除指定索引的规则
    ///
    /// # 参数
    ///
    /// * `index` - 要删除的规则索引
    ///
    /// # 错误
    ///
    /// 如果索引超出范围，返回 `Err`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::stylesheet::CSSStyleSheet;
    ///
    /// let mut sheet = CSSStyleSheet::new();
    /// sheet.insert_rule(".class { color: red; }", 0).unwrap();
    /// sheet.delete_rule(0).unwrap();
    ///
    /// assert_eq!(sheet.css_rules().lock().unwrap().length(), 0);
    /// ```
    pub fn delete_rule(&mut self, index: usize) -> Result<(), String> {
        if self.disabled {
            return Err("Cannot delete rule from a disabled stylesheet".to_string());
        }

        let mut rules = self.css_rules.lock().unwrap();
        if index >= rules.length() as usize {
            return Err("Index out of bounds".to_string());
        }
        
        rules.remove_rule(index);
        
        // 更新内部样式表
        self.internal_stylesheet.lock().unwrap().rules.remove(index);

        Ok(())
    }

    /// 替换指定索引的规则
    ///
    /// # 参数
    ///
    /// * `old_rule` - 旧的 CSS 规则文本（用于匹配）
    /// * `new_rule` - 新的 CSS 规则文本
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误
    ///
    /// # 注意
    ///
    /// 这是较新的 API（replaceRule），部分浏览器支持
    pub fn replace_rule(&mut self, old_rule: &str, new_rule: &str) -> Result<u32, String> {
        let rules = self.css_rules.lock().unwrap();
        let texts = rules.get_all_css_texts();
        
        // 查找旧规则
        let index = texts.iter().position(|t| t == old_rule);
        drop(rules);
        
        if let Some(index) = index {
            // 删除旧规则
            self.delete_rule(index)?;
            // 插入新规则
            self.insert_rule(new_rule, index)
        } else {
            Err("Old rule not found".to_string())
        }
    }

    /// 添加一条规则到末尾（较新的 API）
    ///
    /// # 参数
    ///
    /// * `rule` - CSS 规则文本
    ///
    /// # 返回值
    ///
    /// 成功返回新规则的索引
    pub fn add_rule(&mut self, rule: &str) -> Result<u32, String> {
        let index = self.css_rules.lock().unwrap().length() as usize;
        self.insert_rule(rule, index)
    }

    /// 获取内部样式表（用于与 iris-layout 集成）
    pub fn internal_stylesheet(&self) -> crate::css::Stylesheet {
        let wrapper = self.internal_stylesheet.lock().unwrap();
        crate::css::Stylesheet {
            rules: wrapper.rules.clone(),
        }
    }

    /// 获取 CSS 文本（整个样式表的文本表示）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::stylesheet::CSSStyleSheet;
    ///
    /// let mut sheet = CSSStyleSheet::new();
    /// sheet.insert_rule(".class { color: red; }", 0).unwrap();
    ///
    /// let css_text = sheet.get_css_text();
    /// assert!(css_text.contains(".class"));
    /// ```
    pub fn get_css_text(&self) -> String {
        let rules = self.css_rules.lock().unwrap();
        rules.get_all_css_texts().join("\n")
    }

    /// 清空所有规则
    pub fn clear(&mut self) {
        self.css_rules.lock().unwrap().clear();
        self.internal_stylesheet.lock().unwrap().rules.clear();
    }

    /// 获取规则数量
    pub fn rule_count(&self) -> u32 {
        self.css_rules.lock().unwrap().length()
    }
}

impl Default for CSSStyleSheet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stylesheet() {
        let sheet = CSSStyleSheet::new();
        assert!(!sheet.disabled());
        assert_eq!(sheet.css_rules().lock().unwrap().length(), 0);
    }

    #[test]
    fn test_from_css() {
        let css = ".container { padding: 20px; }";
        let sheet = CSSStyleSheet::from_css(css);
        assert_eq!(sheet.css_rules().lock().unwrap().length(), 1);
    }

    #[test]
    fn test_insert_rule() {
        let mut sheet = CSSStyleSheet::new();
        let result = sheet.insert_rule(".class { color: red; }", 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(sheet.rule_count(), 1);
    }

    #[test]
    fn test_delete_rule() {
        let mut sheet = CSSStyleSheet::new();
        sheet.insert_rule(".class { color: red; }", 0).unwrap();
        sheet.delete_rule(0).unwrap();
        assert_eq!(sheet.rule_count(), 0);
    }

    #[test]
    fn test_insert_rule_out_of_bounds() {
        let mut sheet = CSSStyleSheet::new();
        let result = sheet.insert_rule(".class { color: red; }", 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_rule_out_of_bounds() {
        let mut sheet = CSSStyleSheet::new();
        let result = sheet.delete_rule(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_disabled_stylesheet() {
        let mut sheet = CSSStyleSheet::new();
        sheet.set_disabled(true);
        let result = sheet.insert_rule(".class { color: red; }", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_rule() {
        let mut sheet = CSSStyleSheet::new();
        sheet.add_rule(".class1 { color: red; }").unwrap();
        sheet.add_rule(".class2 { color: blue; }").unwrap();
        assert_eq!(sheet.rule_count(), 2);
    }

    #[test]
    fn test_get_css_text() {
        let mut sheet = CSSStyleSheet::new();
        sheet.insert_rule(".class { color: red; }", 0).unwrap();
        let css_text = sheet.get_css_text();
        assert!(css_text.contains(".class"));
        assert!(css_text.contains("color: red"));
    }

    #[test]
    fn test_clear() {
        let mut sheet = CSSStyleSheet::new();
        sheet.insert_rule(".class1 { color: red; }", 0).unwrap();
        sheet.insert_rule(".class2 { color: blue; }", 1).unwrap();
        sheet.clear();
        assert_eq!(sheet.rule_count(), 0);
    }

    #[test]
    fn test_internal_stylesheet() {
        let mut sheet = CSSStyleSheet::new();
        sheet.insert_rule(".class { color: red; }", 0).unwrap();
        
        let internal = sheet.internal_stylesheet();
        assert_eq!(internal.rules.len(), 1);
        assert_eq!(internal.rules[0].selector.text, ".class");
    }
}
