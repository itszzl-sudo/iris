//! ES Modules 解析器和注入器
//!
//! 解析 JavaScript 模块中的 import/export 语句，
//! 并将其转换为 Boa Engine 可执行的代码。

use boa_engine::{Context, JsValue, Source};
use std::collections::HashMap;
use regex::Regex;

/// ES Modules 管理器
pub struct EsModules {
    /// 已注册的模块代码
    module_codes: HashMap<String, String>,
    /// 已执行的模块导出
    module_exports: HashMap<String, HashMap<String, JsValue>>,
}

impl EsModules {
    /// 创建新的 ES Modules 管理器
    pub fn new() -> Self {
        Self {
            module_codes: HashMap::new(),
            module_exports: HashMap::new(),
        }
    }

    /// 注册模块代码
    pub fn register_module(&mut self, specifier: &str, code: &str) {
        self.module_codes.insert(specifier.to_string(), code.to_string());
    }

    /// 注入 ES Modules 支持到 JavaScript 环境
    pub fn inject(&mut self, context: &mut Context) -> Result<(), String> {
        self.inject_import_system(context)?;
        self.inject_export_system(context)?;
        Ok(())
    }

    /// 注入 import 系统
    fn inject_import_system(&self, context: &mut Context) -> Result<(), String> {
        // 使用 eval 创建简化的模块系统
        // 注意：Boa Engine 不支持原生 ES modules，我们需要模拟
        let import_code = r#"
            (function() {
                // 模块注册表
                var moduleRegistry = {};
                
                // 注册模块
                globalThis.__registerModule = function(specifier, moduleFn) {
                    moduleRegistry[specifier] = moduleFn;
                };
                
                // 动态 import（简化实现）
                globalThis.import = function(specifier) {
                    return new Promise(function(resolve, reject) {
                        if (moduleRegistry[specifier]) {
                            var module = moduleRegistry[specifier]();
                            resolve(module);
                        } else {
                            reject(new Error('Module not found: ' + specifier));
                        }
                    });
                };
                
                // 同步获取模块（用于内部使用）
                globalThis.__getModule = function(specifier) {
                    if (moduleRegistry[specifier]) {
                        return moduleRegistry[specifier]();
                    }
                    return {};
                };
            })()
        "#;

        context
            .eval(Source::from_bytes(import_code))
            .map_err(|e| format!("Failed to inject import system: {}", e))?;

        Ok(())
    }

    /// 注入 export 系统
    fn inject_export_system(&self, context: &mut Context) -> Result<(), String> {
        let export_code = r#"
            (function() {
                // 创建默认导出对象
                globalThis.__createModule = function(exports) {
                    return {
                        default: exports.default || null,
                        ...exports
                    };
                };
            })()
        "#;

        context
            .eval(Source::from_bytes(export_code))
            .map_err(|e| format!("Failed to inject export system: {}", e))?;

        Ok(())
    }

