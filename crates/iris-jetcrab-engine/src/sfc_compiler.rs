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

/// 将 render 函数注入到 setup 函数中
/// 替换 `return { ... };` 为 `return render_fn;`
/// 这样 render 作为闭包可以访问 setup 中的局部变量（如 Pinia store）
/// Vue 3 支持 setup 返回函数作为渲染函数
fn inject_render_into_setup_return(script: &str, render_fn: &str) -> String {
    if !script.contains("return {") {
        // 没有 return 语句，回退到在 export default 层添加 render
        return script.replace(
            "export default {",
            &format!("export default {{\n  render: {},", render_fn)
        );
    }
    
    // 找到最后一个 "return {"（应该是 setup 的 return）
    if let Some(return_pos) = script.rfind("return {") {
        let before = &script[..return_pos];
        let after_return = &script[return_pos + 8..]; // 跳过 "return {"
        
        // 查找匹配的 }
        let mut depth = 1i32;
        let mut end_idx: Option<usize> = None;
        for (i, ch) in after_return.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_idx = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        
        if let Some(end) = end_idx {
            // after_stmt = "};..." or "}..."
            let after_stmt = &after_return[end..];
            // 跳过 } 和可选的 ;
            let after_close = &after_stmt[1..]; // 跳过 }
            let after_close = if after_close.starts_with(';') {
                &after_close[1..] // 跳过 ;
            } else {
                after_close
            };
            
            // 替换整个 "return { ... };" 为 "return render_fn"
            format!("{}return {}{}", before, render_fn, after_close)
        } else {
            // 无法找到匹配的 }，回退
            script.replace(
                "export default {",
                &format!("export default {{\n  render: {},", render_fn)
            )
        }
    } else {
        script.to_string()
    }
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
            // 将 render 函数注入到 setup 的 return 语句中
            // 这样 render 作为闭包可以访问 setup 中的局部变量（如 Pinia store）
            if parsed.script.contains("export default {") {
                inject_render_into_setup_return(&parsed.script, &parsed.render_fn)
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

/// 规范化文件路径，统一使用 / 作为分隔符
/// 确保 Windows 和 Unix 系统返回一致的路径格式
fn normalize_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    // 清理路径中的 ./（如 /src/./Foo.vue → /src/Foo.vue）
    normalized.replace("/./", "/")
}

/// 解析模块导入路径
pub fn resolve_module(import_path: &str, importer: &str) -> Result<String> {
    debug!("Resolving import: {} from {}", import_path, importer);

    // 如果是相对路径
    if import_path.starts_with('.') || import_path.starts_with('/') {
        // 使用 Path::parent() 获取导入者所在的目录，兼容 Windows 和 Unix 路径
        let importer_path = std::path::Path::new(importer);
        if let Some(parent_dir) = importer_path.parent() {
            // 使用 Path::join 正确处理 ../ 和 ./ 路径
            let resolved = parent_dir.join(import_path);

            // 规范化路径分隔符为 /
            let resolved_str = normalize_path(&resolved.to_string_lossy());
            let normalized_path = std::path::Path::new(&resolved_str);

            // 1. 如果路径已存在且是文件，直接返回
            if normalized_path.is_file() {
                return Ok(resolved_str);
            }

            // 2. 尝试常见扩展名
            let extensions = [".vue", ".ts", ".tsx", ".js", ".jsx", ".mjs"];
            for ext in &extensions {
                let with_ext_str = format!("{}{}", resolved_str, ext);
                if std::path::Path::new(&with_ext_str).is_file() {
                    return Ok(with_ext_str);
                }
            }

            // 3. 如果是目录，尝试 index 文件
            if normalized_path.is_dir() {
                let index_files = ["index.ts", "index.js", "index.tsx", "index.jsx", "index.mjs"];
                for index_file in &index_files {
                    let index_str = format!("{}/{}", resolved_str, index_file);
                    if std::path::Path::new(&index_str).is_file() {
                        return Ok(index_str);
                    }
                }
            }

            // 4. 所有尝试都失败，回退到添加 .vue 后缀
            if !resolved_str.ends_with(".vue") && !resolved_str.ends_with(".js") {
                return Ok(format!("{}.vue", resolved_str));
            }
            return Ok(resolved_str);
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
