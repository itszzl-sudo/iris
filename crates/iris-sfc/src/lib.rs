//! Iris SFC —— SFC/TS 即时转译层
//!
//! 核心使命：零编译直接运行源码。
//! 解析 .vue 文件，提取 template/script/style，编译为可执行模块。
//!
//! **注意**：当前实现是简化版本（演示用途），用于验证热重载流程。
//! 完整的模板编译器和 TypeScript 转译器将在后续版本实现。

#![warn(missing_docs)]

mod cache;
pub use iris_cssom::css_modules;
mod scoped_css;
mod scss_processor;
mod script_setup;
mod template_compiler;
pub mod ts_compiler;
pub mod postcss_processor;
pub mod less_processor;

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;
use tracing::{debug, info, warn};

pub use cache::*;

/// 预编译的正则表达式（性能优化：避免每次调用时重新编译）。
///
/// 性能对比：
/// - 每次编译：~10-50μs
/// - LazyLock 单次编译：~0.1μs
/// - 性能提升：100-500 倍
static TEMPLATE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"(?s)<template\b[^>]*>(.*?)</\s*template\s*>"#).unwrap());

static SCRIPT_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"(?s)<script\b([^>]*)>(.*?)</\s*script\s*>"#).unwrap());

/// 全局 TypeScript 编译器实例（复用，避免重复创建）
///
/// 使用 LazyLock 确保：
/// 1. 线程安全的懒初始化
/// 2. 整个生命周期只创建一个 TsCompiler 实例
/// 3. 复用内部缓存和 SourceMap
/// 4. 禁用 Source Map 以节省内存和提升编译速度
static TS_COMPILER: LazyLock<ts_compiler::TsCompiler> = LazyLock::new(|| {
    // 从环境变量读取 Source Map 配置
    let enable_source_map = std::env::var("IRIS_SOURCE_MAP")
        .map(|v| v == "true" || v == "1" || v == "yes")
        .unwrap_or(false); // 默认禁用

    ts_compiler::TsCompiler::new(ts_compiler::TsCompilerConfig {
        source_map: enable_source_map,
        ..Default::default()
    })
});

/// 全局 SFC 缓存实例（用于热重载加速）
///
/// 使用 LazyLock 确保：
/// 1. 线程安全的懒初始化
/// 2. 整个生命周期只创建一个缓存实例
/// 3. 默认容量 100 项，自动 LRU 淘汰
/// 4. 基于源码哈希，确保内容一致性
static SFC_CACHE: LazyLock<SfcCache> = LazyLock::new(|| {
    // 从环境变量读取缓存配置
    let capacity = std::env::var("IRIS_CACHE_CAPACITY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100); // 默认 100 项

    let enabled = std::env::var("IRIS_CACHE_ENABLED")
        .map(|v| v != "false" && v != "0" && v != "no")
        .unwrap_or(true); // 默认启用

    SfcCache::new(SfcCacheConfig { capacity, enabled })
});

static STYLE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"(?s)<style\b([^>]*)>(.*?)</\s*style\s*>"#).unwrap());

/// Vue SFC 编译结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfcModule {
    /// 组件名称（从文件名提取）。
    pub name: String,
    /// Template 编译结果（渲染函数）。
    pub render_fn: String,
    /// Script 编译结果（JavaScript）。
    pub script: String,
    /// Style 编译结果（CSS）。
    pub styles: Vec<StyleBlock>,
    /// 源码哈希（用于缓存验证）。
    pub source_hash: u64,
}

/// 样式块。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleBlock {
    /// CSS 内容。
    pub css: String,
    /// 是否 scoped。
    pub scoped: bool,
    /// 样式语言（css/scss/less）。
    pub lang: String,
    /// 是否启用 CSS Modules。
    pub module: bool,
    /// 类名映射（仅 module=true 时有值）。
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub class_mapping: std::collections::HashMap<String, String>,
}

