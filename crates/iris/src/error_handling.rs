//! 错误边界系统
//!
//! 提供组件级别的错误隔离和恢复机制：
//! - ErrorBoundary: 捕获子组件渲染错误
//! - 错误恢复策略
//! - 错误信息聚合
//! - 堆栈追踪支持

use std::fmt;
use std::error::Error;

/// 错误严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// 警告 - 不影响渲染
    Warning,
    /// 错误 - 组件渲染失败
    Error,
    /// 致命 - 整个应用崩溃
    Fatal,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Warning => write!(f, "WARNING"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// 错误来源
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSource {
    /// 渲染错误
    Render,
    /// 布局错误
    Layout,
    /// 样式错误
    Style,
    /// JavaScript 错误
    Script,
    /// 网络错误
    Network,
    /// 未知错误
    Unknown(String),
}

impl fmt::Display for ErrorSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSource::Render => write!(f, "Render"),
            ErrorSource::Layout => write!(f, "Layout"),
            ErrorSource::Style => write!(f, "Style"),
            ErrorSource::Script => write!(f, "Script"),
            ErrorSource::Network => write!(f, "Network"),
            ErrorSource::Unknown(msg) => write!(f, "Unknown({})", msg),
        }
    }
}

/// Iris 错误类型
#[derive(Debug)]
pub struct IrisError {
    /// 错误消息
    pub message: String,
    /// 错误来源
    pub source: ErrorSource,
    /// 严重级别
    pub severity: ErrorSeverity,
    /// 组件路径
    pub component_path: Option<String>,
    /// 原始错误
    pub source_error: Option<Box<dyn Error + Send + Sync>>,
    /// 时间戳
    pub timestamp: std::time::SystemTime,
}

impl IrisError {
    /// 创建新的错误
    pub fn new(message: &str, source: ErrorSource, severity: ErrorSeverity) -> Self {
        Self {
            message: message.to_string(),
            source,
            severity,
            component_path: None,
            source_error: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// 设置组件路径
    pub fn with_component(mut self, path: &str) -> Self {
        self.component_path = Some(path.to_string());
        self
    }

    /// 设置原始错误
    pub fn with_source_error(mut self, error: Box<dyn Error + Send + Sync>) -> Self {
        self.source_error = Some(error);
        self
    }

    /// 格式化错误信息
    pub fn format(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("[{}] ", self.severity));
        output.push_str(&format!("[{}] ", self.source));

        if let Some(ref path) = self.component_path {
            output.push_str(&format!("[{}] ", path));
        }

        output.push_str(&self.message);

        if let Some(ref source_err) = self.source_error {
            output.push_str(&format!("\nCaused by: {}", source_err));
        }

        output
    }
}

impl fmt::Display for IrisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

impl Error for IrisError {}

/// 错误边界状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorBoundaryState {
    /// 正常状态
    Normal,
    /// 捕获到错误
    Errored(IrisError),
    /// 已恢复
    Recovered,
}

/// 错误边界
///
/// 用于隔离组件错误，防止错误传播到整个应用
pub struct ErrorBoundary {
    /// 边界名称
    pub name: String,
    /// 当前状态
    state: ErrorBoundaryState,
    /// 错误历史
    error_history: Vec<IrisError>,
    /// 最大错误历史长度
    max_history: usize,
    /// 是否继续渲染子组件
    continue_rendering: bool,
    /// 备用内容（当错误发生时显示）
    fallback: Option<String>,
}

