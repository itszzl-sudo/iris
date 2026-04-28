//! CSSRule 对象封装
//!
//! 实现 Web 标准的 CSSRule 接口及其子类型。

use std::sync::{Arc, Mutex};
use crate::cssom::CSSStyleDeclaration;

/// CSSRule 类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum CSSRuleType {
    /// 样式规则: selector { ... }
    STYLE_RULE,
    /// 媒体规则: @media { ... }
    MEDIA_RULE,
    /// 关键帧规则: @keyframes { ... }
    KEYFRAMES_RULE,
    /// 导入规则: @import
    IMPORT_RULE,
    /// 字体规则: @font-face
    FONT_FACE_RULE,
    /// 支持规则: @supports
    SUPPORTS_RULE,
}

/// CSSRule 对象（基类）
///
/// 表示一个 CSS 规则，是所有 CSS 规则类型的基类。
#[derive(Debug, Clone)]
pub struct CSSRuleOM {
    /// 规则类型
    pub rule_type: CSSRuleType,
    /// CSS 文本
    css_text: String,
    /// 父样式表（可选）
    parent_stylesheet: Option<String>,
}

impl CSSRuleOM {
    /// 创建新的 CSSRule
    pub fn new(rule_type: CSSRuleType, css_text: &str) -> Self {
        Self {
            rule_type,
            css_text: css_text.to_string(),
            parent_stylesheet: None,
        }
    }

    /// 获取 CSS 文本
    pub fn get_css_text(&self) -> &str {
        &self.css_text
    }

    /// 设置 CSS 文本
    pub fn set_css_text(&mut self, text: &str) {
        self.css_text = text.to_string();
    }

    /// 获取父样式表名称
    pub fn parent_stylesheet(&self) -> Option<&str> {
        self.parent_stylesheet.as_deref()
    }
}

/// CSSStyleRule - 样式规则
///
/// 表示一个普通的样式规则：`selector { declarations }`
///
/// # 示例
///
/// ```rust
/// use iris_cssom::cssrule::{CSSStyleRule, CSSRuleOM};
/// use iris_cssom::cssom::CSSStyleDeclaration;
///
/// let mut rule = CSSStyleRule::new(".class");
/// rule.style().set_property("color", "red", "");
/// ```
#[derive(Debug, Clone)]
pub struct CSSStyleRule {
    /// 基础规则
    base: CSSRuleOM,
    /// 选择器文本
    selector_text: String,
    /// 样式声明
    style: Arc<Mutex<CSSStyleDeclaration>>,
}

impl CSSStyleRule {
    /// 创建新的样式规则
    pub fn new(selector: &str) -> Self {
        let css_text = format!("{} {{ }}", selector);
        Self {
            base: CSSRuleOM::new(CSSRuleType::STYLE_RULE, &css_text),
            selector_text: selector.to_string(),
            style: Arc::new(Mutex::new(CSSStyleDeclaration::new())),
        }
    }

    /// 从内部 CSSRule 创建
    pub fn from_internal(rule: &crate::css::CSSRule) -> Self {
        let selector_text = rule.selector.text.clone();
        let style = Arc::new(Mutex::new(
            CSSStyleDeclaration::from_declarations(&rule.declarations)
        ));
        let css_text = format!("{} {{ {} }}", selector_text, style.lock().unwrap().get_css_text());
        
        Self {
            base: CSSRuleOM::new(CSSRuleType::STYLE_RULE, &css_text),
            selector_text,
            style,
        }
    }

    /// 获取选择器文本
    pub fn selector_text(&self) -> &str {
        &self.selector_text
    }

    /// 设置选择器文本
    pub fn set_selector_text(&mut self, text: &str) {
        self.selector_text = text.to_string();
        self.update_css_text();
    }

    /// 获取样式声明对象
    pub fn style(&self) -> Arc<Mutex<CSSStyleDeclaration>> {
        Arc::clone(&self.style)
    }

    /// 更新 CSS 文本
    fn update_css_text(&mut self) {
        let style_text = self.style.lock().unwrap().get_css_text();
        self.base.set_css_text(&format!(
            "{} {{ {} }}",
            self.selector_text, style_text
        ));
    }

    /// 转换为内部 CSSRule
    pub fn to_internal(&self) -> crate::css::CSSRule {
        let selector = crate::css::Selector::new(&self.selector_text);
        let declarations = self.style.lock().unwrap().to_declarations();
        crate::css::CSSRule::new(selector, declarations)
    }
}

/// CSSMediaRule - 媒体查询规则
///
/// 表示一个 @media 规则：`@media query { ...rules... }`
///
/// # 示例
///
/// ```rust
/// use iris_cssom::cssrule::CSSMediaRule;
///
/// let mut media_rule = CSSMediaRule::new("screen and (max-width: 600px)");
/// // 可以添加子规则
/// ```
#[derive(Clone)]
pub struct CSSMediaRule {
    /// 基础规则
    base: CSSRuleOM,
    /// 媒体查询条件
    condition_text: String,
    /// 子规则列表
    css_rules: Vec<Arc<Mutex<dyn CSSRuleTrait>>>,
}