/// SFC 解析结果（中间表示）。
#[derive(Debug)]
struct SfcDescriptor {
    /// Template 原始源码。
    template: Option<String>,
    /// Script 原始源码。
    script: Option<String>,
    /// Script 属性（lang, setup）。
    script_attrs: script_setup::ScriptAttrs,
    /// Style 原始源码列表。
    styles: Vec<StyleRaw>,
}

/// 原始样式块（编译前）。
#[derive(Debug)]
struct StyleRaw {
    content: String,
    scoped: bool,
    lang: String,
    module: bool,
}

/// 编译错误类型（包含位置信息）。
#[derive(Debug, thiserror::Error)]
pub enum SfcError {
    /// 文件读取失败。
    #[error("❌ Failed to read file: {file}\n   Reason: {source}\n   💡 Suggestion: Check if file exists and you have read permissions.")]
    IoError {
        /// The underlying IO error.
        source: std::io::Error,
        /// The file name being processed.
        file: String,
    },

    /// SFC 格式错误。
    #[error("❌ Parse error at {file}:{line}:{column}\n   {message}\n   💡 Suggestion: Ensure .vue file has at least <template> or <script> tag.")]
    ParseError {
        /// Error message.
        message: String,
        /// File name.
        file: String,
        /// Line number (1-based).
        line: usize,
        /// Column number (1-based).
        column: usize,
    },

    /// Template 编译失败。
    #[error("❌ Template error at {file}:{line}:{column}\n   {message}\n   💡 Suggestion: Check for invalid HTML syntax or unsupported directives.")]
    TemplateError {
        /// Error message.
        message: String,
        /// File name.
        file: String,
        /// Line number (1-based).
        line: usize,
        /// Column number (1-based).
        column: usize,
    },

    /// Script 转译失败。
    #[error("❌ Script error at {file}:{line}:{column}\n   {message}\n   💡 Suggestion: Check TypeScript syntax and ensure all compiler macros are valid.")]
    ScriptError {
        /// Error message.
        message: String,
        /// File name.
        file: String,
        /// Line number (1-based).
        line: usize,
        /// Column number (1-based).
        column: usize,
    },
}

/// 错误严重性级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// 致命错误，编译失败
    Fatal,
    /// 警告，编译成功但有潜在问题
    Warning,
    /// 信息提示
    Info,
}

impl SfcError {
    /// 获取错误严重性级别
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            SfcError::IoError { .. } => ErrorSeverity::Fatal,
            SfcError::ParseError { .. } => ErrorSeverity::Fatal,
            SfcError::TemplateError { .. } => ErrorSeverity::Fatal,
            SfcError::ScriptError { .. } => ErrorSeverity::Fatal,
        }
    }

    /// 获取文件名
    pub fn file(&self) -> &str {
        match self {
            SfcError::IoError { file, .. } => file,
            SfcError::ParseError { file, .. } => file,
            SfcError::TemplateError { file, .. } => file,
            SfcError::ScriptError { file, .. } => file,
        }
    }

    /// 格式化为人类可读的错误信息（带颜色支持）
    pub fn format_pretty(&self, use_color: bool) -> String {
        let reset = if use_color { "\x1b[0m" } else { "" };
        let red = if use_color { "\x1b[31m" } else { "" };
        let _yellow = if use_color { "\x1b[33m" } else { "" };
        let cyan = if use_color { "\x1b[36m" } else { "" };

        format!(
            "{}error{}: {}\n{}help{}: {}",
            red,
            reset,
            self,
            cyan,
            reset,
            self.help_message()
        )
    }

    /// 获取帮助信息
    fn help_message(&self) -> String {
        match self {
            SfcError::IoError { source, file } => match source.kind() {
                std::io::ErrorKind::NotFound => {
                    format!("File '{}' not found. Check the file path.", file)
                }
                std::io::ErrorKind::PermissionDenied => {
                    format!(
                        "Permission denied when reading '{}'. Check file permissions.",
                        file
                    )
                }
                _ => {
                    format!("IO error occurred while reading '{}': {}", file, source)
                }
            },
            SfcError::ParseError { message, .. } => {
                if message.contains("must have at least") {
                    "A .vue file must contain either a <template> or <script> tag.".to_string()
                } else {
                    format!("Parse error: {}", message)
                }
            }
            SfcError::TemplateError { message, .. } => {
                if message.contains("v-") {
                    format!("Invalid Vue directive in template: {}", message)
                } else {
                    format!("Template syntax error: {}", message)
                }
            }
            SfcError::ScriptError { message, .. } => {
                if message.contains("TypeScript") {
                    format!("TypeScript compilation failed: {}\nConsider running 'tsc --noEmit' for detailed type checking.", message)
                } else if message.contains("Script setup") {
                    format!("Script setup transformation failed: {}\nCheck defineProps and defineEmits syntax.", message)
                } else {
                    format!("Script error: {}", message)
                }
            }
        }
    }
}