impl ErrorBoundary {
    /// 创建新的错误边界
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: ErrorBoundaryState::Normal,
            error_history: Vec::new(),
            max_history: 100,
            continue_rendering: true,
            fallback: None,
        }
    }

    /// 设置备用内容
    pub fn with_fallback(mut self, fallback: &str) -> Self {
        self.fallback = Some(fallback.to_string());
        self
    }

    /// 设置最大错误历史长度
    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// 捕获错误
    pub fn catch_error(&mut self, error: IrisError) {
        // 添加到错误历史
        self.error_history.push(error.clone());

        // 限制历史长度
        if self.error_history.len() > self.max_history {
            self.error_history.remove(0);
        }

        // 更新状态
        self.state = ErrorBoundaryState::Errored(error);

        // 根据严重级别决定是否继续渲染
        match error.severity {
            ErrorSeverity::Warning => {
                self.continue_rendering = true;
            }
            ErrorSeverity::Error => {
                self.continue_rendering = false;
            }
            ErrorSeverity::Fatal => {
                self.continue_rendering = false;
            }
        }
    }

    /// 尝试恢复
    pub fn recover(&mut self) -> Result<(), &IrisError> {
        if let ErrorBoundaryState::Errored(ref error) = self.state {
            if error.severity == ErrorSeverity::Warning {
                self.state = ErrorBoundaryState::Recovered;
                self.continue_rendering = true;
                Ok(())
            } else {
                Err(error)
            }
        } else {
            Ok(())
        }
    }

    /// 重置错误边界
    pub fn reset(&mut self) {
        self.state = ErrorBoundaryState::Normal;
        self.continue_rendering = true;
    }

    /// 获取当前状态
    pub fn state(&self) -> &ErrorBoundaryState {
        &self.state
    }

    /// 是否应该继续渲染
    pub fn should_continue_rendering(&self) -> bool {
        self.continue_rendering
    }

    /// 获取备用内容
    pub fn fallback_content(&self) -> Option<&str> {
        if matches!(self.state, ErrorBoundaryState::Errored(_)) {
            self.fallback.as_deref()
        } else {
            None
        }
    }

    /// 获取错误历史
    pub fn error_history(&self) -> &[IrisError] {
        &self.error_history
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.error_history.len()
    }

    /// 清除错误历史
    pub fn clear_history(&mut self) {
        self.error_history.clear();
    }

    /// 获取最近的错误
    pub fn latest_error(&self) -> Option<&IrisError> {
        self.error_history.last()
    }
}

/// 错误报告器
///
/// 收集和报告错误信息
pub struct ErrorReporter {
    /// 所有错误
    errors: Vec<IrisError>,
    /// 最大错误数
    max_errors: usize,
    /// 是否启用报告
    enabled: bool,
}