impl std::fmt::Debug for CSSMediaRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CSSMediaRule")
            .field("base", &self.base)
            .field("condition_text", &self.condition_text)
            .field("css_rules_count", &self.css_rules.len())
            .finish()
    }
}

impl CSSMediaRule {
    /// 创建新的媒体规则
    pub fn new(condition: &str) -> Self {
        let css_text = format!("@media {} {{ }}", condition);
        Self {
            base: CSSRuleOM::new(CSSRuleType::MEDIA_RULE, &css_text),
            condition_text: condition.to_string(),
            css_rules: Vec::new(),
        }
    }

    /// 获取媒体查询条件
    pub fn condition_text(&self) -> &str {
        &self.condition_text
    }

    /// 设置媒体查询条件
    pub fn set_condition_text(&mut self, text: &str) {
        self.condition_text = text.to_string();
        self.update_css_text();
    }

    /// 插入子规则
    pub fn insert_rule(&mut self, rule: Arc<Mutex<dyn CSSRuleTrait>>, index: usize) {
        if index > self.css_rules.len() {
            self.css_rules.push(rule);
        } else {
            self.css_rules.insert(index, rule);
        }
        self.update_css_text();
    }

    /// 删除子规则
    pub fn delete_rule(&mut self, index: usize) -> bool {
        if index < self.css_rules.len() {
            self.css_rules.remove(index);
            self.update_css_text();
            true
        } else {
            false
        }
    }

    /// 获取子规则数量
    pub fn length(&self) -> usize {
        self.css_rules.len()
    }

    /// 获取子规则
    pub fn css_rule(&self, index: usize) -> Option<Arc<Mutex<dyn CSSRuleTrait>>> {
        if index < self.css_rules.len() {
            Some(Arc::clone(&self.css_rules[index]))
        } else {
            None
        }
    }

    /// 更新 CSS 文本
    fn update_css_text(&mut self) {
        let rules_text = self.css_rules
            .iter()
            .map(|r| r.lock().unwrap().get_css_text().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        
        self.base.set_css_text(&format!(
            "@media {} {{\n{}\n}}",
            self.condition_text, rules_text
        ));
    }
}

/// CSSRule trait - 所有 CSSRule 类型的通用接口
pub trait CSSRuleTrait {
    /// 获取规则类型
    fn rule_type(&self) -> CSSRuleType;
    
    /// 获取 CSS 文本
    fn get_css_text(&self) -> &str;
    
    /// 设置 CSS 文本
    fn set_css_text(&mut self, text: &str);
}

impl CSSRuleTrait for CSSStyleRule {
    fn rule_type(&self) -> CSSRuleType {
        CSSRuleType::STYLE_RULE
    }
    
    fn get_css_text(&self) -> &str {
        self.base.get_css_text()
    }
    
    fn set_css_text(&mut self, text: &str) {
        self.base.set_css_text(text);
    }
}

impl CSSRuleTrait for CSSMediaRule {
    fn rule_type(&self) -> CSSRuleType {
        CSSRuleType::MEDIA_RULE
    }
    
    fn get_css_text(&self) -> &str {
        self.base.get_css_text()
    }
    
    fn set_css_text(&mut self, text: &str) {
        self.base.set_css_text(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_style_rule() {
        let rule = CSSStyleRule::new(".container");
        assert_eq!(rule.selector_text(), ".container");
        assert_eq!(rule.base.rule_type, CSSRuleType::STYLE_RULE);
    }

    #[test]
    fn test_css_style_rule_set_property() {
        let rule = CSSStyleRule::new(".container");
        rule.style().lock().unwrap().set_property("color", "red", "");
        assert_eq!(rule.style().lock().unwrap().get_property_value("color"), "red");
    }

    #[test]
    fn test_css_style_rule_to_internal() {
        let mut rule = CSSStyleRule::new(".container");
        rule.style().lock().unwrap().set_property("color", "red", "");
        
        let internal = rule.to_internal();
        assert_eq!(internal.selector.text, ".container");
        assert_eq!(internal.declarations.len(), 1);
    }

    #[test]
    fn test_css_media_rule() {
        let media_rule = CSSMediaRule::new("screen and (max-width: 600px)");
        assert_eq!(media_rule.condition_text(), "screen and (max-width: 600px)");
        assert_eq!(media_rule.base.rule_type, CSSRuleType::MEDIA_RULE);
    }

    #[test]
    fn test_css_media_rule_insert_delete() {
        let mut media_rule = CSSMediaRule::new("screen");
        let style_rule = Arc::new(Mutex::new(CSSStyleRule::new(".class")));
        
        media_rule.insert_rule(style_rule.clone(), 0);
        assert_eq!(media_rule.length(), 1);
        
        let deleted = media_rule.delete_rule(0);
        assert!(deleted);
        assert_eq!(media_rule.length(), 0);
    }
}