impl From<std::io::Error> for SfcError {
    fn from(err: std::io::Error) -> Self {
        SfcError::IoError {
            source: err,
            file: String::from("unknown"),
        }
    }
}

/// 编译 .vue 文件。
///
/// # 参数
///
/// * `path` - .vue 文件路径
///
/// # 返回
///
/// 返回编译后的 SFC 模块。
///
/// # 示例
///
/// ```ignore
/// use iris_sfc::compile;
///
/// let module = compile("App.vue")?;
/// println!("Component: {}", module.name);
/// println!("Render function: {}", module.render_fn);
/// ```
pub fn compile<P: AsRef<Path>>(path: P) -> Result<SfcModule, SfcError> {
    let path = path.as_ref();
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("unknown"));

    info!(path = ?path, "Compiling Vue SFC");

    // 读取文件
    let source = std::fs::read_to_string(path).map_err(|e| SfcError::IoError {
        source: e,
        file: file_name.clone(),
    })?;

    // 提取组件名
    let name = extract_component_name(path);

    // 使用缓存编译（自动处理缓存命中/未命中）
    let start_time = std::time::Instant::now();
    let module = SFC_CACHE
        .get_or_compile(&name, &source, || {
            // 实际编译逻辑（仅在缓存未命中时执行）
            compile_sfc_internal(&name, &source, &file_name)
        })
        .map_err(|e| SfcError::ScriptError {
            message: e,
            file: file_name.clone(),
            line: 1,
            column: 1,
        })?;

    let compile_time = start_time.elapsed();

    // 记录编译时间日志
    debug!(
        name = %name,
        compile_time_ms = compile_time.as_secs_f64() * 1000.0,
        from_cache = module.source_hash != calculate_hash(&source),  // 如果哈希匹配说明是缓存
        "SFC compilation completed"
    );

    Ok(module)
}

/// 内部编译函数（实际执行编译逻辑）
///
/// 该函数仅在缓存未命中时被调用。
///
/// # 参数
///
/// * `name` - 组件名称
/// * `source` - SFC 源码
/// * `file_name` - 文件名（用于错误定位）
///
/// # 返回
///
/// 返回编译后的 SFC 模块
fn compile_sfc_internal(name: &str, source: &str, file_name: &str) -> Result<SfcModule, String> {
    let source_hash = calculate_hash(source);
    let descriptor = parse_sfc(source, file_name).map_err(|e| format!("Parse error: {}", e))?;

    let render_fn = compile_template(file_name, descriptor.template.as_deref().unwrap_or(""))
        .map_err(|e| format!("Template compile error: {}", e))?;

    let script = if let Some(script_source) = &descriptor.script {
        compile_script(file_name, script_source, &descriptor.script_attrs)
            .map_err(|e| format!("Script compile error: {}", e))?
    } else {
        String::new()
    };

    // 类型检查（如果启用）
    // 注意：对转换后的脚本进行类型检查（包含宏展开）
    if let Some(script_source) = &descriptor.script {
        let type_check_config = ts_compiler::TypeCheckConfig::default();

        if type_check_config.enabled {
            debug!("Running type check...");

            // 使用转换后的脚本进行类型检查
            let script_for_check = if descriptor.script_attrs.setup {
                // 如果是 script setup，使用转换后的脚本
                script.clone()
            } else {
                script_source.clone()
            };

            match (&*TS_COMPILER).type_check(&script_for_check, file_name, &type_check_config) {
                ts_compiler::TypeCheckResult::Success => {
                    debug!("Type check passed");
                }
                ts_compiler::TypeCheckResult::Errors { errors } => {
                    warn!(error_count = errors.len(), "Type check failed (non-fatal)");
                    // 注意：类型检查失败不阻断编译，仅警告
                    // 如果需要阻断，可以返回错误：
                    // return Err(format!("Type check failed:\n{}", errors.join("\n")));
                }
                ts_compiler::TypeCheckResult::Skipped => {
                    debug!("Type check skipped");
                }
            }
        }
    }

    let styles = compile_styles(&descriptor.styles);

    debug!(
        name = %name,
        has_template = descriptor.template.is_some(),
        has_script = descriptor.script.is_some(),
        style_count = styles.len(),
        "SFC internal compilation completed"
    );

    Ok(SfcModule {
        name: name.to_string(),
        render_fn,
        script,
        styles,
        source_hash,
    })
}

