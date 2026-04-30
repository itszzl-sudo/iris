//! 调试工具系统
//!
//! 提供开发时调试能力：
//! - 组件树检查
//! - 性能分析
//! - 状态检查
//! - 渲染调试

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::error_handling::{ErrorReporter, IrisError};
#[cfg(test)]
use crate::error_handling::{ErrorSource, ErrorSeverity};

/// 组件信息
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// 组件名称
    pub name: String,
    /// 组件路径
    pub path: String,
    /// 子组件数量
    pub children_count: usize,
    /// 渲染时间
    pub render_time: Option<Duration>,
    /// 是否有错误
    pub has_error: bool,
}

impl ComponentInfo {
    /// 创建新的组件信息
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            children_count: 0,
            render_time: None,
            has_error: false,
        }
    }
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// 总渲染时间
    pub total_render_time: Duration,
    /// 布局计算时间
    pub layout_time: Duration,
    /// GPU 渲染时间
    pub gpu_time: Duration,
    /// 帧率（FPS）
    pub fps: f64,
    /// 帧时间（毫秒）
    pub frame_time_ms: f64,
    /// 内存使用（KB）
    pub memory_usage_kb: Option<u64>,
}

impl PerformanceMetrics {
    /// 创建性能指标
    pub fn new() -> Self {
        Self {
            total_render_time: Duration::ZERO,
            layout_time: Duration::ZERO,
            gpu_time: Duration::ZERO,
            fps: 0.0,
            frame_time_ms: 0.0,
            memory_usage_kb: None,
        }
    }

    /// 计算 FPS
    pub fn calculate_fps(&mut self, frame_count: u64, elapsed: Duration) {
        if elapsed.as_secs_f64() > 0.0 {
            self.fps = frame_count as f64 / elapsed.as_secs_f64();
            self.frame_time_ms = elapsed.as_secs_f64() * 1000.0 / frame_count as f64;
        }
    }

    /// 格式化输出
    pub fn format(&self) -> String {
        format!(
            "Performance Metrics:\n\
              FPS: {:.1}\n\
              Frame Time: {:.2}ms\n\
              Total Render: {:?}\n\
              Layout: {:?}\n\
              GPU: {:?}\n\
              Memory: {} KB",
            self.fps,
            self.frame_time_ms,
            self.total_render_time.as_micros() as f64 / 1000.0,
            self.layout_time,
            self.gpu_time,
            self.memory_usage_kb.map_or("N/A".to_string(), |m| m.to_string()),
        )
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// 调试工具
///
/// 提供开发时的调试和诊断能力
pub struct DevTools {
    /// 组件树
    component_tree: HashMap<String, ComponentInfo>,
    /// 性能指标
    metrics: PerformanceMetrics,
    /// 错误报告器
    error_reporter: ErrorReporter,
    /// 是否启用
    enabled: bool,
    /// 性能分析器
    profiling_enabled: bool,
    /// 帧计数器
    frame_count: u64,
    /// 帧计时器
    frame_timer: Instant,
    /// 渲染计时器
    render_timer: Option<Instant>,
    /// 布局计时器
    layout_timer: Option<Instant>,
}

impl DevTools {
    /// 创建新的调试工具
    pub fn new() -> Self {
        Self {
            component_tree: HashMap::new(),
            metrics: PerformanceMetrics::new(),
            error_reporter: ErrorReporter::new(),
            enabled: true,
            profiling_enabled: false,
            frame_count: 0,
            frame_timer: Instant::now(),
            render_timer: None,
            layout_timer: None,
        }
    }

    /// 启用/禁用调试工具
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 启用/禁用性能分析
    pub fn set_profiling(&mut self, enabled: bool) {
        self.profiling_enabled = enabled;
    }

    // ============================================
    // 组件树检查
    // ============================================

    /// 注册组件
    pub fn register_component(&mut self, name: &str, path: &str) {
        if !self.enabled {
            return;
        }

        let info = ComponentInfo::new(name, path);
        self.component_tree.insert(path.to_string(), info);
    }

    /// 更新组件信息
    pub fn update_component(&mut self, path: &str, update: impl Fn(&mut ComponentInfo)) {
        if !self.enabled {
            return;
        }

        if let Some(info) = self.component_tree.get_mut(path) {
            update(info);
        }
    }

    /// 获取组件信息
    pub fn get_component(&self, path: &str) -> Option<&ComponentInfo> {
        self.component_tree.get(path)
    }

    /// 获取所有组件
    pub fn components(&self) -> &HashMap<String, ComponentInfo> {
        &self.component_tree
    }

    /// 获取组件数量
    pub fn component_count(&self) -> usize {
        self.component_tree.len()
    }

    /// 打印组件树
    pub fn print_component_tree(&self) {
        if !self.enabled {
            return;
        }

        println!("\n=== Component Tree ===");
        for (path, info) in &self.component_tree {
            let status = if info.has_error { "❌" } else { "✓" };
            let render_time = info.render_time
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "N/A".to_string());

            println!(
                "{} {} ({}) - Children: {}, Render: {}",
                status, info.name, path, info.children_count, render_time
            );
        }
        println!("=== End Tree ===\n");
    }

    // ============================================
    // 性能分析
    // ============================================

    /// 开始帧
    pub fn begin_frame(&mut self) {
        if !self.enabled || !self.profiling_enabled {
            return;
        }

        self.frame_timer = Instant::now();
    }

    /// 结束帧
    pub fn end_frame(&mut self) {
        if !self.enabled || !self.profiling_enabled {
            return;
        }

        self.frame_count += 1;
        let elapsed = self.frame_timer.elapsed();

        // 每 60 帧更新一次 FPS
        if self.frame_count % 60 == 0 {
            self.metrics.calculate_fps(self.frame_count, elapsed);
            self.frame_count = 0;
            self.frame_timer = Instant::now();
        }
    }

    /// 开始渲染计时
    pub fn begin_render(&mut self) {
        if !self.enabled || !self.profiling_enabled {
            return;
        }

        self.render_timer = Some(Instant::now());
    }

    /// 结束渲染计时
    pub fn end_render(&mut self) {
        if !self.enabled || !self.profiling_enabled {
            return;
        }

        if let Some(timer) = self.render_timer.take() {
            let duration = timer.elapsed();
            self.metrics.total_render_time = duration;
        }
    }

    /// 开始布局计时
    pub fn begin_layout(&mut self) {
        if !self.enabled || !self.profiling_enabled {
            return;
        }

        self.layout_timer = Some(Instant::now());
    }

    /// 结束布局计时
    pub fn end_layout(&mut self) {
        if !self.enabled || !self.profiling_enabled {
            return;
        }

        if let Some(timer) = self.layout_timer.take() {
            let duration = timer.elapsed();
            self.metrics.layout_time = duration;
        }
    }

    /// 设置 GPU 渲染时间
    pub fn set_gpu_time(&mut self, duration: Duration) {
        if !self.enabled || !self.profiling_enabled {
            return;
        }

        self.metrics.gpu_time = duration;
    }

    /// 获取性能指标
    pub fn metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// 打印性能指标
    pub fn print_metrics(&self) {
        if !self.enabled {
            return;
        }

        println!("\n{}", self.metrics.format());
    }

    // ============================================
    // 错误调试
    // ============================================

    /// 报告错误
    pub fn report_error(&mut self, error: IrisError) {
        if !self.enabled {
            return;
        }

        self.error_reporter.report(error);
    }

    /// 获取错误报告器
    pub fn error_reporter(&self) -> &ErrorReporter {
        &self.error_reporter
    }

    /// 获取错误报告器（可变）
    pub fn error_reporter_mut(&mut self) -> &mut ErrorReporter {
        &mut self.error_reporter
    }

    /// 打印错误报告
    pub fn print_error_report(&self) {
        if !self.enabled {
            return;
        }

        println!("\n{}", self.error_reporter.generate_report());
    }

    // ============================================
    // 综合调试
    // ============================================

    /// 打印完整的调试信息
    pub fn print_debug_info(&self) {
        if !self.enabled {
            return;
        }

        println!("\n========== Iris DevTools ==========");
        self.print_component_tree();
        self.print_metrics();
        self.print_error_report();
        println!("===================================\n");
    }

    /// 重置所有状态
    pub fn reset(&mut self) {
        self.component_tree.clear();
        self.metrics = PerformanceMetrics::new();
        self.error_reporter.clear();
        self.frame_count = 0;
        self.frame_timer = Instant::now();
    }
}

impl Default for DevTools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devtools_creation() {
        let devtools = DevTools::new();
        assert!(devtools.enabled);
        assert!(!devtools.profiling_enabled);
        assert_eq!(devtools.component_count(), 0);
    }

