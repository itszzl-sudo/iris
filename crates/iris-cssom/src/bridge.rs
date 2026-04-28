//! CSSOM 桥接层
//!
//! 提供 CSSOM 与 iris-layout 之间的集成桥接。

use std::sync::{Arc, Mutex};
use crate::stylesheet::CSSStyleSheet;

/// CSSOM 管理器
///
/// 管理多个样式表，并提供与 iris-layout 的集成接口。
///
/// # 示例
///
/// ```rust
/// use iris_cssom::bridge::CSSOMManager;
///
/// let mut manager = CSSOMManager::new();
/// manager.add_stylesheet("sheet1");
/// manager.insert_rule_to_sheet("sheet1", ".class { color: red; }", 0).unwrap();
///
/// let stylesheet = manager.get_stylesheet_for_layout("sheet1");
/// assert!(stylesheet.is_some());
/// ```
#[derive(Debug)]
pub struct CSSOMManager {
    /// 样式表注册表
    stylesheets: std::collections::HashMap<String, Arc<Mutex<CSSStyleSheet>>>,
}

impl CSSOMManager {
    /// 创建新的 CSSOM 管理器
    pub fn new() -> Self {
        Self {
            stylesheets: std::collections::HashMap::new(),
        }
    }

    /// 添加样式表
    ///
    /// # 参数
    ///
    /// * `name` - 样式表名称（唯一标识）
    pub fn add_stylesheet(&mut self, name: &str) {
        let sheet = CSSStyleSheet::new();
        self.stylesheets.insert(name.to_string(), Arc::new(Mutex::new(sheet)));
    }

    /// 从 CSS 文本添加样式表
    ///
    /// # 参数
    ///
    /// * `name` - 样式表名称
    /// * `css_text` - CSS 文本内容
    pub fn add_stylesheet_from_css(&mut self, name: &str, css_text: &str) {
        let sheet = CSSStyleSheet::from_css(css_text);
        self.stylesheets.insert(name.to_string(), Arc::new(Mutex::new(sheet)));
    }

    /// 获取样式表
    pub fn get_stylesheet(&self, name: &str) -> Option<Arc<Mutex<CSSStyleSheet>>> {
        self.stylesheets.get(name).map(Arc::clone)
    }

    /// 移除样式表
    pub fn remove_stylesheet(&mut self, name: &str) {
        self.stylesheets.remove(name);
    }

    /// 向指定样式表插入规则
    ///
    /// # 参数
    ///
    /// * `sheet_name` - 样式表名称
    /// * `rule` - CSS 规则文本
    /// * `index` - 插入位置
    pub fn insert_rule_to_sheet(
        &mut self,
        sheet_name: &str,
        rule: &str,
        index: usize,
    ) -> Result<u32, String> {
        if let Some(sheet) = self.stylesheets.get(sheet_name) {
            sheet.lock().unwrap().insert_rule(rule, index)
        } else {
            Err(format!("Stylesheet '{}' not found", sheet_name))
        }
    }

    /// 从指定样式表删除规则
    pub fn delete_rule_from_sheet(
        &mut self,
        sheet_name: &str,
        index: usize,
    ) -> Result<(), String> {
        if let Some(sheet) = self.stylesheets.get(sheet_name) {
            sheet.lock().unwrap().delete_rule(index)
        } else {
            Err(format!("Stylesheet '{}' not found", sheet_name))
        }
    }

    /// 获取用于 iris-layout 的样式表
    ///
    /// 这将返回 iris-layout 可以使用的内部样式表格式
    pub fn get_stylesheet_for_layout(&self, name: &str) -> Option<crate::css::Stylesheet> {
        self.stylesheets
            .get(name)
            .map(|sheet| sheet.lock().unwrap().internal_stylesheet())
    }

    /// 获取所有样式表的名称列表
    pub fn get_stylesheet_names(&self) -> Vec<String> {
        self.stylesheets.keys().cloned().collect()
    }

    /// 获取样式表数量
    pub fn stylesheet_count(&self) -> usize {
        self.stylesheets.len()
    }

    /// 清空所有样式表
    pub fn clear_all(&mut self) {
        self.stylesheets.clear();
    }
}

impl Default for CSSOMManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager() {
        let manager = CSSOMManager::new();
        assert_eq!(manager.stylesheet_count(), 0);
    }

    #[test]
    fn test_add_stylesheet() {
        let mut manager = CSSOMManager::new();
        manager.add_stylesheet("main");
        assert_eq!(manager.stylesheet_count(), 1);
        assert!(manager.get_stylesheet("main").is_some());
    }

    #[test]
    fn test_add_stylesheet_from_css() {
        let mut manager = CSSOMManager::new();
        manager.add_stylesheet_from_css("main", ".class { color: red; }");
        
        let sheet = manager.get_stylesheet("main");
        assert!(sheet.is_some());
        assert_eq!(sheet.unwrap().lock().unwrap().rule_count(), 1);
    }

    #[test]
    fn test_insert_rule_to_sheet() {
        let mut manager = CSSOMManager::new();
        manager.add_stylesheet("main");
        
        let result = manager.insert_rule_to_sheet("main", ".class { color: red; }", 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_rule_from_sheet() {
        let mut manager = CSSOMManager::new();
        manager.add_stylesheet_from_css("main", ".class { color: red; }");
        
        let result = manager.delete_rule_from_sheet("main", 0);
        assert!(result.is_ok());
        assert_eq!(
            manager.get_stylesheet("main").unwrap().lock().unwrap().rule_count(),
            0
        );
    }

    #[test]
    fn test_get_stylesheet_for_layout() {
        let mut manager = CSSOMManager::new();
        manager.add_stylesheet_from_css("main", ".class { color: red; }");
        
        let layout_sheet = manager.get_stylesheet_for_layout("main");
        assert!(layout_sheet.is_some());
        assert_eq!(layout_sheet.unwrap().rules.len(), 1);
    }

    #[test]
    fn test_remove_stylesheet() {
        let mut manager = CSSOMManager::new();
        manager.add_stylesheet("main");
        manager.remove_stylesheet("main");
        assert_eq!(manager.stylesheet_count(), 0);
    }

    #[test]
    fn test_clear_all() {
        let mut manager = CSSOMManager::new();
        manager.add_stylesheet("sheet1");
        manager.add_stylesheet("sheet2");
        manager.clear_all();
        assert_eq!(manager.stylesheet_count(), 0);
    }
}
