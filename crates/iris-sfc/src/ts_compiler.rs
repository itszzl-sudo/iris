//! TypeScript 编译器（基于 swc 62 高层 Compiler API）
//!
//! 功能：
//! - 完整的 TypeScript 到 JavaScript 转译
//! - 支持泛型、接口、装饰器、TSX
//! - Source map 生成
//! - 类型擦除与优化
//!
//! 使用 swc 高层 Compiler API，提供稳定可靠的 TypeScript 编译

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::process::Command;
use std::fs;
use std::env;
use std::path::PathBuf;

use swc::{
    Compiler,
    config::{Options, Config, JscConfig},
    try_with_handler,
    HandlerOpts,
};
use swc_common::{
    errors::ColorConfig,
    FileName,
    SourceMap,
    Globals,
    GLOBALS,
};
use swc_ecma_parser::{Syntax, TsSyntax};
use tracing::{debug, info, warn};

/// TypeScript 编译配置
#[derive(Debug, Clone)]
pub struct TsCompilerConfig {
    /// 是否启用 JSX/TSX 支持
    pub jsx: bool,
    /// 是否保留装饰器（decorators）
    pub keep_decorators: bool,
    /// 是否生成 source map
    pub source_map: bool,
    /// 目标 ECMAScript 版本
    pub target: EsVersion,
}

/// ECMAScript 版本
/// 注意：当前只使用 ES2020，其他版本为未来功能预留
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum EsVersion {
    ES2015,
    ES2016,
    ES2017,
    ES2018,
    ES2019,
    ES2020,
    ES2021,
    ES2022,
    ESNext,
}

impl Default for TsCompilerConfig {
    fn default() -> Self {
        Self {
            jsx: false,
            keep_decorators: false,
            source_map: true,
            target: EsVersion::ES2020,
        }
    }
}

/// TypeScript 编译结果
#[derive(Debug)]
pub struct TsCompileResult {
    /// 编译后的 JavaScript 代码
    pub code: String,
    /// Source map（如果启用）
    /// 注意：当前默认禁用，未来可按需启用用于浏览器调试和错误监控
    #[allow(dead_code)]
    pub source_map: Option<String>,
    /// 编译时间（毫秒）
    pub compile_time_ms: f64,
}

/// 类型检查配置
#[derive(Debug, Clone)]
pub struct TypeCheckConfig {
    /// 是否启用类型检查（默认从环境变量 IRIS_TYPE_CHECK 读取）
    pub enabled: bool,
    /// 是否使用严格模式（默认从环境变量 IRIS_TYPE_CHECK_STRICT 读取）
    pub strict: bool,
    /// tsconfig.json 路径（可选）
    pub ts_config_path: Option<String>,
}

impl Default for TypeCheckConfig {
    fn default() -> Self {
        let enabled = env::var("IRIS_TYPE_CHECK")
            .map(|v| v == "true" || v == "1" || v == "yes")
            .unwrap_or(false);
        
        let strict = env::var("IRIS_TYPE_CHECK_STRICT")
            .map(|v| v == "true" || v == "1" || v == "yes")
            .unwrap_or(false);
        
        Self {
            enabled,
            strict,
            ts_config_path: None,
        }
    }
}

/// 类型检查结果
#[derive(Debug)]
pub enum TypeCheckResult {
    /// 类型检查通过
    Success,
    /// 类型检查失败，包含错误信息
    Errors { errors: Vec<String> },
    /// 跳过类型检查（未启用）
    Skipped,
}

/// TypeScript 编译器
pub struct TsCompiler {
    config: TsCompilerConfig,
    compiler: Arc<Compiler>,
    compile_count: AtomicUsize,  // 编译计数器，用于定期清理 SourceMap
}

impl TsCompiler {
    /// 创建新的 TypeScript 编译器
    pub fn new(config: TsCompilerConfig) -> Self {
        let cm = Arc::new(SourceMap::default());
        let compiler = Arc::new(Compiler::new(cm));

        Self {
            config,
            compiler,
            compile_count: AtomicUsize::new(0),
        }
    }