    #[test]
    fn test_component_registration() {
        let mut devtools = DevTools::new();

        devtools.register_component("App", "App");
        devtools.register_component("Header", "App/Header");

        assert_eq!(devtools.component_count(), 2);
        assert!(devtools.get_component("App").is_some());
        assert!(devtools.get_component("App/Header").is_some());
    }

    #[test]
    fn test_component_update() {
        let mut devtools = DevTools::new();
        devtools.register_component("App", "App");

        devtools.update_component("App", |info| {
            info.children_count = 5;
            info.render_time = Some(Duration::from_millis(10));
        });

        let component = devtools.get_component("App").unwrap();
        assert_eq!(component.children_count, 5);
        assert!(component.render_time.is_some());
    }

    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();
        metrics.calculate_fps(60, Duration::from_secs(1));

        assert!((metrics.fps - 60.0).abs() < 0.1);
        assert!((metrics.frame_time_ms - 16.67).abs() < 0.1);
    }

    #[test]
    fn test_profiling_timers() {
        let mut devtools = DevTools::new();
        devtools.set_profiling(true);

        devtools.begin_render();
        std::thread::sleep(Duration::from_millis(5));
        devtools.end_render();

        assert!(devtools.metrics().total_render_time.as_millis() >= 5);
    }

    #[test]
    fn test_error_reporting() {
        let mut devtools = DevTools::new();

        let error = IrisError::new("Test error", ErrorSource::Render, ErrorSeverity::Error);
        devtools.report_error(error);

        assert_eq!(devtools.error_reporter().error_count(), 1);
    }

    #[test]
    fn test_devtools_disable() {
        let mut devtools = DevTools::new();
        devtools.set_enabled(false);

        devtools.register_component("App", "App");
        assert_eq!(devtools.component_count(), 0);
    }

    #[test]
    fn test_devtools_reset() {
        let mut devtools = DevTools::new();

        devtools.register_component("App", "App");
        devtools.report_error(IrisError::new("Error", ErrorSource::Render, ErrorSeverity::Error));

        assert_eq!(devtools.component_count(), 1);
        assert_eq!(devtools.error_reporter().error_count(), 1);

        devtools.reset();

        assert_eq!(devtools.component_count(), 0);
        assert_eq!(devtools.error_reporter().error_count(), 0);
    }
}