/// 从字符串编译 .vue 文件（用于测试）。
///
/// # 参数
///
/// * `name` - 组件名称
/// * `source` - .vue 源码字符串
///
/// # 返回
///
/// 返回编译后的 SFC 模块。
pub fn compile_from_string(name: &str, source: &str) -> Result<SfcModule, SfcError> {
    let source_hash = calculate_hash(source);
    let descriptor = parse_sfc(source, name)?;

    let render_fn = compile_template(name, descriptor.template.as_deref().unwrap_or(""))?;
    let script = if let Some(script_source) = &descriptor.script {
        compile_script(name, script_source, &descriptor.script_attrs)?
    } else {
        String::new()
    };
    let styles = compile_styles(&descriptor.styles);

    Ok(SfcModule {
        name: name.to_string(),
        render_fn,
        script,
        styles,
        source_hash,
    })
}

/// SFC 解析器。
///
/// 使用预编译的正则表达式提取 template/script/style 块。
///
/// # 参数
///
/// * `source` - .vue 源码字符串
/// * `file_name` - 文件名（用于错误定位）
///
/// # 返回
///
/// 返回解析后的 SFC 描述符。
fn parse_sfc(source: &str, file_name: &str) -> Result<SfcDescriptor, SfcError> {
    let mut template = None;
    let mut script = None;
    let mut styles = Vec::new();

    // 提取 <template> 部分（使用预编译正则，支持多行）
    if let Some(caps) = TEMPLATE_RE.captures(source) {
        if let Some(content) = caps.get(1) {
            template = Some(content.as_str().to_string());
            debug!(template_size = content.as_str().len(), "Template extracted");
        }
    }

    // 提取 <script> 部分（支持属性如 lang="ts", setup）
    let mut script_attrs = script_setup::ScriptAttrs {
        lang: "javascript".to_string(),
        setup: false,
    };

    if let Some(caps) = SCRIPT_RE.captures(source) {
        if let Some(attrs_match) = caps.get(1) {
            script_attrs = script_setup::parse_script_attrs(attrs_match.as_str());
        }
        if let Some(content) = caps.get(2) {
            script = Some(content.as_str().to_string());
            debug!(script_size = content.as_str().len(), "Script extracted");
        }
    }

    // 提取所有 <style> 部分（支持多个样式块）
    for caps in STYLE_RE.captures_iter(source) {
        let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        let scoped = attrs.contains("scoped");
        let module = attrs.contains("module");
        let lang = extract_lang(attrs);

        debug!(
            style_size = content.len(),
            scoped,
            module,
            lang = %lang,
            "Style extracted"
        );

        styles.push(StyleRaw {
            content: content.to_string(),
            scoped,
            module,
            lang,
        });
    }

    // SFC 至少要有一个 template 或 script（允许纯逻辑组件）
    if template.is_none() && script.is_none() {
        return Err(SfcError::ParseError {
            message: "SFC must have at least <template> or <script>".to_string(),
            file: file_name.to_string(),
            line: 1,
            column: 1,
        });
    }

    debug!(
        has_template = template.is_some(),
        has_script = script.is_some(),
        style_count = styles.len(),
        "SFC parsing complete"
    );

    Ok(SfcDescriptor {
        template,
        script,
        script_attrs,
        styles,
    })
}

