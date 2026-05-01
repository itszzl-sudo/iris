//! 代码审查模块
//!
//! 使用本地 LLM 对代码进行静态审查，发现潜在问题：
//! - Bug 风险
//! - 安全隐患
//! - 性能问题
//! - 代码风格
//! - 最佳实践

use crate::prompt::detect_language;

/// 问题严重级别
#[derive(Debug, Clone, PartialEq)]
pub enum ReviewSeverity {
    /// 严重问题（可能导致运行时错误）
    Critical,
    /// 潜在问题（可能引发异常或逻辑错误）
    Warning,
    /// 信息提示（优化建议）
    Info,
    /// 风格建议（代码规范相关）
    Suggestion,
}

impl ReviewSeverity {
    /// 严重级别标签（用于显示）
    pub fn label(&self) -> &'static str {
        match self {
            Self::Critical => "🔴 严重",
            Self::Warning => "🟡 警告",
            Self::Info => "🔵 提示",
            Self::Suggestion => "🟢 建议",
        }
    }

    /// 排序权重（严重最高）
    pub fn sort_weight(&self) -> u8 {
        match self {
            Self::Critical => 0,
            Self::Warning => 1,
            Self::Info => 2,
            Self::Suggestion => 3,
        }
    }
}

/// 审查问题类型
#[derive(Debug, Clone, PartialEq)]
pub enum IssueType {
    /// Bug 风险
    Bug,
    /// 安全问题
    Security,
    /// 性能问题
    Performance,
    /// 代码风格
    Style,
    /// 最佳实践
    BestPractice,
    /// 可维护性
    Maintainability,
    /// 其他
    Other(String),
}

