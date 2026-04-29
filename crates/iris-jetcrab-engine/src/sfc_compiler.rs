//! Vue SFC 编译器
//!
//! 提供 Vue 单文件组件的编译、依赖解析和模块路径解析功能

use anyhow::{Result, Context};
use tracing::debug;
use serde::{Serialize, Deserialize};

/// 编译后的模块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledModule {
    /// 转换后的 JavaScript 代码
    pub script: String,
    /// 样式块列表
    pub styles: Vec<StyleBlock>,
    /// 依赖列表
    pub deps: Vec<String>,
}

/// 样式块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleBlock {
    /// CSS 代码
    pub code: String,
    /// 是否启用作用域
    pub scoped: bool,
}

/// 编译 Vue SFC 文件
pub fn compile_sfc(source: &str, filename: &str) -> Result<CompiledModule> {
    debug!("Compiling SFC: {}", filename);

    // 使用 iris-sfc 进行编译
    let parsed = iris_sfc::compile_from_string(filename, source)
        .context(format!("Failed to parse {}", filename))?;

    // 合并 render 函数到 script 中
    let script = if !parsed.render_fn.is_empty() {
        // 如果 script 为空，创建完整的组件导出
        let script_with_render = if parsed.script.is_empty() || parsed.script.trim() == "export default {}" {
            format!(
                "export default {{\n  render: {}\n}}",
                parsed.render_fn
            )
        } else {
            // 将 render 函数添加到已有的 export default 中
            // 查找 export default { 并注入 render
            if parsed.script.contains("export default {") {
                parsed.script.replace(
                    "export default {",
                    &format!("export default {{\n  render: {},", parsed.render_fn)
                )
            } else if parsed.script.contains("export default") {
                // 如果是 export default { ... } 格式
                format!(
                    "{{\n  render: {}\n}}\n{}",
                    parsed.render_fn, parsed.script
                )
            } else {
                // 没有 export default，添加一个
                format!(
                    "export default {{\n  render: {},\n  ...{}\n}}",
                    parsed.render_fn, parsed.script
                )
            }
        };
        
        // 添加 h 函数的导入（如果还没有的话）
        if !script_with_render.contains("import { h }") && !script_with_render.contains("import {h}") {
            debug!("Adding h function import from vue");
            format!("import {{ h }} from 'vue';\n{}", script_with_render)
        } else {
            script_with_render
        }
    } else {
        parsed.script.clone()
    };

    // 提取样式部分
    let styles: Vec<StyleBlock> = parsed
        .styles
        .iter()
        .map(|style| StyleBlock {
            code: style.css.clone(),
            scoped: style.scoped,
        })
        .collect();

    // 解析依赖
    let deps = parse_dependencies(&script)?;

    Ok(CompiledModule {
        script,
        styles,
        deps,
    })
}

/// 解析模块导入路径
pub fn resolve_module(import_path: &str, importer: &str) -> Result<String> {
    debug!("Resolving import: {} from {}", import_path, importer);

    // 如果是相对路径
    if import_path.starts_with('.') || import_path.starts_with('/') {
        // 简化处理：拼接路径
        if let Some(pos) = importer.rfind('/') {
            let base_dir = &importer[..pos + 1];
            let resolved = format!("{}{}", base_dir, import_path.trim_start_matches("./"));
            
            // 添加默认扩展名
            if !resolved.ends_with(".vue") && !resolved.ends_with(".js") {
                return Ok(format!("{}.vue", resolved));
            }
            
            return Ok(resolved);
        }
    }

    // 如果是裸模块名（npm 包）
    Ok(import_path.to_string())
}

/// 解析脚本中的依赖
fn parse_dependencies(script: &str) -> Result<Vec<String>> {
    let mut deps = Vec::new();
    
    // 简单的 import 语句解析
    for line in script.lines() {
        let line = line.trim();
        
        // 匹配: import ... from '...'
        if line.starts_with("import ") {
            if let Some(start) = line.find('\'') {
                if let Some(end) = line.rfind('\'') {
                    if start < end {
                        let dep = &line[start + 1..end];
                        deps.push(dep.to_string());
                    }
                }
            }
        }
        
        // 匹配: import(...) 动态导入
        if line.contains("import(") {
            if let Some(start) = line.find('\'') {
                if let Some(end) = line[start+1..].find('\'') {
                    let dep = &line[start + 1..start + 1 + end];
                    deps.push(dep.to_string());
                }
            }
        }
    }

    Ok(deps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dependencies() {
        let script = r#"
            import { ref } from 'vue';
            import Foo from './components/Foo.vue';
            import Bar from '../Bar';
        "#;

        let deps = parse_dependencies(script).unwrap();
        assert!(deps.contains(&"vue".to_string()));
        assert!(deps.contains(&"./components/Foo.vue".to_string()));
        assert!(deps.contains(&"../Bar".to_string()));
    }

    #[test]
    fn test_parse_dependencies_dynamic_import() {
        let script = r#"
            const module = await import('./lazy.vue');
        "#;

        let deps = parse_dependencies(script).unwrap();
        assert!(deps.contains(&"./lazy.vue".to_string()));
    }

    #[test]
    fn test_resolve_module() {
        // 相对路径解析
        let resolved = resolve_module("./Foo.vue", "/src/App.vue").unwrap();
        assert_eq!(resolved, "/src/Foo.vue");

        // 裸模块名
        let resolved = resolve_module("vue", "/src/App.vue").unwrap();
        assert_eq!(resolved, "vue");
    }

    #[test]
    fn test_resolve_module_adds_extension() {
        // 没有扩展名时添加 .vue
        let resolved = resolve_module("./Foo", "/src/App.vue").unwrap();
        assert_eq!(resolved, "/src/Foo.vue");
    }
}