/// 从标签属性中提取 lang 属性。
///
/// 例如：`<script lang="ts">` → `"ts"`
fn extract_lang(attrs: &str) -> String {
    if let Some(start) = attrs.find("lang=\"") {
        let start = start + 6;
        if let Some(end) = attrs[start..].find('"') {
            return attrs[start..start + end].to_string();
        }
    }
    "css".to_string()
}

/// Template 编译器（完整版）。
///
/// 使用 html5ever 解析 HTML 模板，生成虚拟 DOM 创建函数。
/// 支持 Vue 指令：v-if, v-for, v-bind, v-on, v-model
///
/// # 参数
///
/// * `file_name` - 文件名（用于错误定位）
/// * `template` - 模板源码
///
/// # 返回
///
/// 返回渲染函数字符串。
fn compile_template(file_name: &str, template: &str) -> Result<String, SfcError> {
    if template.is_empty() {
        warn!(file = file_name, "Template is empty");
        return Ok("function render() { return null; }".to_string());
    }

    info!(
        file = file_name,
        "Compiling Vue template with full compiler"
    );

    // 步骤 1: 解析 HTML 为 AST
    let (vnodes, component_names) =
        template_compiler::parse_template(template).map_err(|e| SfcError::TemplateError {
            message: format!("Failed to parse template: {}", e),
            file: file_name.to_string(),
            line: 1,
            column: 1,
        })?;

    // 步骤 2: 生成渲染函数（传入检测到的组件名）
    let render_fn = template_compiler::generate_render_fn_with_components(&vnodes, &component_names);

    debug!(
        file = file_name,
        render_fn_size = render_fn.len(),
        "Template compiled successfully"
    );

    Ok(render_fn)
}

/// Script 编译器（TypeScript 转译）。
///
/// # 注意
///
/// **当前实现是演示版本**：只移除基本类型注解。
/// 完整版本应该集成 swc 或其他 TS 编译器，支持泛型、装饰器、TSX。
///
/// # 参数
///
/// * `file_name` - 文件名（用于错误定位）
/// * `script` - script 源码
/// * `attrs` - script 属性
///
/// # 返回
///
/// 返回转译后的 JavaScript 代码。
fn compile_script(
    file_name: &str,
    script: &str,
    attrs: &script_setup::ScriptAttrs,
) -> Result<String, SfcError> {
    if script.is_empty() {
        return Ok("export default {}".to_string());
    }

    info!(
        file = file_name,
        setup = attrs.setup,
        "Compiling script with swc TypeScript compiler"
    );

    // 1. 如果是 <script setup>，先转换编译器宏
    let processed_script = if attrs.setup {
        debug!("Transforming <script setup> and compiler macros");
        script_setup::transform_script_setup(script).map_err(|e| SfcError::ScriptError {
            message: format!("Script setup transformation failed: {}", e),
            file: file_name.to_string(),
            line: 1,
            column: 1,
        })?
    } else {
        // 普通 <script>，检查是否有 export default
        if !script.contains("export default") {
            debug!("No export default found in <script>, wrapping with export default");
            // 将原始代码包装到 export default 的 setup 函数中
            format!(
                "export default {{\n  setup() {{\n    {}\n  }}\n}}",
                script
            )
        } else {
            script.to_string()
        }
    };

    // 2. 使用全局编译器实例编译 TypeScript
    let result = TS_COMPILER
        .compile(&processed_script, file_name)
        .map_err(|e| SfcError::ScriptError {
            message: format!("TypeScript compilation failed: {}", e),
            file: file_name.to_string(),
            line: 1,
            column: 1,
        })?;

    debug!(
        file = file_name,
        compile_time_ms = result.compile_time_ms,
        output_size = result.code.len(),
        "Script compiled with swc"
    );

    Ok(result.code)
}

