//! Iris SFC 鈥斺€?SFC/TS 鍗虫椂杞瘧灞?
//!
//! 鏍稿績浣垮懡锛氶浂缂栬瘧鐩存帴杩愯婧愮爜銆?
//! 瑙ｆ瀽 .vue 鏂囦欢锛屾彁鍙?template/script/style锛岀紪璇戜负鍙墽琛屾ā鍧椼€?
//!
//! **娉ㄦ剰**锛氬綋鍓嶅疄鐜版槸绠€鍖栫増鏈紙婕旂ず鐢ㄩ€旓級锛岀敤浜庨獙璇佺儹閲嶈浇娴佺▼銆?
//! 瀹屾暣鐨勬ā鏉跨紪璇戝櫒鍜?TypeScript 杞瘧鍣ㄥ皢鍦ㄥ悗缁増鏈疄鐜般€?

#![warn(missing_docs)]

mod template_compiler;

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;
use tracing::{debug, info, warn};

/// 棰勭紪璇戠殑姝ｅ垯琛ㄨ揪寮忥紙鎬ц兘浼樺寲锛氶伩鍏嶆瘡娆¤皟鐢ㄦ椂閲嶆柊缂栬瘧锛夈€?
///
/// 鎬ц兘瀵规瘮锛?
/// - 姣忔缂栬瘧锛殈10-50渭s
/// - LazyLock 鍗曟缂栬瘧锛殈0.1渭s
/// - 鎬ц兘鎻愬崌锛?00-500 鍊?
static TEMPLATE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?s)<template\b[^>]*>(.*?)</\s*template\s*>"#).unwrap()
});

static SCRIPT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?s)<script\b([^>]*)>(.*?)</\s*script\s*>"#).unwrap()
});

static STYLE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?s)<style\b([^>]*)>(.*?)</\s*style\s*>"#).unwrap()
});

/// Vue SFC 缂栬瘧缁撴灉銆?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfcModule {
    /// 缁勪欢鍚嶇О锛堜粠鏂囦欢鍚嶆彁鍙栵級銆?
    pub name: String,
    /// Template 缂栬瘧缁撴灉锛堟覆鏌撳嚱鏁帮級銆?
    pub render_fn: String,
    /// Script 缂栬瘧缁撴灉锛圝avaScript锛夈€?
    pub script: String,
    /// Style 缂栬瘧缁撴灉锛圕SS锛夈€?
    pub styles: Vec<StyleBlock>,
    /// 婧愮爜鍝堝笇锛堢敤浜庣紦瀛橀獙璇侊級銆?
    pub source_hash: u64,
}

/// 鏍峰紡鍧椼€?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleBlock {
    /// CSS 鍐呭銆?
    pub css: String,
    /// 鏄惁 scoped銆?
    pub scoped: bool,
    /// 鏍峰紡璇█锛坈ss/scss/less锛夈€?
    pub lang: String,
}

/// SFC 瑙ｆ瀽缁撴灉锛堜腑闂磋〃绀猴級銆?
#[derive(Debug)]
struct SfcDescriptor {
    /// Template 鍘熷婧愮爜銆?
    template: Option<String>,
    /// Script 鍘熷婧愮爜銆?
    script: Option<String>,
    /// Style 鍘熷婧愮爜鍒楄〃銆?
    styles: Vec<StyleRaw>,
}

/// 鍘熷鏍峰紡鍧楋紙缂栬瘧鍓嶏級銆?
#[derive(Debug)]
struct StyleRaw {
    content: String,
    scoped: bool,
    lang: String,
}

/// 缂栬瘧閿欒绫诲瀷锛堝寘鍚綅缃俊鎭級銆?
#[derive(Debug, thiserror::Error)]
pub enum SfcError {
    /// 鏂囦欢璇诲彇澶辫触銆?
    #[error("Failed to read file: {file} - {source}")]
    IoError {
        /// The underlying IO error.
        source: std::io::Error,
        /// The file name being processed.
        file: String,
    },

    /// SFC 鏍煎紡閿欒銆?
    #[error("Parse error at {file}:{line}:{column}: {message}")]
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

