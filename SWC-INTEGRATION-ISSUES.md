# swc 集成问题报告

**日期**: 2026-04-24  
**状态**: ⚠️ 存在版本兼容性问题  

---

## 📋 问题概述

在集成 swc TypeScript 编译器时，遇到了严重的依赖版本冲突问题，导致无法编译。

---

## ❌ 遇到的错误

### 错误 1: `unicode-id-start` 版本冲突

```
error: failed to select a version for `unicode-id-start`.
    ... required by package `swc_ecma_ast v0.118.0`
    ... which satisfies dependency `unicode-id-start = "^1.2.0"`
    
  previously selected package `unicode-id-start v1.0.4`
    ... which satisfies dependency `unicode-id-start = "=1.0.4"` 
    of package `swc_ecma_codegen v0.151.0`
```

**原因**: `swc_ecma_parser` 和 `swc_ecma_codegen` 依赖不同版本的 `swc_ecma_ast`，而这两个 `swc_ecma_ast` 版本又依赖不同版本的 `unicode-id-start`。

---

### 错误 2: `serde::__private` 不存在

```
error[E0432]: unresolved import `serde::__private`
   --> swc_config-0.1.15\src\config_types\bool_or_data.rs:128:20
    |
128 |         use serde::__private::de;
    |                    ^^^^^^^^^ could not find `__private` in `serde`
```

**原因**: 旧版本的 `swc_config` (0.1.15) 使用了 serde 的内部 API `__private`，该 API 在 serde 1.0.228 中已被移除。

---

### 错误 3: API 变更

```
error[E0432]: unresolved import `swc_ecma_parser::TsSyntax`
  --> crates\iris-sfc\src\ts_compiler.rs:15:52
   |
15 | use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
   |                                                    ^^^^^^^^
   |                                                    no `TsSyntax` in the root
```

**原因**: swc 的 API 在不同版本之间有重大变更：
- `TsSyntax` 在 0.141 版本中改名为 `TsConfig`
- `strip_with_config` 函数签名变更
- Source map 类型从 `Vec<u8>` 改为 `Vec<(BytePos, LineCol)>`

---

## 🔍 根本原因分析

swc 是一个快速迭代的项目，其子包（`swc_ecma_parser`, `swc_ecma_codegen`, `swc_ecma_ast` 等）版本必须**精确匹配**。但问题是：

1. **版本锁定困难**: Cargo 的语义化版本管理会自动选择兼容版本，但 swc 的子包之间即使主版本号相同也可能不兼容
2. **依赖冲突**: 不同 swc 子包可能依赖不同版本的共同依赖（如 `serde`, `unicode-id-start`）
3. **API 不稳定**: swc 的公共 API 在不同版本间频繁变化

---

## 💡 建议的解决方案

### 方案 1: 使用官方 swc 元包（推荐）

使用 `swc` 主包而不是单独引入子包，让 swc 自己管理内部依赖：

```toml
[dependencies]
swc = "0.270"
swc_common = "0.37"
```

```rust
use swc::Compiler;
use swc_common::SourceMap;

let compiler = Compiler::new(Default::default());
let result = compiler.process_js_with_custom_pass(/* ... */);
```

**优点**: 
- 版本由 swc 官方管理，保证兼容
- API 更稳定
- 文档更完善

**缺点**:
- 包体积较大
- 可能引入不需要的功能

---

### 方案 2: 使用 `[patch]` 段强制版本

在 workspace 级别的 `Cargo.toml` 中添加：

```toml
[patch.crates-io]
unicode-id-start = { version = "=1.0.4" }
serde = { version = "=1.0.210" }
```

**优点**:
- 可以强制使用特定版本
- 不需要修改代码

**缺点**:
- 可能导致其他包不兼容
- 维护成本高

---

### 方案 3: 使用替代方案

考虑使用其他 TypeScript 编译器：

#### 3.1 typescript-crystal (Rust binding)
```toml
[dependencies]
typescript-compiler = "1.0"
```

#### 3.2 调用外部 tsc 命令
```rust
use std::process::Command;

let output = Command::new("tsc")
    .arg("--target")
    .arg("ES2020")
    .arg("input.ts")
    .output()?;
```

#### 3.3 esbuild-wasm (通过 WASM)
```toml
[dependencies]
esbuild-wasm = "0.20"
```

**优点**:
- 避免 swc 版本问题
- 可能有更好的性能

**缺点**:
- 需要额外的运行时依赖
- API 可能不同

---

### 方案 4: 等待 swc 稳定（长期方案）

跟踪 swc 的版本发布，等待其 API 稳定后再集成：

- 关注 [swc releases](https://github.com/swc-project/swc/releases)
- 使用最新的稳定版本组合
- 加入 swc Discord/论坛获取帮助

---

## 📝 已尝试的版本组合

| Parser | Transforms | Codegen | Common | 结果 |
|--------|-----------|---------|--------|------|
| 0.149 | 0.234 | 0.151 | 0.37 | ❌ unicode-id-start 冲突 |
| 0.148 | 0.233 | 0.150 | 0.36 | ❌ unicode-id-start 冲突 |
| 0.146 | 0.230 | 0.148 | 0.34 | ❌ serde 版本问题 |
| 0.141 | 0.185 | 0.146 | 0.33 | ❌ serde 版本问题 |

---

## 🎯 下一步行动

### 立即可做：

1. **使用方案 1（推荐）**:
   ```bash
   # 更新 Cargo.toml
   cargo add swc swc_common
   
   # 重写 ts_compiler.rs 使用 Compiler API
   ```

2. **测试外部 tsc 调用**（快速验证）:
   ```rust
   // 临时方案：调用系统 tsc
   let output = std::process::Command::new("npx")
       .args(["tsc", "--target", "ES2020", file])
       .output()?;
   ```

### 中期计划：

3. **联系 swc 社区**:
   - 在 GitHub 上提交 issue
   - 询问推荐的版本组合
   - 获取官方示例代码

4. **创建测试项目**:
   - 独立的 Cargo 项目
   - 只包含 swc 依赖
   - 测试不同版本组合

---

## 📚 相关资源

- [swc 官方文档](https://swc.rs/docs/)
- [swc GitHub](https://github.com/swc-project/swc)
- [swc Discord](https://discord.gg/swc-project)
- [Cargo 版本管理文档](https://doc.rust-lang.org/cargo/reference/resolver.html)

---

## ⚠️ 当前状态

**swc 集成已暂时禁用**，项目恢复到使用简单正则表达式转译 TypeScript 的状态。

要重新启用，需要：
1. 解决版本冲突问题
2. 更新 `ts_compiler.rs` 中的 API 调用
3. 添加完整的单元测试

---

**维护者**: Iris 开发团队  
**最后更新**: 2026-04-24