impl ErrorReporter {
    /// 创建新的错误报告器
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            max_errors: 1000,
            enabled: true,
        }
    }

    /// 报告错误
    pub fn report(&mut self, error: IrisError) {
        if !self.enabled {
            return;
        }

        // 打印错误
        println!("{}", error.format());

        // 存储错误
        self.errors.push(error);

        // 限制错误数量
        if self.errors.len() > self.max_errors {
            self.errors.remove(0);
        }
    }

    /// 获取所有错误
    pub fn errors(&self) -> &[IrisError] {
        &self.errors
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 按严重级别过滤错误
    pub fn errors_by_severity(&self, severity: ErrorSeverity) -> Vec<&IrisError> {
        self.errors.iter().filter(|e| e.severity == severity).collect()
    }

    /// 按来源过滤错误
    pub fn errors_by_source(&self, source: &ErrorSource) -> Vec<&IrisError> {
        self.errors.iter().filter(|e| e.source == *source).collect()
    }

    /// 清除所有错误
    pub fn clear(&mut self) {
        self.errors.clear();
    }

    /// 启用/禁用报告
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 生成错误报告
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Error Report ===\n\n");

        report.push_str(&format!("Total Errors: {}\n", self.errors.len()));

        // 按严重级别统计
        let warnings = self.errors_by_severity(ErrorSeverity::Warning).len();
        let errors = self.errors_by_severity(ErrorSeverity::Error).len();
        let fatals = self.errors_by_severity(ErrorSeverity::Fatal).len();

        report.push_str(&format!("  Warnings: {}\n", warnings));
        report.push_str(&format!("  Errors:   {}\n", errors));
        report.push_str(&format!("  Fatals:   {}\n\n", fatals));

        // 按来源统计
        report.push_str("Errors by Source:\n");
        let sources = [&ErrorSource::Render, &ErrorSource::Layout, &ErrorSource::Style, &ErrorSource::Script];
        for source in sources {
            let count = self.errors_by_source(source).len();
            if count > 0 {
                report.push_str(&format!("  {}: {}\n", source, count));
            }
        }

        report.push_str("\n=== End Report ===\n");
        report
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = IrisError::new("Test error", ErrorSource::Render, ErrorSeverity::Error);
        assert_eq!(error.message, "Test error");
        assert_eq!(error.source, ErrorSource::Render);
        assert_eq!(error.severity, ErrorSeverity::Error);
    }

    #[test]
    fn test_error_with_component() {
        let error = IrisError::new("Test error", ErrorSource::Render, ErrorSeverity::Error)
            .with_component("App/Header");
        assert_eq!(error.component_path, Some("App/Header".to_string()));
    }

    #[test]
    fn test_error_boundary_capture() {
        let mut boundary = ErrorBoundary::new("TestBoundary");
        assert!(boundary.should_continue_rendering());

        let error = IrisError::new("Render failed", ErrorSource::Render, ErrorSeverity::Error);
        boundary.catch_error(error);

        assert!(!boundary.should_continue_rendering());
        assert_eq!(boundary.error_count(), 1);
    }

    #[test]
    fn test_error_boundary_warning() {
        let mut boundary = ErrorBoundary::new("TestBoundary");

        let warning = IrisError::new("Minor issue", ErrorSource::Style, ErrorSeverity::Warning);
        boundary.catch_error(warning);

        // 警告不应阻止渲染
        assert!(boundary.should_continue_rendering());
        assert_eq!(boundary.error_count(), 1);
    }

    #[test]
    fn test_error_boundary_recovery() {
        let mut boundary = ErrorBoundary::new("TestBoundary");

        let warning = IrisError::new("Minor issue", ErrorSource::Style, ErrorSeverity::Warning);
        boundary.catch_error(warning);

        // 可以从警告中恢复
        assert!(boundary.recover().is_ok());
        assert_eq!(boundary.state(), &ErrorBoundaryState::Recovered);
    }

    #[test]
    fn test_error_boundary_reset() {
        let mut boundary = ErrorBoundary::new("TestBoundary");

        let error = IrisError::new("Render failed", ErrorSource::Render, ErrorSeverity::Error);
        boundary.catch_error(error);

        boundary.reset();
        assert_eq!(boundary.state(), &ErrorBoundaryState::Normal);
        assert!(boundary.should_continue_rendering());
    }

    #[test]
    fn test_error_boundary_fallback() {
        let mut boundary = ErrorBoundary::new("TestBoundary")
            .with_fallback("<div>Error occurred</div>");

        let error = IrisError::new("Render failed", ErrorSource::Render, ErrorSeverity::Error);
        boundary.catch_error(error);

        assert_eq!(
            boundary.fallback_content(),
            Some("<div>Error occurred</div>")
        );
    }

    #[test]
    fn test_error_boundary_history() {
        let mut boundary = ErrorBoundary::new("TestBoundary").with_max_history(3);

        for i in 0..5 {
            let error = IrisError::new(
                &format!("Error {}", i),
                ErrorSource::Render,
                ErrorSeverity::Error,
            );
            boundary.catch_error(error);
        }

        // 应该只保留最近的 3 个错误
        assert_eq!(boundary.error_count(), 3);
        assert_eq!(boundary.latest_error().unwrap().message, "Error 4");
    }

    #[test]
    fn test_error_reporter() {
        let mut reporter = ErrorReporter::new();

        let error1 = IrisError::new("Error 1", ErrorSource::Render, ErrorSeverity::Error);
        let error2 = IrisError::new("Warning 1", ErrorSource::Style, ErrorSeverity::Warning);

        reporter.report(error1);
        reporter.report(error2);

        assert_eq!(reporter.error_count(), 2);
        assert_eq!(reporter.errors_by_severity(ErrorSeverity::Error).len(), 1);
        assert_eq!(reporter.errors_by_severity(ErrorSeverity::Warning).len(), 1);
    }

    #[test]
    fn test_error_report_generation() {
        let mut reporter = ErrorReporter::new();

        reporter.report(IrisError::new("Error 1", ErrorSource::Render, ErrorSeverity::Error));
        reporter.report(IrisError::new("Warning 1", ErrorSource::Style, ErrorSeverity::Warning));
        reporter.report(IrisError::new("Error 2", ErrorSource::Render, ErrorSeverity::Error));

        let report = reporter.generate_report();
        assert!(report.contains("Total Errors: 3"));
        assert!(report.contains("Warnings: 1"));
        assert!(report.contains("Errors:   2"));
    }
}