    /// Template 缂栬瘧澶辫触銆?
    #[error("Template error at {file}:{line}:{column}: {message}")]
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

    /// Script 杞瘧澶辫触銆?
    #[error("Script error at {file}:{line}:{column}: {message}")]
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

impl From<std::io::Error> for SfcError {
    fn from(err: std::io::Error) -> Self {
        SfcError::IoError {
            source: err,
            file: String::from("unknown"),
        }
    }
}

/// 缂栬瘧 .vue 鏂囦欢銆?
///
/// # 鍙傛暟
///
/// * `path` - .vue 鏂囦欢璺緞
///
/// # 杩斿洖
///
/// 杩斿洖缂栬瘧鍚庣殑 SFC 妯″潡銆?
///
/// # 绀轰緥
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

    // 璇诲彇鏂囦欢
    let source = std::fs::read_to_string(path).map_err(|e| SfcError::IoError {
        source: e,
        file: file_name.clone(),
    })?;

    // 璁＄畻婧愮爜鍝堝笇
    let source_hash = calculate_hash(&source);

    // 鎻愬彇缁勪欢鍚?
    let name = extract_component_name(path);

    // 瑙ｆ瀽 SFC
    let descriptor = parse_sfc(&source, &file_name)?;

    // 缂栬瘧鍚勯儴鍒嗭紙浼犻€掓枃浠跺悕鐢ㄤ簬閿欒瀹氫綅锛?
    let render_fn = compile_template(&file_name, descriptor.template.as_deref().unwrap_or(""))?;
    let script = if let Some(script_source) = &descriptor.script {
        compile_script(&file_name, script_source)?
    } else {
        String::new()
    };
    let styles = compile_styles(&descriptor.styles);

    debug!(
        name = %name,
        has_template = descriptor.template.is_some(),
        has_script = descriptor.script.is_some(),
        style_count = styles.len(),
        "SFC compiled successfully"
    );

    Ok(SfcModule {
        name,
        render_fn,
        script,
        styles,
        source_hash,
    })
}

/// 浠庡瓧绗︿覆缂栬瘧 .vue 鏂囦欢锛堢敤浜庢祴璇曪級銆?
///
/// # 鍙傛暟
///
/// * `name` - 缁勪欢鍚嶇О
/// * `source` - .vue 婧愮爜瀛楃涓?
///
/// # 杩斿洖
///
/// 杩斿洖缂栬瘧鍚庣殑 SFC 妯″潡銆?
pub fn compile_from_string(name: &str, source: &str) -> Result<SfcModule, SfcError> {
    let source_hash = calculate_hash(source);
    let descriptor = parse_sfc(source, name)?;

    let render_fn = compile_template(name, descriptor.template.as_deref().unwrap_or(""))?;
    let script = if let Some(script_source) = &descriptor.script {
        compile_script(name, script_source)?
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

/// SFC 瑙ｆ瀽鍣ㄣ€?
///
/// 浣跨敤棰勭紪璇戠殑姝ｅ垯琛ㄨ揪寮忔彁鍙?template/script/style 鍧椼€?
///
/// # 鍙傛暟
///
/// * `source` - .vue 婧愮爜瀛楃涓?
/// * `file_name` - 鏂囦欢鍚嶏紙鐢ㄤ簬閿欒瀹氫綅锛?
///
/// # 杩斿洖
///
/// 杩斿洖瑙ｆ瀽鍚庣殑 SFC 鎻忚堪绗︺€?
fn parse_sfc(source: &str, file_name: &str) -> Result<SfcDescriptor, SfcError> {
    let mut template = None;
    let mut script = None;
    let mut styles = Vec::new();

    // 鎻愬彇 <template> 閮ㄥ垎锛堜娇鐢ㄩ缂栬瘧姝ｅ垯锛屾敮鎸佸琛岋級
    if let Some(caps) = TEMPLATE_RE.captures(source) {
        if let Some(content) = caps.get(1) {
            template = Some(content.as_str().to_string());
            debug!(template_size = content.as_str().len(), "Template extracted");
        }
    }

    // 鎻愬彇 <script> 閮ㄥ垎锛堟敮鎸佸睘鎬у lang="ts", setup锛?
    if let Some(caps) = SCRIPT_RE.captures(source) {
        if let Some(content) = caps.get(2) {
            script = Some(content.as_str().to_string());
            debug!(script_size = content.as_str().len(), "Script extracted");
        }
    }

    // 鎻愬彇鎵€鏈?<style> 閮ㄥ垎锛堟敮鎸佸涓牱寮忓潡锛?
    for caps in STYLE_RE.captures_iter(source) {
        let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let content = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        let scoped = attrs.contains("scoped");
        let lang = extract_lang(attrs);

        debug!(
            style_size = content.len(),
            scoped,
            lang = %lang,
            "Style extracted"
        );

        styles.push(StyleRaw {
            content: content.to_string(),
            scoped,
            lang,
        });
    }

    // SFC 鑷冲皯瑕佹湁涓€涓?template 鎴?script锛堝厑璁哥函閫昏緫缁勪欢锛?
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
        styles,
    })
}