    /// 解析模块代码中的 import 语句
    pub fn parse_imports(&self, code: &str) -> Vec<String> {
        let mut imports = Vec::new();
        
        // 匹配 import ... from '...' 或 import ... from "..."
        let import_regex = Regex::new(r#"import\s+.*?from\s+['"]([^'"]+)['"]"#).unwrap();
        for cap in import_regex.captures_iter(code) {
            if let Some(specifier) = cap.get(1) {
                imports.push(specifier.as_str().to_string());
            }
        }
        
        // 匹配 import '...' 或 import "..."（side-effect imports）
        let side_effect_regex = Regex::new(r#"import\s+['"]([^'"]+)['"]"#).unwrap();
        for cap in side_effect_regex.captures_iter(code) {
            if let Some(specifier) = cap.get(1) {
                imports.push(specifier.as_str().to_string());
            }
        }
        
        imports
    }

    /// 转换模块代码，将 import/export 转换为可执行代码
    pub fn transform_module(&self, specifier: &str, code: &str) -> String {
        let mut transformed = String::new();
        
        // 添加模块包装器
        transformed.push_str(&format!("(function() {{\n"));
        transformed.push_str(&format!("  var exports = {{}};\n"));
        transformed.push_str(&format!("  var __module = {{}};\n\n"));
        
        // 处理 import 语句
        let lines: Vec<&str> = code.lines().collect();
        for line in lines {
            let trimmed = line.trim();
            
            // 处理 import { x, y } from 'module'
            if trimmed.starts_with("import {") {
                if let Some((names, source)) = self.parse_named_import(trimmed) {
                    transformed.push_str(&format!(
                        "  var __mod = __getModule('{}');\n",
                        source
                    ));
                    for name in names {
                        let name = name.trim();
                        transformed.push_str(&format!(
                            "  var {} = __mod['{}'];\n",
                            name, name
                        ));
                    }
                    continue;
                }
            }
            
            // 处理 import * as name from 'module'
            if trimmed.starts_with("import * as") {
                if let Some((alias, source)) = self.parse_namespace_import(trimmed) {
                    transformed.push_str(&format!(
                        "  var {} = __getModule('{}');\n",
                        alias, source
                    ));
                    continue;
                }
            }
            
            // 处理 import default from 'module'
            if trimmed.starts_with("import ") && trimmed.contains(" from ") && !trimmed.contains("{") && !trimmed.contains("*") {
                if let Some((default_name, source)) = self.parse_default_import(trimmed) {
                    transformed.push_str(&format!(
                        "  var __mod = __getModule('{}');\n",
                        source
                    ));
                    transformed.push_str(&format!(
                        "  var {} = __mod['default'] || __mod;\n",
                        default_name
                    ));
                    continue;
                }
            }
            
            // 处理 export default
            if trimmed.starts_with("export default") {
                let value = trimmed["export default ".len()..].trim();
                transformed.push_str(&format!("  exports['default'] = {};\n", value));
                continue;
            }
            
            // 处理 export { x, y }
            if trimmed.starts_with("export {") {
                if let Some(names) = self.parse_export_names(trimmed) {
                    for name in names {
                        let name = name.trim();
                        transformed.push_str(&format!("  exports['{}'] = {};\n", name, name));
                    }
                    continue;
                }
            }
            
            // 处理 export const/let/var/function/class
            if trimmed.starts_with("export ") {
                if let Some(decl) = self.parse_export_declaration(trimmed) {
                    transformed.push_str(&format!("  {}\n", decl));
                    // 提取变量名并添加到 exports
                    if let Some(var_name) = self.extract_variable_name(&decl) {
                        transformed.push_str(&format!("  exports['{}'] = {};\n", var_name, var_name));
                    }
                    continue;
                }
            }
            
            // 普通代码行
            transformed.push_str(&format!("  {}\n", line));
        }
        
        // 注册模块
        transformed.push_str(&format!("\n"));
        transformed.push_str(&format!("  __registerModule('{}', function() {{\n", specifier));
        transformed.push_str(&format!("    return exports;\n"));
        transformed.push_str(&format!("  }});\n"));
        transformed.push_str(&format!("}})();\n"));
        
        transformed
    }

    /// 解析命名导入: import { x, y } from 'module'
    fn parse_named_import(&self, line: &str) -> Option<(Vec<String>, String)> {
        let regex = Regex::new(r#"import\s+\{([^}]+)\}\s+from\s+['"]([^'"]+)['"]"#).unwrap();
        if let Some(cap) = regex.captures(line) {
            let names: Vec<String> = cap.get(1).unwrap().as_str().split(',').map(|s| s.trim().to_string()).collect();
            let source = cap.get(2).unwrap().as_str().to_string();
            Some((names, source))
        } else {
            None
        }
    }

    /// 解析命名空间导入: import * as name from 'module'
    fn parse_namespace_import(&self, line: &str) -> Option<(String, String)> {
        let regex = Regex::new(r#"import\s+\*\s+as\s+(\w+)\s+from\s+['"]([^'"]+)['"]"#).unwrap();
        if let Some(cap) = regex.captures(line) {
            let alias = cap.get(1).unwrap().as_str().to_string();
            let source = cap.get(2).unwrap().as_str().to_string();
            Some((alias, source))
        } else {
            None
        }
    }

    /// 解析默认导入: import name from 'module'
    fn parse_default_import(&self, line: &str) -> Option<(String, String)> {
        let regex = Regex::new(r#"import\s+(\w+)\s+from\s+['"]([^'"]+)['"]"#).unwrap();
        if let Some(cap) = regex.captures(line) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let source = cap.get(2).unwrap().as_str().to_string();
            Some((name, source))
        } else {
            None
        }
    }

    /// 解析导出名称: export { x, y }
    fn parse_export_names(&self, line: &str) -> Option<Vec<String>> {
        let regex = Regex::new(r#"export\s+\{([^}]+)\}"#).unwrap();
        if let Some(cap) = regex.captures(line) {
            let names: Vec<String> = cap.get(1).unwrap().as_str().split(',').map(|s| s.trim().to_string()).collect();
            Some(names)
        } else {
            None
        }
    }

    /// 解析导出声明: export const x = 1
    fn parse_export_declaration(&self, line: &str) -> Option<String> {
        if line.starts_with("export ") {
            Some(line["export ".len()..].to_string())
        } else {
            None
        }
    }

    /// 从声明中提取变量名
    fn extract_variable_name(&self, decl: &str) -> Option<String> {
        // const x = ... 或 let x = ... 或 var x = ... 或 function x(...
        let regex = Regex::new(r#"(?:const|let|var|function|class)\s+(\w+)"#).unwrap();
        if let Some(cap) = regex.captures(decl) {
            Some(cap.get(1).unwrap().as_str().to_string())
        } else {
            None
        }
    }
}

impl Default for EsModules {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports() {
        let esm = EsModules::new();
        let code = r#"
            import { foo, bar } from './utils';
            import * as helpers from './helpers';
            import Vue from 'vue';
            import './styles.css';
        "#;
        
        let imports = esm.parse_imports(code);
        assert_eq!(imports.len(), 4);
        assert!(imports.contains(&"./utils".to_string()));
        assert!(imports.contains(&"./helpers".to_string()));
        assert!(imports.contains(&"vue".to_string()));
        assert!(imports.contains(&"./styles.css".to_string()));
    }

    #[test]
    fn test_transform_named_import() {
        let esm = EsModules::new();
        let code = r#"import { foo, bar } from './utils';"#;
        let transformed = esm.transform_module("test.js", code);
        
        assert!(transformed.contains("__getModule('./utils')"));
        assert!(transformed.contains("var foo = __mod['foo']"));
        assert!(transformed.contains("var bar = __mod['bar']"));
    }

    #[test]
    fn test_transform_export_default() {
        let esm = EsModules::new();
        let code = r#"export default function App() {}"#;
        let transformed = esm.transform_module("test.js", code);
        
        assert!(transformed.contains("exports['default']"));
    }

    #[test]
    fn test_transform_export_named() {
        let esm = EsModules::new();
        let code = r#"export const name = 'test';"#;
        let transformed = esm.transform_module("test.js", code);
        
        assert!(transformed.contains("const name = 'test'"));
        assert!(transformed.contains("exports['name'] = name"));
    }

    #[test]
    fn test_transform_export_object() {
        let esm = EsModules::new();
        let code = r#"
            const a = 1;
            const b = 2;
            export { a, b };
        "#;
        let transformed = esm.transform_module("test.js", code);
        
        assert!(transformed.contains("exports['a'] = a"));
        assert!(transformed.contains("exports['b'] = b"));
    }

    #[test]
    fn test_transform_namespace_import() {
        let esm = EsModules::new();
        let code = r#"import * as utils from './utils';"#;
        let transformed = esm.transform_module("test.js", code);
        
        assert!(transformed.contains("var utils = __getModule('./utils')"));
    }

    #[test]
    fn test_transform_default_import() {
        let esm = EsModules::new();
        let code = r#"import Vue from 'vue';"#;
        let transformed = esm.transform_module("test.js", code);
        
        assert!(transformed.contains("var Vue = __mod['default'] || __mod"));
    }
}
