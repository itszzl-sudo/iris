# Iris Runtime 架构调整说明

> **调整日期**: 2026-04-28  
> **状态**: ✅ 已完成

---

## 📋 变更概述

`iris-runtime` 从一个 **Rust crate + npm 包混合项目** 调整为 **纯 npm 包项目**。

### 变更前

```
crates/iris-runtime/
├── Cargo.toml          ❌ 已删除
├── src/                ❌ 已删除
│   ├── lib.rs          (WASM 导出接口)
│   ├── compiler.rs     (SFC 编译)
│   └── hmr.rs          (HMR 补丁生成)
├── bin/                ✅ 保留
│   └── iris-runtime.js
├── lib/                ✅ 保留
│   ├── dev-server.js
│   └── templates/
│       └── index.html
└── package.json        ✅ 保留
```

### 变更后

```
crates/iris-runtime/
├── bin/                ✅ CLI 入口
│   └── iris-runtime.js
├── lib/                ✅ 开发服务器
│   ├── dev-server.js
│   └── templates/
│       └── index.html
├── package.json        ✅ npm 包配置
└── README.md           ✅ 使用文档
```

---

## 🔄 功能迁移

### 已迁移到 iris-jetcrab-engine

| 原位置 (iris-runtime) | 新位置 (iris-jetcrab-engine) | 状态 |
|----------------------|----------------------------|------|
| `src/lib.rs` (WASM 接口) | 待实现 WASM 导出 | ⏳ |
| `src/compiler.rs` | `src/sfc_compiler.rs` | ✅ 已完成 |
| `src/hmr.rs` | `src/hmr.rs` | ✅ 已完成 |

### iris-jetcrab-engine 新增模块

```
crates/iris-jetcrab-engine/src/
├── lib.rs              # 主模块
├── engine.rs           # 核心编排器
├── project_scanner.rs  # 项目扫描器
├── module_graph.rs     # 模块依赖图
├── hmr.rs              # HMR 管理器
└── sfc_compiler.rs     # ✨ 新增：SFC 编译器
```

---

## 🎯 iris-runtime 新定位

### 职责

`iris-runtime` 是一个 **npm 包**，提供：

1. **CLI 工具** (`bin/iris-runtime.js`)
   - 启动开发服务器
   - 解析命令行参数
   - 调用引擎 WASM 模块

2. **开发服务器** (`lib/dev-server.js`)
   - HTTP 服务器
   - WebSocket HMR
   - 文件监听
   - 静态文件服务

3. **HTML 模板** (`lib/templates/index.html`)
   - 加载引擎 WASM
   - 初始化运行时
   - 挂载 Vue 应用

### 与引擎的关系

```
iris-runtime (npm 包)
    ↓ 调用
┌──────────────┬──────────────┐
│  iris-engine │ 或 iris-     │
│  (Boa)       │  jetcrab-   │
│              │  engine     │
└──────────────┴──────────────┘
         ↓
    共享核心层
    (iris-sfc, iris-dom, iris-layout, iris-gpu)
```

**关键点**:
- iris-runtime **不实现编译逻辑**
- 它通过 WASM 调用引擎的编译能力
- 两个引擎的 WASM 产出物名称相同（统一接口）

---

## 📦 WASM 接口统一

### iris-engine WASM 导出（待实现）

```rust
// crates/iris-engine/src/wasm_api.rs (待创建)
#[wasm_bindgen]
pub struct IrisEngine {
    // ...
}

#[wasm_bindgen]
impl IrisEngine {
    pub fn compile_sfc(&mut self, source: &str, filename: &str) -> Result<String, JsError>;
    pub fn resolve_import(&self, import_path: &str, importer: &str) -> Result<String, JsError>;
    pub fn generate_hmr_patch(&mut self, old: &str, new: &str, filename: &str) -> Result<String, JsError>;
}
```

### iris-jetcrab-engine WASM 导出（待实现）

```rust
// crates/iris-jetcrab-engine/src/wasm_api.rs (待创建)
#[wasm_bindgen]
pub struct IrisEngine {  // 相同的名称！
    // ...
}

#[wasm_bindgen]
impl IrisEngine {
    pub fn compile_sfc(&mut self, source: &str, filename: &str) -> Result<String, JsError>;
    pub fn resolve_import(&self, import_path: &str, importer: &str) -> Result<String, JsError>;
    pub fn generate_hmr_patch(&mut self, old: &str, new: &str, filename: &str) -> Result<String, JsError>;
}
```

**统一接口的优势**:
- iris-runtime 无需关心使用哪个引擎
- 可以动态切换引擎（Boa vs JetCrab）
- npm 包保持单一

---

## 🚀 下一步工作

### 1. 为 iris-jetcrab-engine 添加 WASM 导出

```bash
# 创建 wasm_api.rs
cd crates/iris-jetcrab-engine
# 实现 IrisEngine 结构体和 WASM 导出方法
```

### 2. 为 iris-engine 添加 WASM 导出

```bash
# 创建 wasm_api.rs
cd crates/iris-engine
# 实现相同的接口
```

### 3. 编译 WASM 模块

```bash
# iris-jetcrab-engine
wasm-pack build --target web --release

# iris-engine  
wasm-pack build --target web --release
```

### 4. 更新 iris-runtime

```javascript
// bin/iris-runtime.js
import initEngine, { IrisEngine } from './pkg/iris_engine.js';

async function start() {
  await initEngine();
  const engine = new IrisEngine();
  // 使用 engine.compileSfc() 等
}
```

---

## 📊 当前状态

| 组件 | 状态 | 说明 |
|------|------|------|
| **iris-jetcrab-engine** | ✅ 完成 | SFC 编译 + HMR 功能已实现 |
| **iris-runtime Rust 代码** | ✅ 已清理 | 从 workspace 移除 |
| **iris-runtime npm 包** | ⚠️ 待更新 | 需要适配新架构 |
| **WASM 导出接口** | ❌ 待实现 | 两个引擎都需要 |
| **文档更新** | 🔄 进行中 | 本文档 |

---

## 📝 技术决策

### 为什么将 Rust 代码移到 iris-jetcrab-engine？

1. **职责分离**
   - iris-runtime: npm 包，专注于 CLI 和服务器
   - iris-jetcrab-engine: Rust crate，专注于编译和运行时

2. **引擎无关**
   - iris-runtime 不依赖具体引擎实现
   - 通过统一 WASM 接口调用

3. **可维护性**
   - Rust 代码集中在 engine crate
   - JS 代码集中在 npm 包

4. **发布流程**
   - Rust crate 发布到 crates.io
   - npm 包发布到 npm registry
   - 各自独立版本管理

---

**文档创建日期**: 2026-04-28  
**调整状态**: ✅ 已完成  
**下一步**: 实现 WASM 导出接口