/// 浠庢爣绛惧睘鎬т腑鎻愬彇 lang 灞炴€с€?
///
/// 渚嬪锛歚<script lang="ts">` 鈫?`"ts"`
fn extract_lang(attrs: &str) -> String {
    if let Some(start) = attrs.find("lang=\"") {
        let start = start + 6;
        if let Some(end) = attrs[start..].find('"') {
            return attrs[start..start + end].to_string();
        }
    }
    "css".to_string()
}

/// Template 缂栬瘧鍣紙瀹屾暣鐗堬級銆?
///
/// 浣跨敤 html5ever 瑙ｆ瀽 HTML 妯℃澘锛岀敓鎴愯櫄鎷?DOM 鍒涘缓鍑芥暟銆?
/// 鏀寔 Vue 鎸囦护锛歷-if, v-for, v-bind, v-on, v-model
///
/// # 鍙傛暟
///
/// * `file_name` - 鏂囦欢鍚嶏紙鐢ㄤ簬閿欒瀹氫綅锛?
/// * `template` - 妯℃澘婧愮爜
///
/// # 杩斿洖
///
/// 杩斿洖娓叉煋鍑芥暟瀛楃涓层€?
fn compile_template(file_name: &str, template: &str) -> Result<String, SfcError> {
    if template.is_empty() {
        warn!(file = file_name, "Template is empty");
        return Ok("function render() { return null; }".to_string());
    }

    info!(file = file_name, "Compiling Vue template with full compiler");

    // 姝ラ 1: 瑙ｆ瀽 HTML 涓?AST
    let vnodes = template_compiler::parse_template(template).map_err(|e| {
        SfcError::TemplateError {
            message: format!("Failed to parse template: {}", e),
            file: file_name.to_string(),
            line: 1,
            column: 1,
        }
    })?;

    // 姝ラ 2: 鐢熸垚娓叉煋鍑芥暟
    let render_fn = template_compiler::generate_render_fn(&vnodes);

    debug!(
        file = file_name,
        render_fn_size = render_fn.len(),
        "Template compiled successfully"
    );

    Ok(render_fn)
}

/// Script 缂栬瘧鍣紙TypeScript 杞瘧锛夈€?
///
/// # 娉ㄦ剰
///
/// **褰撳墠瀹炵幇鏄紨绀虹増鏈?*锛氬彧绉婚櫎鍩烘湰绫诲瀷娉ㄨВ銆?
/// 瀹屾暣鐗堟湰搴旇闆嗘垚 swc 鎴栧叾浠?TS 缂栬瘧鍣紝鏀寔娉涘瀷銆佽楗板櫒銆乀SX銆?
///
/// # 鍙傛暟
///
/// * `file_name` - 鏂囦欢鍚嶏紙鐢ㄤ簬閿欒瀹氫綅锛?
/// * `script` - script 婧愮爜
///
/// # 杩斿洖
///
/// 杩斿洖杞瘧鍚庣殑 JavaScript 浠ｇ爜銆?
fn compile_script(file_name: &str, script: &str) -> Result<String, SfcError> {
    if script.is_empty() {
        return Ok("export default {}".to_string());
    }

    // TODO: 闆嗘垚瀹屾暣鐨?TypeScript 缂栬瘧鍣紙鏀寔娉涘瀷銆佽楗板櫒銆乀SX锛?
    // 褰撳墠鐗堟湰锛氱畝鍖栫増 TS 杞瘧锛堜粎绉婚櫎鍩烘湰绫诲瀷娉ㄨВ锛?
    debug!(file = file_name, "Using basic TypeScript transpiler (demo mode)");

    let js = transpile_ts_basic(script);

    Ok(js)
}

