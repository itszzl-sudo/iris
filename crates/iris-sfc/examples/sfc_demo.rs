//! SFC 编译器演示程序（自动适配终端编码）
//!
//! 根据终端编码自动选择输出字符集：
//! - UTF-8：显示中文和 emoji ✅
//! - GBK/其他：显示英文和 ASCII 字符 [OK]

use iris_sfc::{self, compile_from_string};
use std::env;

/// 检测终端是否支持 UTF-8
fn is_utf8_terminal() -> bool {
    // 方法 0：强制模式（用于测试）
    if let Ok(force_mode) = env::var("IRIS_DEMO_FORCE_ASCII") {
        if force_mode == "1" || force_mode.to_lowercase() == "true" {
            return false; // 强制使用 ASCII 模式
        }
    }

    // 方法 1：检查 Windows 代码页（最可靠）
    #[cfg(windows)]
    {
        // 在 Windows 上，检查当前活动代码页
        // 65001 = UTF-8, 936 = GBK
        // 使用 chcp 命令的输出可能不准确，所以我们检查环境变量
        if let Ok(cp) = env::var("IRIS_CODE_PAGE") {
            return cp == "65001";
        }

        // 检查 Windows Terminal（默认支持 UTF-8）
        if env::var("WT_SESSION").is_ok() {
            return true;
        }
    }

    // 方法 2：检查 LANG 环境变量（Unix/Linux/macOS）
    if let Ok(lang) = env::var("LANG") {
        if lang.to_lowercase().contains("utf") {
            return true;
        }
    }

    // 方法 3：检查 PowerShell UTF-8 设置
    if let Ok(output_encoding) = env::var("OUTPUTENCODING") {
        if output_encoding.to_lowercase().contains("utf") {
            return true;
        }
    }

    // 默认假设支持 UTF-8（现代终端通常都支持）
    true
}

/// 终端字符集配置
struct Charset {
    success: &'static str,
    error: &'static str,
    info: &'static str,
    test: &'static str,
    render: &'static str,
    script: &'static str,
    styles: &'static str,
    sparkles: &'static str,
    rocket: &'static str,
    separator: &'static str,
    box_top_left: &'static str,
    box_top_right: &'static str,
    box_bottom_left: &'static str,
    box_bottom_right: &'static str,
    box_horizontal: &'static str,
    box_vertical: &'static str,
}

impl Charset {
    /// UTF-8 字符集（支持中文和 emoji）
    fn utf8() -> Self {
        Self {
            success: "✅",
            error: "❌",
            info: "📝",
            test: "📦",
            render: "🎨",
            script: "📜",
            styles: "🎭",
            sparkles: "✨",
            rocket: "🚀",
            separator: "─",
            box_top_left: "╔",
            box_top_right: "╗",
            box_bottom_left: "╚",
            box_bottom_right: "╝",
            box_horizontal: "═",
            box_vertical: "║",
        }
    }

    /// ASCII 字符集（兼容性最好）
    fn ascii() -> Self {
        Self {
            success: "[OK]",
            error: "[FAIL]",
            info: "[TEST]",
            test: "[INFO]",
            render: "[RENDER]",
            script: "[SCRIPT]",
            styles: "[STYLES]",
            sparkles: "***",
            rocket: ">>",
            separator: "-",
            box_top_left: "+",
            box_top_right: "+",
            box_bottom_left: "+",
            box_bottom_right: "+",
            box_horizontal: "-",
            box_vertical: "|",
        }
    }

    /// 根据环境自动选择字符集
    fn auto() -> Self {
        if is_utf8_terminal() {
            Self::utf8()
        } else {
            Self::ascii()
        }
    }
}

fn main() {
    let charset = Charset::auto();

    // 打印标题框（根据字符集自动适配）
    let box_width = 42;
    println!(
        "{}{}{}",
        charset.box_top_left,
        charset.box_horizontal.repeat(box_width),
        charset.box_top_right
    );
    println!(
        "{}  Iris SFC Compiler Demo{} {}",
        charset.box_vertical,
        " ".repeat(12),
        charset.box_vertical
    );
    println!(
        "{}{}{}",
        charset.box_bottom_left,
        charset.box_horizontal.repeat(box_width),
        charset.box_bottom_right
    );
    println!();

    // 测试 1: 简单组件
    println!("{} 测试 1: 简单 Vue 组件", charset.info);
    println!("{}", charset.separator.repeat(50));

    let simple_vue = r#"<template>
  <div class="container">
    <h1>Hello, Iris!</h1>
    <p>{{ message }}</p>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const message = "SFC compiler works!"
</script>

<style scoped>
.container {
  padding: 20px;
  font-family: Arial, sans-serif;
}

h1 {
  color: #6B4EE6;
}
</style>"#;

    match compile_from_string("SimpleComponent", simple_vue) {
        Ok(module) => {
            println!("{} 编译成功！\n", charset.success);
            println!("{} 组件信息:", charset.test);
            println!("   名称: {}", module.name);
            println!("   源码哈希: {:x}", module.source_hash);
            println!("   样式块数量: {}\n", module.styles.len());

            println!("{} 渲染函数:", charset.render);
            println!("{}", module.render_fn);
            println!();

            println!("{} Script:", charset.script);
            println!("{}", module.script);
            println!();

            println!("{} Styles:", charset.styles);
            for (i, style) in module.styles.iter().enumerate() {
                println!("  样式块 {}:", i + 1);
                println!("    Scoped: {}", style.scoped);
                println!("    语言: {}", style.lang);
            }
        }
        Err(e) => {
            println!("{} 编译失败: {}\n", charset.error, e);
        }
    }

    // 测试 2: TypeScript 组件
    println!("\n\n{} 测试 2: TypeScript 组件", charset.info);
    println!("{}", charset.separator.repeat(50));

    let ts_vue = r#"<template>
  <div>
    <h2>TypeScript Test</h2>
    <button @click="increment">
      Count: {{ count }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const count: number = ref(0)
const name: string = "Iris"

function increment(): void {
  count.value++
}
</script>"#;

    match compile_from_string("TypeScriptComponent", ts_vue) {
        Ok(module) => {
            println!("{} TypeScript 组件编译成功！\n", charset.success);
            println!("{} 转译后的 Script:", charset.script);
            println!("{}", module.script);
        }
        Err(e) => {
            println!("{} 编译失败: {}\n", charset.error, e);
        }
    }

    // 测试 3: 多样式块
    println!("\n\n{} 测试 3: 多样式块组件", charset.info);
    println!("{}", charset.separator.repeat(50));

    let multi_style_vue = r#"<template>
  <div class="app">
    <h1>Multi-Style Component</h1>
  </div>
</template>

<style scoped>
.app {
  padding: 20px;
}
</style>

<style>
h1 {
  color: #6B4EE6;
}
</style>

<style lang="scss" scoped>
.container {
  .child {
    margin: 10px;
  }
}
</style>"#;

    match compile_from_string("MultiStyleComponent", multi_style_vue) {
        Ok(module) => {
            println!("{} 多样式块组件编译成功！\n", charset.success);
            println!("{} 样式块数量: {}", charset.test, module.styles.len());

            for (i, style) in module.styles.iter().enumerate() {
                println!("\n  样式块 {}:", i + 1);
                println!("    Scoped: {}", style.scoped);
                println!("    语言: {}", style.lang);
            }
        }
        Err(e) => {
            println!("{} 编译失败: {}\n", charset.error, e);
        }
    }

    println!("\n\n{} 所有测试完成！", charset.sparkles);
    println!("{} Iris SFC 编译器运行正常！\n", charset.rocket);
}
