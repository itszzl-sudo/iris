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
#[derive(Debug, Clone, Copy)]
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
    pub source_map: Option<String>,
    /// 编译时间（毫秒）
    pub compile_time_ms: f64,
}

/// TypeScript 编译器
pub struct TsCompiler {
    config: TsCompilerConfig,
    compiler: Arc<Compiler>,
}

impl TsCompiler {
    /// 创建新的 TypeScript 编译器
    pub fn new(config: TsCompilerConfig) -> Self {
        let cm = Arc::new(SourceMap::default());
        let compiler = Arc::new(Compiler::new(cm));

        Self {
            config,
            compiler,
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
                Err(format!("TypeScript compilation failed: {}", e))
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
}