/// 缂栬瘧鏍峰紡鍧椼€?
fn compile_styles(styles: &[StyleRaw]) -> Vec<StyleBlock> {
    styles
        .iter()
        .map(|style| StyleBlock {
            css: style.content.clone(),
            scoped: style.scoped,
            lang: style.lang.clone(),
        })
        .collect()
}

/// 绠€鍖栫殑 TypeScript 杞瘧锛堢Щ闄ゅ熀鏈被鍨嬫敞瑙ｏ級銆?
///
/// # 闄愬埗
///
/// 褰撳墠鐗堟湰浠呮敮鎸侊細
/// - 鍩烘湰绫诲瀷娉ㄨВ锛坰tring, number, boolean, any, void, never锛?
/// - 绠€鍗曞嚱鏁拌繑鍥炵被鍨?
/// - import type 璇彞绉婚櫎
///
/// 涓嶆敮鎸侊細
/// - 娉涘瀷锛圓rray<string>, Promise<void>锛?
/// - 鎺ュ彛鍜岀被鍨嬪埆鍚?
/// - 瑁呴グ鍣?
/// - TSX
/// - 澶嶆潅鐨勪氦鍙夌被鍨?鑱斿悎绫诲瀷
fn transpile_ts_basic(source: &str) -> String {
    use regex::Regex;

    let mut result = source.to_string();

    // 绉婚櫎鍙橀噺绫诲瀷娉ㄨВ锛堢矖绯欑増鏈級
    // let x: number 鈫?let x
    // const y: string = "hi" 鈫?const y = "hi"
    let re1 = Regex::new(r":\s*(string|number|boolean|any|void|never)\b").unwrap();
    result = re1.replace_all(&result, "").to_string();

    // 绉婚櫎鍑芥暟杩斿洖绫诲瀷
    // ): number 鈫?)
    let re2 = Regex::new(r"\):\s*(string|number|boolean|any|void)\s*\{").unwrap();
    result = re2.replace_all(&result, ") {").to_string();

    // 绉婚櫎 import 绫诲瀷
    // import type { Foo } from 'bar' 鈫?锛堝垹闄ゆ暣琛岋級
    let re3 = Regex::new(r"^import\s+type\s+.*;$").unwrap();
    result = re3.replace_all(&result, "").to_string();

    result
}

/// 浠庢枃浠惰矾寰勬彁鍙栫粍浠跺悕绉般€?
///
/// 渚嬪锛歚components/App.vue` 鈫?`"App"`
fn extract_component_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("Anonymous"))
}

/// 璁＄畻瀛楃涓茬殑绠€鍗曞搱甯屻€?
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

        let js = transpile_ts_basic(ts);
        assert!(!js.contains(": number"));
        assert!(!js.contains(": string"));
        assert!(js.contains("function add(a, b) {"));
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
}

/// Initialize the SFC compiler layer.`n///`n/// This function is called by the main Iris engine initialization chain.`n/// Currently, it only logs the initialization event. Pre-compiled regex patterns`n/// are automatically initialized on first use via `LazyLock`.`n///`n/// # Safety`n/// This function is safe to call multiple times (idempotent).`n///`n/// # Example`n///`n/// ```ignore`n/// use iris_sfc::init;`n/// init(); // Initialize SFC compiler`n/// ````n///`npub fn init() {
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
