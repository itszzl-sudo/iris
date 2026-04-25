//! TypeScript 编译器（基于 swc）
//!
//! 功能：
//! - 完整的 TypeScript 到 JavaScript 转译
//! - 支持泛型、接口、装饰器、TSX
//! - Source map 生成
//! - 类型擦除与优化

use swc_common::{
    errors::{ColorConfig, Handler, EmitterWriter},
    source_map::SourceMap,
    sync::Lrc,
    FileName,
};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsConfig};
use swc_ecma_transforms_typescript::strip;
use swc_ecma_codegen::{
    text_writer::JsWriter,
    Emitter,
};
use swc_ecma_ast::{Module, Program};
use tracing::{debug, info};

/// TypeScript 编译配置
#[derive(Debug, Clone)]
pub struct TsCompilerConfig {
    /// 是否启用 JSX/TSX 支持
    pub jsx: bool,
    /// 是否保留装饰器（decorators）
    pub keep_decorators: bool,
    /// 是否生成 source map
    pub source_map: bool,
}

impl Default for TsCompilerConfig {
    fn default() -> Self {
        Self {
            jsx: false,
            keep_decorators: false,
            source_map: true,
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
    source_map: Lrc<SourceMap>,
    handler: Lrc<Handler>,
}

impl TsCompiler {
    /// 创建新的 TypeScript 编译器
    pub fn new(config: TsCompilerConfig) -> Self {
        let source_map = Lrc::new(SourceMap::default());
        
        // 创建错误处理器
        let handler = {
            let emitter = Box::new(EmitterWriter::stderr(ColorConfig::Always));
            Lrc::new(Handler::with_emitter(true, false, emitter))
        };

        Self {
            config,
            source_map,
            handler,
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
            "Compiling TypeScript with swc"
        );

        // 1. 创建源文件
        let source_file = self.source_map.new_source_file(
            FileName::Real(filename.into()),
            source.into(),
        );

        // 2. 解析 TypeScript
        let module = self.parse_typescript(&source_file)?;
        debug!("TypeScript parsed successfully");

        // 3. 应用 TypeScript 转换（类型擦除）
        let transformed = self.transform_typescript(module)?;
        debug!("TypeScript transformed to JavaScript");

        // 4. 生成 JavaScript 代码
        let (code, source_map) = self.generate_code(transformed)?;
        debug!("JavaScript code generated");

        let compile_time = start_time.elapsed().as_secs_f64() * 1000.0;

        info!(
            compile_time_ms = compile_time,
            output_size = code.len(),
            "TypeScript compilation complete"
        );

        Ok(TsCompileResult {
            code,
            source_map,
            compile_time_ms: compile_time,
        })
    }

    /// 解析 TypeScript 源代码
    fn parse_typescript(&self, source_file: &Lrc<swc_common::SourceFile>) -> Result<Module, String> {
        let syntax = Syntax::Typescript(TsConfig {
            tsx: self.config.jsx,
            decorators: self.config.keep_decorators,
            ..Default::default()
        });

        let mut parser = Parser::new(
            syntax,
            StringInput::from(&*source_file),
            None,
        );

        let module = parser.parse_module().map_err(|e| {
            e.into_diagnostic(&self.handler).emit();
            format!("TypeScript parse error: {}", e.kind().msg())
        })?;

        Ok(module)
    }

    /// 应用 TypeScript 转换
    fn transform_typescript(&self, module: Module) -> Result<Module, String> {
        // 应用 strip 转换（移除类型注解）
        let config = TsConfig {
            no_transform_annotations: false,
        };
        let transformed = strip_with_config(module, &config)
            .map_err(|e| format!("TypeScript transform failed: {:?}", e))?;

        Ok(transformed)
    }

    /// 生成 JavaScript 代码
    fn generate_code(&self, module: Module) -> Result<(String, Option<String>), String> {
        let mut buf = Vec::new();
        let mut srcmap = Vec::new();

        // 创建代码生成器
        let mut emitter = Emitter {
            cfg: Default::default(),
            cm: self.source_map.clone(),
            comments: None,
            wr: JsWriter::new(
                self.source_map.clone(),
                "\n",
                &mut buf,
                Some(&mut srcmap),
            ),
        };

        let program = Program::Module(module);

        emitter.emit_program(&program).map_err(|e| {
            format!("Code generation error: {:?}", e)
        })?;

        let code = String::from_utf8(buf).map_err(|e| {
            format!("Invalid UTF-8 in generated code: {}", e)
        })?;

        let source_map = if self.config.source_map {
            String::from_utf8(srcmap).ok()
        } else {
            None
        };

        Ok((code, source_map))
    }

    /// 快速编译（使用默认配置）
    ///
    /// 这是一个便捷方法，使用默认配置编译 TypeScript
    pub fn compile_simple(source: &str, filename: &str) -> Result<String, String> {
        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(source, filename)?;
        Ok(result.code)
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
            
            export { count, name, greet };
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(ts, "test.ts").unwrap();

        assert!(result.code.contains("const count = 42"));
        assert!(result.code.contains("const name = \"Iris\""));
        assert!(result.code.contains("function greet(user)"));
        assert!(!result.code.contains(": number"));
        assert!(!result.code.contains(": string"));
    }

    #[test]
    fn test_interface_and_generics() {
        let ts = r#"
            interface User {
                name: string;
                age: number;
            }
            
            function createUser<T extends User>(data: T): T {
                return data;
            }
            
            const user: User = { name: "Alice", age: 25 };
            const created = createUser<User>(user);
        "#;

        let result = TsCompiler::compile_simple(ts, "test.ts").unwrap();
        
        assert!(result.code.contains("function createUser(data)"));
        assert!(!result.code.contains("interface"));
        assert!(!result.code.contains("<T extends User>"));
    }

    #[test]
    fn test_enum() {
        let ts = r#"
            enum Status {
                Pending = "pending",
                Active = "active",
                Completed = "completed"
            }
            
            const currentStatus: Status = Status.Active;
        "#;

        let result = TsCompiler::compile_simple(ts, "test.ts").unwrap();
        
        // swc 会将枚举转换为 IIFE
        assert!(result.code.contains("Status"));
        assert!(!result.code.contains("enum"));
    }

    #[test]
    fn test_compile_performance() {
        let ts = r#"
            interface Config {
                debug: boolean;
                features: string[];
                metadata: Record<string, unknown>;
            }
            
            class AppConfig {
                private config: Config;
                
                constructor(config: Partial<Config>) {
                    this.config = {
                        debug: false,
                        features: [],
                        metadata: {},
                        ...config
                    };
                }
                
                get isDebug(): boolean {
                    return this.config.debug;
                }
                
                setFeature(name: string, enabled: boolean): void {
                    if (enabled) {
                        this.config.features.push(name);
                    } else {
                        this.config.features = this.config.features.filter(f => f !== name);
                    }
                }
            }
            
            export { AppConfig };
            export type { Config };
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(ts, "performance.ts").unwrap();

        assert!(result.compile_time_ms < 500.0, "Compilation should be fast (< 500ms)");
        assert!(result.code.contains("class AppConfig"));
    }
}