/// 编译样式块。
fn compile_styles(styles: &[StyleRaw]) -> Vec<StyleBlock> {
    // PostCSS 配置（全部启用）
    let postcss_config = postcss_processor::PostCssConfig::default();

    styles
        .iter()
        .map(|style| {
            // 第一步：编译 SCSS/Less 为 CSS
            let css_content = if style.lang == "scss" || style.lang == "sass" {
                let config = scss_processor::ScssConfig::default();
                match scss_processor::compile_scss(&style.content, &config) {
                    Ok(result) => {
                        debug!(lang = %style.lang, "SCSS compiled");
                        result.css
                    }
                    Err(e) => {
                        warn!(error = %e, "SCSS compilation failed, using original content");
                        style.content.clone()
                    }
                }
            } else if style.lang == "less" {
                let config = less_processor::LessConfig::default();
                match less_processor::compile_less(&style.content, &config) {
                    Ok(result) => {
                        debug!("Less compiled");
                        result.css
                    }
                    Err(e) => {
                        warn!(error = %e, "Less compilation failed, using original content");
                        style.content.clone()
                    }
                }
            } else {
                // 普通 CSS，直接使用
                style.content.clone()
            };

            // 第二步：PostCSS 转换（SCSS/Less/CSS 编译后的 autoprefixer/nesting）
            let postcss_result = postcss_processor::process_css(
                &css_content,
                &postcss_config,
                &format!("style[lang={}]", style.lang)
            );

            if postcss_result.transformed {
                debug!(
                    "PostCSS applied: {} -> {} bytes",
                    postcss_result.original_size,
                    postcss_result.output_size
                );
            }

            let css_content = postcss_result.css;

            // 第三步：应用 CSS Modules 或 Scoped CSS
            if style.module {
                // CSS Modules: 作用域化类名
                let hash = css_modules::generate_short_hash(&css_content);
                let scoped_css = css_modules::transform_css(&css_content, &hash);
                let class_mapping = css_modules::generate_mapping(&css_content, &hash);

                debug!(
                    hash = %hash,
                    class_count = class_mapping.len(),
                    "CSS Modules compiled"
                );

                StyleBlock {
                    css: scoped_css,
                    scoped: true, // CSS Modules 自动 scoped
                    lang: style.lang.clone(),
                    module: true,
                    class_mapping,
                }
            } else if style.scoped {
                // Scoped CSS: 为选择器添加唯一属性
                let scope_id = scoped_css::generate_scope_id("component", &css_content);
                let scoped_css = scoped_css::transform_css_scoped(&css_content, &scope_id);

                debug!(scope_id = %scope_id, "Scoped CSS applied");

                StyleBlock {
                    css: scoped_css,
                    scoped: true,
                    lang: style.lang.clone(),
                    module: false,
                    class_mapping: std::collections::HashMap::new(),
                }
            } else {
                // 普通样式：保持原样
                StyleBlock {
                    css: css_content,
                    scoped: false,
                    lang: style.lang.clone(),
                    module: false,
                    class_mapping: std::collections::HashMap::new(),
                }
            }
        })
        .collect()
}

/// 从文件路径提取组件名称。
///
/// 例如：`components/App.vue` → `"App"`
fn extract_component_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("Anonymous"))
}

