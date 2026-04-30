//! 代码修改专用的 prompt 构建模块

/// 根据文件路径检测语言类型
pub fn detect_language(file_path: &str) -> &'static str {
    let lower = file_path.to_lowercase();
    if lower.ends_with(".vue") {
        return "Vue SFC (Single File Component)";
    }
    if lower.ends_with(".css") || lower.ends_with(".scss") || lower.ends_with(".less") {
        return "CSS";
    }
    if lower.ends_with(".js") || lower.ends_with(".jsx") || lower.ends_with(".mjs") {
        return "JavaScript";
    }
    if lower.ends_with(".ts") || lower.ends_with(".tsx") || lower.ends_with(".mts") {
        return "TypeScript";
    }
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        return "HTML";
    }
    "Code"
}

/// 构建 Qwen2.5-Instruct 格式的 code-edit prompt
///
/// 格式:
/// <|im_start|>system\n{system}\n<|im_end|>\n<|im_start|>user\n{user}\n<|im_end|>\n<|im_start|>assistant\n
pub fn build_code_edit_prompt(
    file_path: &str,
    instruction: &str,
    code: &str,
) -> String {
    let lang = detect_language(file_path);

    let system = format!(
        "你是一个专业的前端代码修改助手。\
         \n输入包含：\
         \n1. 一个 {lang} 源文件\
         \n2. 一条修改指令\
         \n任务：\
         \n1. 只输出修改后的完整文件内容，不要解释\
         \n2. 保持原有代码风格和缩进\
         \n3. 不要用 ``` 包裹代码\
         \n4. 只修改与指令相关的部分",
        lang = lang
    );

    let user = format!(
        "修改指令：{instruction}\n\n源文件：{file_path}\n\n代码：\n{code}\n\n输出修改后的完整内容：",
        instruction = instruction,
        file_path = file_path,
        code = code
    );

    format!(
        "<|im_start|>system\n{}\n<|im_end|>\n<|im_start|>user\n{}\n<|im_end|>\n<|im_start|>assistant\n",
        system, user
    )
}

/// 从 AI 响应中提取纯代码（移除可能的 ``` 包裹）
pub fn extract_code_from_response(response: &str) -> &str {
    let trimmed = response.trim();
    // 移除开头的 ``` 和语言标识行
    if let Some(rest) = trimmed.strip_prefix("```") {
        let after_first_line = if let Some(pos) = rest.find('\n') {
            &rest[pos + 1..]
        } else {
            return trimmed;
        };
        // 移除结尾的 ```
        if let Some(pos) = after_first_line.rfind("```") {
            return after_first_line[..pos].trim();
        }
        return after_first_line.trim();
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("App.vue"), "Vue SFC (Single File Component)");
        assert_eq!(detect_language("style.css"), "CSS");
        assert_eq!(detect_language("main.js"), "JavaScript");
        assert_eq!(detect_language("main.ts"), "TypeScript");
        assert_eq!(detect_language("index.html"), "HTML");
    }

    #[test]
    fn test_extract_code_no_wrapper() {
        let code = "<template><div>Hello</div></template>";
        assert_eq!(extract_code_from_response(code), code);
    }

    #[test]
    fn test_extract_code_with_fence() {
        let resp = "```vue\n<template><div>Hi</div></template>\n```";
        assert_eq!(extract_code_from_response(resp), "<template><div>Hi</div></template>");
    }

    #[test]
    fn test_prompt_contains_instruction() {
        let prompt = build_code_edit_prompt("test.vue", "改色", "<div/>");
        assert!(prompt.contains("改色"));
        assert!(prompt.contains("Vue SFC"));
        assert!(prompt.contains("<|im_start|>system"));
        assert!(prompt.contains("<|im_start|>assistant"));
    }
}