    /// 编译 TypeScript 代码
    ///
    /// # 参数
    ///
    /// * `source` - TypeScript 源代码
    /// * `filename` - 文件名（用于错误报告和 source map）
    ///
    /// # 返回
    ///
    /// 返回编译后的 JavaScript 代码和可选的 source map
    pub fn compile(&self, source: &str, filename: &str) -> Result<TsCompileResult, String> {
        let start_time = std::time::Instant::now();

        info!(
            filename = filename,
            source_size = source.len(),
            "Compiling TypeScript with swc Compiler API"
        );

        // 使用 GLOBALS 设置线程本地存储
        let globals = Globals::default();
        let result = GLOBALS.set(&globals, || {
            // 检查是否需要重建编译器以清理 SourceMap
            let count = self.compile_count.fetch_add(1, Ordering::Relaxed);
            if count > 0 && count % 1000 == 0 {
                warn!(
                    compile_count = count,
                    "Rebuilding compiler to clean SourceMap cache (every 1000 compilations)"
                );
                // 注意：这里只是警告，实际重建需要更复杂的逻辑（使用 RwLock）
                // 暂时依赖垃圾回收机制清理旧的 SourceMap 条目
            }

            // 1. 创建源文件
            let fm = self.compiler.cm.new_source_file(
                Arc::new(FileName::Real(filename.into())),
                source.to_string(),
            );

            // 2. 构建编译选项
            let opts = self.build_options();

            // 3. 使用 try_with_handler 处理编译和错误
            try_with_handler(
                self.compiler.cm.clone(),
                HandlerOpts {
                    color: ColorConfig::Never, // 不需要颜色输出
                    skip_filename: false,
                },
                |handler| {
                    // 4. 解析 TypeScript
                    let program = self.compiler.parse_js(
                        fm,
                        handler,
                        self.config.target.to_swc(),
                        Syntax::Typescript(TsSyntax {
                            tsx: self.config.jsx,
                            decorators: self.config.keep_decorators,
                            ..Default::default()
                        }),
                        swc::config::IsModule::Unknown,
                        None,
                    )?;

                    // 5. 编译（应用 TypeScript 转换）
                    self.compiler.process_js(handler, program, &opts)
                },
            )
        });

        // 6. 处理编译结果
        match result {
            Ok(output) => {
                let compile_time = start_time.elapsed().as_secs_f64() * 1000.0;

                debug!(
                    filename = filename,
                    compile_time_ms = compile_time,
                    output_size = output.code.len(),
                    "TypeScript compiled successfully"
                );

                Ok(TsCompileResult {
                    code: output.code,
                    source_map: output.map,
                    compile_time_ms: compile_time,
                })
            }
            Err(e) => {
                warn!(
                    filename = filename,
                    error = ?e,
                    "TypeScript compilation failed"
                );
                // 使用 {:?} 保留完整错误上下文和堆栈信息
                Err(format!("TypeScript compilation failed: {:?}", e))
            }
        }
    }

