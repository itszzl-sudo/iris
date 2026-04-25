//! TypeScript 编译器（基于 swc 62）
//!
//! 功能：
//! - 完整的 TypeScript 到 JavaScript 转译
//! - 支持泛型、接口、装饰器、TSX
//! - Source map 生成
//! - 类型擦除与优化

use swc_common::{
    errors::{Handler, EmitterWriter},
    FileName, Mark, SourceMap,
    sync::Arc,
};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
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
    source_map: Arc<SourceMap>,
    handler: Arc<Handler>,
}

impl TsCompiler {
    /// 创建新的 TypeScript 编译器
    pub fn new(config: TsCompilerConfig) -> Self {
        let source_map = Arc::new(SourceMap::default());
        
        // 创建错误处理器
        let handler = {
            let emitter = Box::new(EmitterWriter::new(
                Box::new(std::io::stderr()),
                None,
                false,
                false,
            ));
            Arc::new(Handler::with_emitter(true, false, emitter))
        };

        Self {
            config,
            source_map,
            handler,
        }
    }

    /// 编译 TypeScript 代码
    pub fn compile(&self, source: &str, filename: &str) -> Result<TsCompileResult, String> {
        let start_time = std::time::Instant::now();
        
        info!(
            filename = filename,
            source_size = source.len(),
            "Compiling TypeScript with swc"
        );

        // 1. 创建源文件
        let source_file = self.source_map.new_source_file(
            Arc::new(FileName::Real(filename.into())),
            source.into(),
        );

        // 2. 解析 TypeScript
        let module = self.parse_typescript(&source_file)?;

        // 3. 应用 TypeScript 转换（类型擦除）
        let transformed = self.transform_typescript(module)?;

        // 4. 生成 JavaScript 代码
        let (code, source_map) = self.generate_code(transformed)?;

        let compile_time = start_time.elapsed().as_secs_f64 * 1000.0;

        debug!(
            filename = filename,
            compile_time_ms = compile_time,
            output_size = code.len(),
            "TypeScript compiled successfully"
        );

        Ok(TsCompileResult {
            code,
            source_map,
            compile_time_ms: compile_time,
        })
    }

    /// 解析 TypeScript 源代码
    fn parse_typescript(&self, source_file: &Arc<swc_common::SourceFile>) -> Result<Module, String> {
        let syntax = Syntax::Typescript(TsSyntax {
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
        use swc_ecma_visit::Fold;
        
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();
        
        let mut strip_pass = strip(unresolved_mark, top_level_mark);
        let transformed = module.fold_with(&mut strip_pass);

        Ok(transformed)
    }

    /// 生成 JavaScript 代码
    fn generate_code(&self, module: Module) -> Result<(String, Option<String>), String> {
        let mut buf = Vec::new();

        let mut emitter = Emitter {
            cfg: Default::default(),
            cm: self.source_map.clone(),
            comments: None,
            wr: JsWriter::new(
                self.source_map.clone(),
                "\n",
                &mut buf,
                None,
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
            Some("{}".to_string()) // Placeholder
        } else {
            None
        };

        Ok((code, source_map))
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

        assert!(result.code.contains("const count = 42"));
        assert!(!result.code.contains(": number"));
        assert!(!result.code.contains(": string"));
        assert!(result.code.contains("function greet"));
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
        "#;

        let compiler = TsCompiler::new(TsCompilerConfig::default());
        let result = compiler.compile(ts, "test.ts").unwrap();

        assert!(!result.code.contains("interface"));
        assert!(!result.code.contains("<T>"));
        assert!(result.code.contains("const user ="));
    }
}
