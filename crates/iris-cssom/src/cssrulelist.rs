//! CSSRuleList - CSSOM 规则列表
//!
//! 实现 Web 标准的 CSSRuleList 接口。

use std::sync::{Arc, Mutex};
use crate::cssrule::CSSRuleTrait;

/// CSSRuleList 对象
///
/// 表示一个 CSSRule 对象的集合，通常通过 `CSSStyleSheet.cssRules` 访问。
/// 这是一个实时更新的列表（live list），当样式表变化时自动更新。
///
/// # 示例
///
/// ```rust
/// use iris_cssom::cssrulelist::CSSRuleList;
/// use iris_cssom::cssrule::CSSStyleRule;
/// use std::sync::{Arc, Mutex};
///
/// let mut list = CSSRuleList::new();
/// let rule = Arc::new(Mutex::new(CSSStyleRule::new(".class")));
/// list.append_rule(rule);
///
/// assert_eq!(list.length(), 1);
/// ```
#[derive(Clone)]
pub struct CSSRuleList {
    /// 规则列表
    rules: Vec<Arc<Mutex<dyn CSSRuleTrait>>>,
}

impl std::fmt::Debug for CSSRuleList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CSSRuleList")
            .field("rules_count", &self.rules.len())
            .finish()
    }
}

impl CSSRuleList {
    /// 创建空的规则列表
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// 从规则向量创建
    pub fn from_rules(rules: Vec<Arc<Mutex<dyn CSSRuleTrait>>>) -> Self {
        Self { rules }
    }

    /// 获取规则数量
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssrulelist::CSSRuleList;
    ///
    /// let list = CSSRuleList::new();
    /// assert_eq!(list.length(), 0);
    /// ```
    pub fn length(&self) -> u32 {
        self.rules.len() as u32
    }

    /// 根据索引获取规则
    ///
    /// # 参数
    ///
    /// * `index` - 规则索引（从 0 开始）
    ///
    /// # 返回值
    ///
    /// 返回指定索引的规则，如果索引超出范围则返回 `None`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssrulelist::CSSRuleList;
    /// use iris_cssom::cssrule::CSSStyleRule;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut list = CSSRuleList::new();
    /// let rule = Arc::new(Mutex::new(CSSStyleRule::new(".class")));
    /// list.append_rule(rule.clone());
    ///
    /// let retrieved = list.item(0);
    /// assert!(retrieved.is_some());
    /// ```
    pub fn item(&self, index: u32) -> Option<Arc<Mutex<dyn CSSRuleTrait>>> {
        if index < self.rules.len() as u32 {
            Some(Arc::clone(&self.rules[index as usize]))
        } else {
            None
        }
    }

    /// 添加规则到列表末尾
    ///
    /// # 参数
    ///
    /// * `rule` - 要添加的规则
    pub fn append_rule(&mut self, rule: Arc<Mutex<dyn CSSRuleTrait>>) {
        self.rules.push(rule);
    }

    /// 插入规则到指定位置
    ///
    /// # 参数
    ///
    /// * `rule` - 要插入的规则
    /// * `index` - 插入位置
    ///
    /// # 返回值
    ///
    /// 插入成功返回 `true`，索引超出范围返回 `false`
    pub fn insert_rule(&mut self, rule: Arc<Mutex<dyn CSSRuleTrait>>, index: usize) -> bool {
        if index > self.rules.len() {
            return false;
        }
        self.rules.insert(index, rule);
        true
    }

    /// 删除指定索引的规则
    ///
    /// # 参数
    ///
    /// * `index` - 要删除的规则索引
    ///
    /// # 返回值
    ///
    /// 删除成功返回 `true`，索引超出范围返回 `false`
    pub fn remove_rule(&mut self, index: usize) -> bool {
        if index < self.rules.len() {
            self.rules.remove(index);
            true
        } else {
            false
        }
    }

    /// 清空所有规则
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// 获取所有规则的迭代器
    pub fn iter(&self) -> impl Iterator<Item = &Arc<Mutex<dyn CSSRuleTrait>>> {
        self.rules.iter()
    }

    /// 获取所有规则的 CSS 文本
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssrulelist::CSSRuleList;
    /// use iris_cssom::cssrule::CSSStyleRule;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut list = CSSRuleList::new();
    /// let rule1 = Arc::new(Mutex::new(CSSStyleRule::new(".class1")));
    /// let rule2 = Arc::new(Mutex::new(CSSStyleRule::new(".class2")));
    /// list.append_rule(rule1);
    /// list.append_rule(rule2);
    ///
    /// let css_texts = list.get_all_css_texts();
    /// assert_eq!(css_texts.len(), 2);
    /// ```
    pub fn get_all_css_texts(&self) -> Vec<String> {
        self.rules
            .iter()
            .map(|r| r.lock().unwrap().get_css_text().to_string())
            .collect()
    }
}

impl Default for CSSRuleList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cssrule::CSSStyleRule;

    #[test]
    fn test_new_list() {
        let list = CSSRuleList::new();
        assert_eq!(list.length(), 0);
    }

    #[test]
    fn test_append_rule() {
        let mut list = CSSRuleList::new();
        let rule = Arc::new(Mutex::new(CSSStyleRule::new(".class")));
        list.append_rule(rule);
        assert_eq!(list.length(), 1);
    }

    #[test]
    fn test_item() {
        let mut list = CSSRuleList::new();
        let rule = Arc::new(Mutex::new(CSSStyleRule::new(".class")));
        list.append_rule(rule.clone());
        
        let retrieved = list.item(0);
        assert!(retrieved.is_some());
        
        let not_found = list.item(1);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_insert_rule() {
        let mut list = CSSRuleList::new();
        let rule1 = Arc::new(Mutex::new(CSSStyleRule::new(".class1")));
        let rule2 = Arc::new(Mutex::new(CSSStyleRule::new(".class2")));
        
        list.append_rule(rule1);
        list.insert_rule(rule2, 0); // 插入到开头
        
        assert_eq!(list.length(), 2);
        assert!(list.item(0).unwrap().lock().unwrap().get_css_text().contains(".class2"));
    }

    #[test]
    fn test_remove_rule() {
        let mut list = CSSRuleList::new();
        let rule = Arc::new(Mutex::new(CSSStyleRule::new(".class")));
        list.append_rule(rule);
        
        assert!(list.remove_rule(0));
        assert_eq!(list.length(), 0);
        
        assert!(!list.remove_rule(0)); // 已经空了
    }

    #[test]
    fn test_clear() {
        let mut list = CSSRuleList::new();
        list.append_rule(Arc::new(Mutex::new(CSSStyleRule::new(".class1"))));
        list.append_rule(Arc::new(Mutex::new(CSSStyleRule::new(".class2"))));
        
        list.clear();
        assert_eq!(list.length(), 0);
    }

    #[test]
    fn test_get_all_css_texts() {
        let mut list = CSSRuleList::new();
        list.append_rule(Arc::new(Mutex::new(CSSStyleRule::new(".class1"))));
        list.append_rule(Arc::new(Mutex::new(CSSStyleRule::new(".class2"))));
        
        let texts = list.get_all_css_texts();
        assert_eq!(texts.len(), 2);
        assert!(texts[0].contains(".class1"));
        assert!(texts[1].contains(".class2"));
    }
}