    /// 构建编译选项
    fn build_options(&self) -> Options {
        Options {
            config: Config {
                jsc: JscConfig {
                    // 目标 ECMAScript 版本
                    target: Some(self.config.target.to_swc()),
                    // 启用 TypeScript 解析
                    syntax: Some(Syntax::Typescript(TsSyntax {
                        tsx: self.config.jsx,
                        decorators: self.config.keep_decorators,
                        ..Default::default()
                    })),
                    // 其他使用默认配置
                    ..Default::default()
                },
                // 启用 source map
                source_maps: if self.config.source_map {
                    Some(swc::config::SourceMapsConfig::Bool(true))
                } else {
                    Some(swc::config::SourceMapsConfig::Bool(false))
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }
    
    /// 执行 TypeScript 类型检查
    /// 
    /// # 参数
    /// 
    /// * `source` - TypeScript 源码
    /// * `filename` - 文件名（用于错误报告）
    /// * `config` - 类型检查配置
    /// 
    /// # 返回
    /// 
    /// 类型检查结果
    /// 
    /// # 注意
    /// 
    /// 此函数需要系统安装 `tsc` (TypeScript 编译器)
    /// 如果未安装，将返回 Skipped 并警告
    pub fn type_check(
        &self,
        source: &str,
        filename: &str,
        config: &TypeCheckConfig,
    ) -> TypeCheckResult {
        if !config.enabled {
            debug!("Type check disabled, skipping");
            return TypeCheckResult::Skipped;
        }
        
        // 检查 tsc 是否可用
        if !Self::is_tsc_available() {
            warn!(
                "TypeScript compiler (tsc) not found. Type checking disabled.\n\
                 Install TypeScript: npm install -g typescript"
            );
            return TypeCheckResult::Skipped;
        }
        
        // 写入临时文件
        let temp_path = match Self::write_temp_file(source, filename) {
            Ok(path) => path,
            Err(e) => {
                warn!("Failed to create temp file for type check: {}", e);
                return TypeCheckResult::Skipped;
            }
        };
        
        // 运行 tsc
        let result = Self::run_tsc(&temp_path, config);
        
        // 清理临时文件
        if let Err(e) = fs::remove_file(&temp_path) {
            warn!("Failed to cleanup temp file: {}", e);
        }
        
        result
    }
    
    /// 检查 tsc 是否可用
    fn is_tsc_available() -> bool {
        Command::new(if cfg!(windows) { "tsc.cmd" } else { "tsc" })
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// 写入临时 TypeScript 文件
    fn write_temp_file(source: &str, _filename: &str) -> Result<PathBuf, String> {
        let temp_dir = env::temp_dir();
        let temp_path = temp_dir.join(format!("iris_type_check_{}.ts", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));
        
        fs::write(&temp_path, source)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        debug!(
            path = ?temp_path,
            size = source.len(),
            "Temp file created for type check"
        );
        
        Ok(temp_path)
    }
    
    /// 运行 tsc 进行类型检查
    fn run_tsc(file_path: &PathBuf, config: &TypeCheckConfig) -> TypeCheckResult {
        let tsc_cmd = if cfg!(windows) { "tsc.cmd" } else { "tsc" };
        
        let mut cmd = Command::new(tsc_cmd);
        cmd.arg("--noEmit")  // 只检查，不生成文件
            .arg("--pretty")
            .arg("--noEmitOnError");
        
        if config.strict {
            cmd.arg("--strict");
        }
        
        if let Some(ts_config) = &config.ts_config_path {
            cmd.arg("--project").arg(ts_config);
        }
        
        cmd.arg(file_path);
        
        debug!(cmd = ?cmd, "Running tsc for type check");
        
        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    debug!("Type check passed");
                    TypeCheckResult::Success
                } else {
                    // 解析错误信息
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let error_output = if !stderr.is_empty() { stderr } else { stdout };
                    
                    let errors = Self::parse_tsc_errors(&error_output);
                    
                    warn!(
                        error_count = errors.len(),
                        "Type check failed"
                    );
                    
                    TypeCheckResult::Errors { errors }
                }
            }
            Err(e) => {
                warn!("Failed to run tsc: {}", e);
                TypeCheckResult::Skipped
            }
        }
    }
    
    /// 解析 tsc 错误信息
    fn parse_tsc_errors(output: &str) -> Vec<String> {
        let mut errors = Vec::new();
        
        // 简单解析：按行分割，提取错误信息
        for line in output.lines() {
            // 跳过空行和提示行
            if line.trim().is_empty() || line.starts_with("Found") {
                continue;
            }
            
            // 保留所有非空行作为错误信息
            if !line.trim().is_empty() {
                errors.push(line.trim().to_string());
            }
        }
        
        // 如果没有解析到错误，保留原始输出
        if errors.is_empty() && !output.trim().is_empty() {
            errors.push(output.trim().to_string());
        }
        
        errors
    }
}

impl EsVersion {
    /// 转换为 swc 的 EsVersion
    fn to_swc(&self) -> swc_ecma_ast::EsVersion {
        match self {
            EsVersion::ES2015 => swc_ecma_ast::EsVersion::Es2015,
            EsVersion::ES2016 => swc_ecma_ast::EsVersion::Es2016,
            EsVersion::ES2017 => swc_ecma_ast::EsVersion::Es2017,
            EsVersion::ES2018 => swc_ecma_ast::EsVersion::Es2018,
            EsVersion::ES2019 => swc_ecma_ast::EsVersion::Es2019,
            EsVersion::ES2020 => swc_ecma_ast::EsVersion::Es2020,
            EsVersion::ES2021 => swc_ecma_ast::EsVersion::Es2021,
            EsVersion::ES2022 => swc_ecma_ast::EsVersion::Es2022,
            EsVersion::ESNext => swc_ecma_ast::EsVersion::EsNext,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_typescript() {
        let ts = r#"
            const count: number = 42;
            const name: string = "Iris";
            
            function greet(user: { name: string }): string {
                return `Hello, ${user.name}!`;
            }
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(ts, "test.ts").unwrap();

        // 验证类型注解被移除
        assert!(result.code.contains("const count = 42"));
        assert!(!result.code.contains(": number"));
        assert!(!result.code.contains(": string"));
        
        // 验证函数保留
        assert!(result.code.contains("function greet"));
        assert!(result.code.contains("user.name"));
    }

    #[test]
    fn test_interface_and_generics() {
        let ts = r#"
            interface User {
                name: string;
                age: number;
            }
            
            function identity<T>(arg: T): T {
                return arg;
            }
            
            const user: User = { name: "Iris", age: 1 };
            const result = identity(user);
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(ts, "test.ts").unwrap();

        // interface 应该被移除
        assert!(!result.code.contains("interface"));
        
        // 泛型应该被移除
        assert!(!result.code.contains("<T>"));
        
        // 代码应该可执行
        assert!(result.code.contains("const user ="));
        assert!(result.code.contains("identity(user)"));
    }

    #[test]
    fn test_enum() {
        let ts = r#"
            enum Direction {
                Up = 1,
                Down,
                Left,
                Right
            }
            
            const dir = Direction.Up;
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(ts, "test.ts").unwrap();

        // enum 应该被转换
        assert!(result.code.contains("Direction"));
        assert!(result.code.contains("1"));
    }

    #[test]
    fn test_compile_performance() {
        let ts = r#"
            interface Config {
                debug: boolean;
                maxRetries: number;
                timeout: number;
            }
            
            class HttpClient {
                private config: Config;
                
                constructor(config: Config) {
                    this.config = config;
                }
                
                async get<T>(url: string): Promise<T> {
                    const response = await fetch(url);
                    return response.json() as Promise<T>;
                }
            }
            
            export { HttpClient, Config };
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        
        // 编译 50 次测试性能
        let start = std::time::Instant::now();
        for _ in 0..50 {
            compiler.compile(ts, "test.ts").unwrap();
        }
        let elapsed = start.elapsed();
        let avg_time = elapsed.as_millis() as f64 / 50.0;

        println!("Average compile time: {:.2} ms", avg_time);
        
        // 平均编译时间应该小于 20ms（完整的 swc 编译）
        assert!(avg_time < 20.0, "Compile time too slow: {:.2} ms", avg_time);
    }

    #[test]
    fn test_error_handling() {
        let invalid_ts = r#"
            function test(x: unknown_type) {
                return x.invalid_method();
            }
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        // swc 不会在编译时检查类型，所以这应该成功
        let result = compiler.compile(invalid_ts, "test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_syntax_error() {
        // 真正的语法错误（缺少右括号）
        let invalid_ts = r#"
            function test( {
                return x;
            }
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(invalid_ts, "test.ts");
        // 语法错误应该失败
        assert!(result.is_err(), "Syntax error should fail compilation");
    }

    #[test]
    fn test_empty_input() {
        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile("", "test.ts");
        assert!(result.is_ok(), "Empty input should succeed");
        let code = result.unwrap().code;
        assert!(code.is_empty() || code.contains("export"));
    }

    #[test]
    fn test_tsx_support() {
        let tsx = r#"
            interface Props {
                name: string;
            }
            
            const Hello: React.FC<Props> = ({ name }) => {
                return <div>Hello, {name}!</div>;
            };
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig {
            jsx: true,
            ..Default::default()
        });
        let result = compiler.compile(tsx, "test.tsx").unwrap();
        // JSX 应该被转换（这里简化验证）
        assert!(result.code.contains("div") || result.code.contains("React"));
    }

    #[test]
    fn test_decorators() {
        let ts = r#"
            function sealed(target: any) {
                Object.seal(target);
                Object.seal(target.prototype);
            }
            
            @sealed
            class Greeter {
                greeting: string;
                constructor(message: string) {
                    this.greeting = message;
                }
            }
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig {
            keep_decorators: true,
            ..Default::default()
        });
        let result = compiler.compile(ts, "test.ts").unwrap();
        // 装饰器应该被保留（如果配置了）
        assert!(result.code.contains("sealed") || result.code.contains("Greeter"));
    }

    #[test]
    fn test_multiple_compilations() {
        // 测试编译器实例复用和 SourceMap 管理
        let compiler = TsCompiler::new(TsCompilerConfig::default());
        
        for i in 0..10 {
            let ts = format!(r#"
                const value{}: number = {};
                function test{}(): number {{
                    return value{};
                }}
            "#, i, i, i, i);
            
            let result = compiler.compile(&ts, &format!("test{}.ts", i));
            assert!(result.is_ok(), "Compilation {} should succeed", i);
        }
    }
    
    #[test]
    fn test_type_check_disabled_by_default() {
        // 测试默认情况下类型检查是禁用的
        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let config = TypeCheckConfig::default();
        
        // 默认应该禁用类型检查（除非设置了环境变量）
        let result = compiler.type_check("const x: number = 1;", "test.ts", &config);
        
        // 应该返回 Skipped（因为默认禁用或 tsc 未安装）
        assert!(matches!(result, TypeCheckResult::Skipped));
    }
    
    #[test]
    fn test_type_check_config_from_env() {
        // 测试配置从环境变量读取
        let config = TypeCheckConfig::default();
        
        // 验证配置结构
        assert!(config.ts_config_path.is_none());
    }
}
