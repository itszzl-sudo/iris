//! CSSStyleDeclaration - CSSOM 样式声明对象
//!
//! 实现 Web 标准的 CSSStyleDeclaration API，用于操作 CSS 属性。
//!
//! # 示例
//!
//! ```rust
//! use iris_cssom::cssom::CSSStyleDeclaration;
//!
//! let mut style = CSSStyleDeclaration::new();
//! style.set_property("color", "red", "");
//! style.set_property("font-size", "16px", "important");
//!
//! assert_eq!(style.get_property_value("color"), "red");
//! assert_eq!(style.get_property_priority("font-size"), "important");
//! ```

use std::collections::HashMap;

/// CSS 属性值（包含优先级信息）
#[derive(Debug, Clone)]
struct CSSPropertyValue {
    value: String,
    important: bool,
}

/// CSSStyleDeclaration 对象
///
/// 表示一个 CSS 声明块，提供对样式属性的读写操作。
/// 对标 Web API: `CSSStyleDeclaration`
#[derive(Debug, Clone)]
pub struct CSSStyleDeclaration {
    /// 属性映射
    properties: HashMap<String, CSSPropertyValue>,
    /// 父样式表引用（可选）
    parent_stylesheet: Option<String>,
}

impl CSSStyleDeclaration {
    /// 创建空的样式声明
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            parent_stylesheet: None,
        }
    }

    /// 从声明列表创建
    pub fn from_declarations(declarations: &[crate::css::Declaration]) -> Self {
        let mut properties = HashMap::new();
        for decl in declarations {
            properties.insert(
                decl.property.clone(),
                CSSPropertyValue {
                    value: decl.value.clone(),
                    important: false, // 默认不重要
                },
            );
        }
        Self {
            properties,
            parent_stylesheet: None,
        }
    }

    /// 获取属性值
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// assert_eq!(style.get_property_value("color"), "red");
    /// assert_eq!(style.get_property_value("background"), ""); // 不存在的属性
    /// ```
    pub fn get_property_value(&self, property: &str) -> String {
        self.properties
            .get(property)
            .map(|p| p.value.clone())
            .unwrap_or_default()
    }

    /// 获取属性优先级
    ///
    /// 返回 "important" 或空字符串
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "important");
    /// assert_eq!(style.get_property_priority("color"), "important");
    /// ```
    pub fn get_property_priority(&self, property: &str) -> String {
        self.properties
            .get(property)
            .map(|p| if p.important { "important" } else { "" })
            .unwrap_or_default()
            .to_string()
    }

    /// 设置属性值
    ///
    /// # 参数
    ///
    /// * `property` - 属性名（如 "color", "font-size"）
    /// * `value` - 属性值（如 "red", "16px"）
    /// * `priority` - 优先级（"" 或 "important"）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// style.set_property("font-weight", "bold", "important");
    /// ```
    pub fn set_property(&mut self, property: &str, value: &str, priority: &str) {
        let important = priority.to_lowercase() == "important";
        self.properties.insert(
            property.to_lowercase(),
            CSSPropertyValue {
                value: value.to_string(),
                important,
            },
        );
    }

    /// 移除属性
    ///
    /// 返回被移除的属性值，如果属性不存在则返回空字符串
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// let removed = style.remove_property("color");
    /// assert_eq!(removed, "red");
    /// assert_eq!(style.get_property_value("color"), "");
    /// ```
    pub fn remove_property(&mut self, property: &str) -> String {
        self.properties
            .remove(&property.to_lowercase())
            .map(|p| p.value)
            .unwrap_or_default()
    }

    /// 获取属性数量
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// style.set_property("font-size", "16px", "");
    /// assert_eq!(style.length(), 2);
    /// ```
    pub fn length(&self) -> usize {
        self.properties.len()
    }

    /// 根据索引获取属性名
    ///
    /// # 注意
    ///
    /// 由于 HashMap 是无序的，这个方法返回的属性名顺序不保证稳定
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// if let Some(prop) = style.item(0) {
    ///     assert_eq!(prop, "color");
    /// }
    /// ```
    pub fn item(&self, index: usize) -> Option<String> {
        self.properties.keys().nth(index).cloned()
    }

    /// 获取 CSS 文本表示
    ///
    /// 返回格式：`property1: value1; property2: value2 !important;`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// style.set_property("font-weight", "bold", "important");
    ///
    /// let css_text = style.get_css_text();
    /// assert!(css_text.contains("color: red"));
    /// assert!(css_text.contains("font-weight: bold !important"));
    /// ```
    pub fn get_css_text(&self) -> String {
        self.properties
            .iter()
            .map(|(k, v)| {
                let important = if v.important { " !important" } else { "" };
                format!("{}: {}{}", k, v.value, important)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// 设置 CSS 文本
    ///
    /// 解析 CSS 文本并替换所有属性
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_css_text("color: red; font-size: 16px");
    /// assert_eq!(style.get_property_value("color"), "red");
    /// assert_eq!(style.get_property_value("font-size"), "16px");
    /// ```
    pub fn set_css_text(&mut self, text: &str) {
        self.properties.clear();
        
        // 简单的 CSS 解析器
        for declaration in text.split(';') {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }
            
            if let Some(colon_pos) = declaration.find(':') {
                let property = declaration[..colon_pos].trim().to_lowercase();
                let mut value_part = declaration[colon_pos + 1..].trim();
                
                // 检查 !important
                let important = value_part.ends_with("!important");
                if important {
                    value_part = value_part[..value_part.len() - 10].trim();
                }
                
                if !property.is_empty() && !value_part.is_empty() {
                    self.properties.insert(
                        property,
                        CSSPropertyValue {
                            value: value_part.to_string(),
                            important,
                        },
                    );
                }
            }
        }
    }

    /// 获取所有属性名列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// style.set_property("font-size", "16px", "");
    ///
    /// let props = style.get_property_names();
    /// assert_eq!(props.len(), 2);
    /// assert!(props.contains(&"color".to_string()));
    /// ```
    pub fn get_property_names(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    /// 检查是否包含某个属性
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// assert!(style.has_property("color"));
    /// assert!(!style.has_property("background"));
    /// ```
    pub fn has_property(&self, property: &str) -> bool {
        self.properties.contains_key(&property.to_lowercase())
    }

    /// 清空所有属性
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style = CSSStyleDeclaration::new();
    /// style.set_property("color", "red", "");
    /// style.clear();
    /// assert_eq!(style.length(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    /// 合并另一个样式声明
    ///
    /// 只在当前没有该属性时才覆盖（低优先级）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_cssom::cssom::CSSStyleDeclaration;
    ///
    /// let mut style1 = CSSStyleDeclaration::new();
    /// style1.set_property("color", "red", "");
    ///
    /// let mut style2 = CSSStyleDeclaration::new();
    /// style2.set_property("font-size", "16px", "");
    /// style2.set_property("color", "blue", ""); // 不会覆盖 style1
    ///
    /// style1.merge(&style2);
    /// assert_eq!(style1.get_property_value("color"), "red"); // 保留原值
    /// assert_eq!(style1.get_property_value("font-size"), "16px"); // 新增
    /// ```
    pub fn merge(&mut self, other: &CSSStyleDeclaration) {
        for (key, value) in &other.properties {
            if !self.properties.contains_key(key) {
                self.properties.insert(key.clone(), value.clone());
            }
        }
    }

    /// 转换为内部声明列表（用于与 iris-layout 集成）
    pub fn to_declarations(&self) -> Vec<crate::css::Declaration> {
        self.properties
            .iter()
            .map(|(k, v)| crate::css::Declaration {
                property: k.clone(),
                value: v.value.clone(),
            })
            .collect()
    }
}

impl Default for CSSStyleDeclaration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_property() {
        let mut style = CSSStyleDeclaration::new();
        style.set_property("color", "red", "");
        assert_eq!(style.get_property_value("color"), "red");
    }

    #[test]
    fn test_important_priority() {
        let mut style = CSSStyleDeclaration::new();
        style.set_property("color", "red", "important");
        assert_eq!(style.get_property_priority("color"), "important");
    }

    #[test]
    fn test_remove_property() {
        let mut style = CSSStyleDeclaration::new();
        style.set_property("color", "red", "");
        let removed = style.remove_property("color");
        assert_eq!(removed, "red");
        assert!(!style.has_property("color"));
    }

    #[test]
    fn test_css_text() {
        let mut style = CSSStyleDeclaration::new();
        style.set_property("color", "red", "");
        style.set_property("font-weight", "bold", "important");
        
        let css_text = style.get_css_text();
        assert!(css_text.contains("color: red"));
        assert!(css_text.contains("font-weight: bold !important"));
    }

    #[test]
    fn test_set_css_text() {
        let mut style = CSSStyleDeclaration::new();
        style.set_css_text("color: red; font-size: 16px !important");
        
        assert_eq!(style.get_property_value("color"), "red");
        assert_eq!(style.get_property_value("font-size"), "16px");
        assert_eq!(style.get_property_priority("font-size"), "important");
    }

    #[test]
    fn test_length_and_item() {
        let mut style = CSSStyleDeclaration::new();
        style.set_property("color", "red", "");
        style.set_property("font-size", "16px", "");
        
        assert_eq!(style.length(), 2);
        assert!(style.item(0).is_some());
    }

    #[test]
    fn test_merge() {
        let mut style1 = CSSStyleDeclaration::new();
        style1.set_property("color", "red", "");
        
        let mut style2 = CSSStyleDeclaration::new();
        style2.set_property("font-size", "16px", "");
        
        style1.merge(&style2);
        assert_eq!(style1.get_property_value("color"), "red");
        assert_eq!(style1.get_property_value("font-size"), "16px");
    }

    #[test]
    fn test_clear() {
        let mut style = CSSStyleDeclaration::new();
        style.set_property("color", "red", "");
        style.clear();
        assert_eq!(style.length(), 0);
    }
}