impl IssueType {
    /// 类型标签
    pub fn label(&self) -> &str {
        match self {
            Self::Bug => "Bug",
            Self::Security => "安全",
            Self::Performance => "性能",
            Self::Style => "风格",
            Self::BestPractice => "最佳实践",
            Self::Maintainability => "可维护性",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// 审查中的单个问题项
#[derive(Debug, Clone)]
pub struct ReviewItem {
    /// 严重级别
    pub severity: ReviewSeverity,
    /// 问题类型
    pub issue_type: IssueType,
    /// 行号范围（可选）
    pub line_range: Option<(usize, usize)>,
    /// 问题描述
    pub description: String,
    /// 建议修复方案
    pub suggestion: String,
}

/// 代码审查结果
#[derive(Debug, Clone)]
pub struct ReviewReport {
    /// 被审查的文件路径
    pub file_path: String,
    /// 审查概要
    pub summary: String,
    /// 审查发现的问题列表
    pub issues: Vec<ReviewItem>,
    /// 原始 AI 响应
    pub raw_response: String,
    /// 审查耗时（秒）
    pub elapsed_secs: f64,
}

impl ReviewReport {
    /// 问题的严重级别计数
    pub fn severity_counts(&self) -> [usize; 4] {
        let mut counts = [0usize; 4];
        for issue in &self.issues {
            match issue.severity {
                ReviewSeverity::Critical => counts[0] += 1,
                ReviewSeverity::Warning => counts[1] += 1,
                ReviewSeverity::Info => counts[2] += 1,
                ReviewSeverity::Suggestion => counts[3] += 1,
            }
        }
        counts
    }

    /// 按严重级别排序的问题列表（严重在前）
    pub fn sorted_issues(&self) -> Vec<&ReviewItem> {
        let mut items: Vec<&ReviewItem> = self.issues.iter().collect();
        items.sort_by_key(|i| i.severity.sort_weight());
        items
    }
}

/// 构建代码审查 prompt
///
/// 要求 AI 以结构化方式审查代码，输出包含：
/// - 总体评价
/// - 问题列表（含严重级别、行号、描述和建议）
pub fn build_code_review_prompt(file_path: &str, code: &str) -> String {
    let lang = detect_language(file_path);
    let line_count = code.lines().count();

    let system = format!(
        "你是一个专业的前端代码审查专家。\
         \n你将被给定一个 {lang} 源文件，请进行全面代码审查。\
         \n\
         \n审查维度：\
         \n1. Bug 风险：逻辑错误、边界条件、类型安全\
         \n2. 安全问题：XSS、注入、敏感信息泄露\
         \n3. 性能问题：不必要的计算、内存泄漏、渲染优化\
         \n4. 代码风格：命名规范、缩进、代码组织\
         \n5. 最佳实践：Vue/React/通用前端最佳实践\
         \n6. 可维护性：函数长度、复杂度、注释、可测试性\
         \n\
         \n输出格式要求：\
         \n第一行输出总体评价（一句话概括代码质量）\
         \n然后对每个问题按以下格式输出：\
         \n\
         \n## [严重级别] 问题类型：简短标题\
         \n- 位置：第 X 行（或第 X-Y 行）\
         \n- 描述：详细描述问题\
         \n- 建议：修复建议\
         \n\
         \n严重级别用 [严重]/[警告]/[提示]/[建议] 表示。\
         \n如果代码没有问题，只输出「✅ 代码质量良好，未发现明显问题。」",
        lang = lang,
    );

    let user = format!(
        "请审查以下 {lang} 文件：\
         \n文件：{file_path}\
         \n行数：{line_count}\
         \n\
         \n代码：\
         \n```{lang_ext}\n{code}\n```\
         \n\
         \n请进行全面代码审查：",
        lang = lang,
        file_path = file_path,
        line_count = line_count,
        lang_ext = file_path.rsplit('.').next().unwrap_or("txt"),
        code = code,
    );

    format!(
        "<|im_start|>system\n{}\n<|im_end|>\n<|im_start|>user\n{}\n<|im_end|>\n<|im_start|>assistant\n",
        system, user
    )
}

/// 从 AI 响应中解析审查结果
pub fn parse_review_response(file_path: &str, response: &str) -> ReviewReport {
    let trimmed = response.trim();
    let lines: Vec<&str> = trimmed.lines().collect();

    // 提取概要：第一行
    let summary = lines.first()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let mut issues = Vec::new();

    // 解析问题条目：匹配 "## [严重级别] 问题类型：标题" 格式
    let mut current_item: Option<(ReviewSeverity, IssueType, String)> = None;
    let mut current_desc = String::new();
    let mut current_suggestion = String::new();
    let mut current_line_range: Option<(usize, usize)> = None;
    let mut in_description = false;
    let mut in_suggestion = false;

    for line in &lines[1..] {
        let trimmed_line = line.trim();

        // 检查是否为新的问题条目标题
        if let Some(captured) = parse_item_header(trimmed_line) {
            // 保存前一个条目
            if let Some((sev, itype, _title)) = current_item.take() {
                issues.push(ReviewItem {
                    severity: sev,
                    issue_type: itype,
                    line_range: current_line_range,
                    description: current_desc.trim().to_string(),
                    suggestion: current_suggestion.trim().to_string(),
                });
            }
            current_item = Some(captured);
            current_desc.clear();
            current_suggestion.clear();
            current_line_range = None;
            in_description = false;
            in_suggestion = false;
            continue;
        }

        if current_item.is_none() {
            continue;
        }

        // 解析位置行
        if trimmed_line.starts_with("- 位置：") || trimmed_line.starts_with("- 位置:") {
            let pos_str = trimmed_line.trim_start_matches("- 位置：")
                .trim_start_matches("- 位置:");
            current_line_range = parse_line_range(pos_str);
            continue;
        }

        // 解析描述行
        if trimmed_line.starts_with("- 描述：") || trimmed_line.starts_with("- 描述:") {
            in_description = true;
            in_suggestion = false;
            let desc = trimmed_line.trim_start_matches("- 描述：")
                .trim_start_matches("- 描述:");
            current_desc.push_str(desc);
            continue;
        }

        // 解析建议行
        if trimmed_line.starts_with("- 建议：") || trimmed_line.starts_with("- 建议:") {
            in_description = false;
            in_suggestion = true;
            let sug = trimmed_line.trim_start_matches("- 建议：")
                .trim_start_matches("- 建议:");
            current_suggestion.push_str(sug);
            continue;
        }

        // 多行描述/建议续行
        if in_description {
            current_desc.push('\n');
            current_desc.push_str(trimmed_line);
        }
        if in_suggestion {
            current_suggestion.push('\n');
            current_suggestion.push_str(trimmed_line);
        }
    }

    // 保存最后一个条目
    if let Some((sev, itype, title)) = current_item {
        if !current_desc.is_empty() || !current_suggestion.is_empty() {
            let desc = if current_desc.trim().is_empty() {
                title.clone()
            } else {
                current_desc.trim().to_string()
            };
            issues.push(ReviewItem {
                severity: sev,
                issue_type: itype,
                line_range: current_line_range,
                description: desc,
                suggestion: current_suggestion.trim().to_string(),
            });
        }
    }

    ReviewReport {
        file_path: file_path.to_string(),
        summary,
        issues,
        raw_response: response.to_string(),
        elapsed_secs: 0.0,
    }
}

/// 解析问题条目标题行
/// 格式: "## [严重级别] 问题类型：标题"
fn parse_item_header(line: &str) -> Option<(ReviewSeverity, IssueType, String)> {
    let line = line.trim();

    // 必须以 "##" 开头
    if !line.starts_with("##") {
        return None;
    }

    // 提取方括号中的内容
    let severity = if line.contains("[严重]") {
        Some(ReviewSeverity::Critical)
    } else if line.contains("[警告]") {
        Some(ReviewSeverity::Warning)
    } else if line.contains("[提示]") {
        Some(ReviewSeverity::Info)
    } else if line.contains("[建议]") {
        Some(ReviewSeverity::Suggestion)
    } else {
        None
    };

    let severity = severity?;

    // 提取问题类型
    let after_bracket = if let Some(pos) = line.rfind(']') {
        &line[pos + 1..]
    } else {
        return None;
    };

    let after_bracket = after_bracket.trim();

    // 分割 "问题类型" 和 "标题"
    let (issue_type_str, title) = if let Some(pos) = after_bracket.find("：") {
        let it = after_bracket[..pos].trim();
        let t = after_bracket[pos + 3..].trim(); // 跳过 "：" 和可能的空格
        (it, t)
    } else if let Some(pos) = after_bracket.find(":") {
        let it = after_bracket[..pos].trim();
        let t = after_bracket[pos + 1..].trim();
        (it, t)
    } else {
        // 没有冒号分隔，整个作为标题
        ("", after_bracket)
    };

    let issue_type = match issue_type_str {
        "Bug" | "bug" => IssueType::Bug,
        "安全" | "Security" | "security" => IssueType::Security,
        "性能" | "Performance" | "performance" => IssueType::Performance,
        "风格" | "Style" | "style" => IssueType::Style,
        "最佳实践" | "BestPractice" | "Best Practice" => IssueType::BestPractice,
        "可维护性" | "Maintainability" | "maintainability" => IssueType::Maintainability,
        other => IssueType::Other(other.to_string()),
    };

    Some((severity, issue_type, title.to_string()))
}

/// 解析行号范围
/// 支持格式: "第 5 行", "第 5-10 行", "第 5 行到第 10 行"
fn parse_line_range(s: &str) -> Option<(usize, usize)> {
    let s = s.trim();

    // 匹配 "第 X 行到第 Y 行" 或 "第 X-Y 行" 或 "第 X 行"
    let digits: Vec<usize> = s.split(|c: char| !c.is_ascii_digit())
        .filter_map(|n| n.parse().ok())
        .collect();

    match digits.len() {
        0 => None,
        1 => Some((digits[0], digits[0])),
        _ => Some((digits[0], digits[1])),
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_review_prompt() {
        let prompt = build_code_review_prompt("test.vue", "<template><div/></template>");
        assert!(prompt.contains("Vue SFC"));
        assert!(prompt.contains("审查维度"));
        assert!(prompt.contains("<|im_start|>system"));
        assert!(prompt.contains("<|im_start|>assistant"));
    }

    #[test]
    fn test_parse_item_header_critical() {
        let result = parse_item_header("## [严重] Bug：空指针风险");
        assert!(result.is_some());
        let (sev, itype, _title) = result.unwrap();
        assert_eq!(sev, ReviewSeverity::Critical);
        assert_eq!(itype, IssueType::Bug);
        assert_eq!(_title, "空指针风险");
    }

    #[test]
    fn test_parse_item_header_warning() {
        let result = parse_item_header("## [警告] 性能：不必要的重渲染");
        assert!(result.is_some());
        let (sev, itype, _title) = result.unwrap();
        assert_eq!(sev, ReviewSeverity::Warning);
        assert_eq!(itype, IssueType::Performance);
        assert_eq!(_title, "不必要的重渲染");
    }

    #[test]
    fn test_parse_item_header_info() {
        let result = parse_item_header("## [提示] Style: Long line");
        assert!(result.is_some());
        let (sev, itype, _title) = result.unwrap();
        assert_eq!(sev, ReviewSeverity::Info);
        assert_eq!(itype, IssueType::Style);
    }

    #[test]
    fn test_parse_item_header_suggestion() {
        let result = parse_item_header("## [建议] 最佳实践：使用计算属性替代方法");
        assert!(result.is_some());
        let (sev, itype, _title) = result.unwrap();
        assert_eq!(sev, ReviewSeverity::Suggestion);
        assert_eq!(itype, IssueType::BestPractice);
    }

    #[test]
    fn test_parse_item_header_invalid() {
        assert!(parse_item_header("普通文本行").is_none());
        assert!(parse_item_header("# 单井号").is_none());
        assert!(parse_item_header("### 三级标题").is_none());
    }

    #[test]
    fn test_parse_line_range_single() {
        let range = parse_line_range("第 5 行");
        assert_eq!(range, Some((5, 5)));
    }

    #[test]
    fn test_parse_line_range_range() {
        let range = parse_line_range("第 10-20 行");
        assert_eq!(range, Some((10, 20)));
    }

    #[test]
    fn test_parse_line_range_chinese_range() {
        let range = parse_line_range("第 5 行到第 10 行");
        assert_eq!(range, Some((5, 10)));
    }

    #[test]
    fn test_parse_review_response_full() {
        let response = "代码质量中等，存在一些可优化的地方。\n\
            \n\
            ## [警告] Bug：可能的空值引用\n\
            - 位置：第 15 行\n\
            - 描述：变量 `user.name` 可能在 `user` 为 null 时崩溃\n\
            - 建议：添加可选链操作符 `user?.name`\n\
            \n\
            ## [建议] 最佳实践：使用 const 替代 let\n\
            - 位置：第 3 行\n\
            - 描述：变量 `config` 从未被重新赋值\n\
            - 建议：使用 `const config = ...` 声明";

        let report = parse_review_response("test.js", response);
        assert_eq!(report.file_path, "test.js");
        assert!(report.summary.contains("代码质量中等"));
        assert_eq!(report.issues.len(), 2);

        // 验证第一个问题
        assert_eq!(report.issues[0].severity, ReviewSeverity::Warning);
        assert_eq!(report.issues[0].issue_type, IssueType::Bug);
        assert_eq!(report.issues[0].line_range, Some((15, 15)));
        assert!(report.issues[0].description.contains("null 时崩溃"));

        // 验证按严重级别排序
        let sorted = report.sorted_issues();
        assert_eq!(sorted[0].severity, ReviewSeverity::Warning);
        assert_eq!(sorted[1].severity, ReviewSeverity::Suggestion);
    }

    #[test]
    fn test_parse_review_response_clean_code() {
        let response = "✅ 代码质量良好，未发现明显问题。";
        let report = parse_review_response("test.js", response);
        assert!(report.issues.is_empty());
        assert!(report.summary.contains("代码质量良好"));
    }

    #[test]
    fn test_review_severity_label() {
        assert_eq!(ReviewSeverity::Critical.label(), "🔴 严重");
        assert_eq!(ReviewSeverity::Warning.label(), "🟡 警告");
        assert_eq!(ReviewSeverity::Info.label(), "🔵 提示");
        assert_eq!(ReviewSeverity::Suggestion.label(), "🟢 建议");
    }

    #[test]
    fn test_severity_counts() {
        let report = ReviewReport {
            file_path: "test.js".into(),
            summary: "Test".into(),
            issues: vec![
                ReviewItem {
                    severity: ReviewSeverity::Critical,
                    issue_type: IssueType::Bug,
                    line_range: None,
                    description: "d1".into(),
                    suggestion: "s1".into(),
                },
                ReviewItem {
                    severity: ReviewSeverity::Warning,
                    issue_type: IssueType::Performance,
                    line_range: None,
                    description: "d2".into(),
                    suggestion: "s2".into(),
                },
                ReviewItem {
                    severity: ReviewSeverity::Info,
                    issue_type: IssueType::Style,
                    line_range: None,
                    description: "d3".into(),
                    suggestion: "s3".into(),
                },
                ReviewItem {
                    severity: ReviewSeverity::Suggestion,
                    issue_type: IssueType::BestPractice,
                    line_range: None,
                    description: "d4".into(),
                    suggestion: "s4".into(),
                },
            ],
            raw_response: String::new(),
            elapsed_secs: 0.0,
        };
        let counts = report.severity_counts();
        assert_eq!(counts, [1, 1, 1, 1]);
    }
}