/// 计算字符串的简单哈希。
fn calculate_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_sfc() {
        let source = r#"<template>
  <div>Hello</div>
</template>

<script setup>
const msg = "Hello"
</script>"#;

        let descriptor = parse_sfc(source, "test.vue").unwrap();
        assert!(descriptor.template.is_some());
        assert!(descriptor.script.is_some());
        assert_eq!(descriptor.styles.len(), 0);
    }

    #[test]
    fn test_parse_multiple_styles() {
        let source = r#"<template><div></div></template>

<style scoped>
.a { color: red; }
</style>

<style>
.b { color: blue; }
</style>

<style lang="scss" scoped>
.c { .d { margin: 0; } }
</style>"#;

        let descriptor = parse_sfc(source, "test.vue").unwrap();
        assert_eq!(descriptor.styles.len(), 3);
        assert!(descriptor.styles[0].scoped);
        assert!(!descriptor.styles[1].scoped);
        assert!(descriptor.styles[2].scoped);
        assert_eq!(descriptor.styles[2].lang, "scss");
    }

    #[test]
    fn test_parse_empty_sfc_error() {
        let source = "<div></div>";
        let result = parse_sfc(source, "test.vue");
        assert!(result.is_err());
    }

    #[test]
    fn test_transpile_basic_ts() {
        let ts = r#"const count: number = 0
const name: string = "Iris"
function add(a: number, b: number): number {
  return a + b
}"#;

        // 使用新的 TsCompiler
        use crate::ts_compiler::{TsCompiler, TsCompilerConfig};
        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(ts, "test.ts").unwrap();

        assert!(!result.code.contains(": number"));
        assert!(!result.code.contains(": string"));
        assert!(result.code.contains("function add(a, b)"));
    }

    #[test]
    fn test_extract_lang() {
        assert_eq!(extract_lang("lang=\"ts\""), "ts");
        assert_eq!(extract_lang("scoped lang=\"scss\""), "scss");
        assert_eq!(extract_lang(""), "css");
    }

    #[test]
    fn test_compile_from_string() {
        let source = r#"<template>
  <div>{{ message }}</div>
</template>

<script setup>
const message = "Hello"
</script>"#;

        let module = compile_from_string("TestComponent", source).unwrap();
        assert_eq!(module.name, "TestComponent");
        assert!(module.render_fn.contains("function render()"));
        assert!(module.script.contains("const message"));
    }

    #[test]
    fn test_hash_consistency() {
        let source = "test content";
        let hash1 = calculate_hash(source);
        let hash2 = calculate_hash(source);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_extract_component_name() {
        assert_eq!(
            extract_component_name(Path::new("components/App.vue")),
            "App"
        );
        assert_eq!(
            extract_component_name(Path::new("MyComponent.vue")),
            "MyComponent"
        );
    }

    #[test]
    fn test_css_modules_style() {
        let source = r#"<template>
  <div class="container">
    <button class="button">Click</button>
  </div>
</template>

<script>
export default {
  name: 'TestComponent'
}
</script>

<style module>
.container {
  padding: 20px;
}

.button {
  color: red;
}
</style>"#;

        let module = compile_from_string("TestComponent", source).unwrap();
        assert_eq!(module.name, "TestComponent");
        assert_eq!(module.styles.len(), 1);

        let style = &module.styles[0];
        assert!(style.module, "Style should be a CSS Module");
        assert!(style.scoped, "CSS Module should be automatically scoped");
        assert!(!style.class_mapping.is_empty(), "Should have class mapping");

        // 验证类名被作用域化
        assert!(
            style.css.contains("__"),
            "CSS should contain scoped class names"
        );
        assert!(
            style.class_mapping.contains_key("container"),
            "Should map 'container' class"
        );
        assert!(
            style.class_mapping.contains_key("button"),
            "Should map 'button' class"
        );
    }

    #[test]
    fn test_mixed_styles() {
        let source = r#"<template>
  <div>Test</div>
</template>

<style scoped>
.normal {
  color: blue;
}
</style>

<style module>
.module-class {
  color: red;
}
</style>"#;

        let module = compile_from_string("TestComponent", source).unwrap();
        assert_eq!(module.styles.len(), 2);

        // 第一个样式：普通 scoped
        assert!(!module.styles[0].module);
        assert!(module.styles[0].scoped);

        // 第二个样式：CSS Module
        assert!(module.styles[1].module);
        assert!(module.styles[1].scoped);
        assert!(!module.styles[1].class_mapping.is_empty());
    }
}

/// Initialize the SFC compiler layer.
///
/// This function is called by the main Iris engine initialization chain.
/// Currently, it only logs the initialization event. Pre-compiled regex patterns
/// are automatically initialized on first use via `LazyLock`.
///
/// # Safety
/// This function is safe to call multiple times (idempotent).
///
/// # Example
///
/// ```ignore
/// use iris_sfc::init;
/// init(); // Initialize SFC compiler
/// ```
pub fn init() {
    info!("Iris SFC compiler initialized");
}
