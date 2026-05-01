# Iris JetCrab WASM 模块说明

> 如何构建和集成 iris-jetcrab WASM 模块到 iris-runtime

---

## 📋 概述

iris-runtime 的 `index.html` 需要加载 **iris-jetcrab** 的 WASM 模块来执行 Vue 应用。

### 架构流程

```
浏览器加载 index.html
    ↓
加载 /@iris/jetcrab.js (WASM JS 绑定)
    ↓
初始化 WASM 模块 (jetcrab_bg.wasm)
    ↓
创建 JetCrabRuntime 实例
    ↓
读取 Vue 项目文件
    ↓
编译 Vue SFC (通过 iris-sfc)
    ↓
执行 JavaScript (通过 JetCrab 引擎)
    ↓
渲染到 DOM
```

---

## 🔨 构建 iris-jetcrab WASM 模块

### 步骤 1: 添加 wasm-bindgen 依赖

在 `crates/iris-jetcrab/Cargo.toml` 中添加：

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde-wasm-bindgen = "0.6"
serde_json = "1.0"
```

### 步骤 2: 创建 WASM 导出接口

创建 `crates/iris-jetcrab/src/wasm_api.rs`:

```rust
use wasm_bindgen::prelude::*;
use crate::JetCrabRuntime;

#[wasm_bindgen]
pub struct JetCrabRuntime {
    runtime: crate::JetCrabRuntime,
}

#[wasm_bindgen]
impl JetCrabRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsError> {
        let runtime = crate::JetCrabRuntime::new();
        Ok(Self { runtime })
    }

    #[wasm_bindgen]
    pub async fn init(&mut self) -> Result<(), JsError> {
        self.runtime.init()
            .map_err(|e| JsError::new(&format!("Initialization failed: {}", e)))
    }

    #[wasm_bindgen(js_name = setProjectRoot)]
    pub fn set_project_root(&mut self, root: &str) {
        self.runtime.set_project_root(root);
    }

    #[wasm_bindgen(js_name = enableHMR)]
    pub fn enable_hmr(&mut self, enabled: bool) {
        self.runtime.enable_hmr(enabled);
    }

    #[wasm_bindgen(js_name = loadEntry)]
    pub async fn load_entry(&mut self, path: &str) -> Result<(), JsError> {
        self.runtime.load_entry(path)
            .await
            .map_err(|e| JsError::new(&format!("Failed to load entry: {}", e)))
    }

    #[wasm_bindgen(js_name = startApp)]
    pub async fn start_app(&mut self) -> Result<(), JsError> {
        self.runtime.start_app()
            .await
            .map_err(|e| JsError::new(&format!("Failed to start app: {}", e)))
    }

    #[wasm_bindgen(js_name = reloadModule)]
    pub async fn reload_module(&mut self, path: &str) -> Result<(), JsError> {
        self.runtime.reload_module(path)
            .await
            .map_err(|e| JsError::new(&format!("Failed to reload module: {}", e)))
    }
}
```

### 步骤 3: 在 lib.rs 中导出

在 `crates/iris-jetcrab/src/lib.rs` 中添加：

```rust
#[cfg(target_arch = "wasm32")]
pub mod wasm_api;

#[cfg(target_arch = "wasm32")]
pub use wasm_api::JetCrabRuntime;
```

### 步骤 4: 编译 WASM

```bash
cd crates/iris-jetcrab
wasm-pack build --target web --release
```

**注意**: 使用 `--target web` 而不是 `--target nodejs`，因为要在浏览器中运行。

### 步骤 5: 复制到模板目录

```bash
# 创建 assets 目录
mkdir -p crates/iris-runtime/lib/templates/assets

# 复制 WASM 文件
cp crates/iris-jetcrab/pkg/jetcrab_bg.wasm crates/iris-runtime/lib/templates/assets/
cp crates/iris-jetcrab/pkg/jetcrab.js crates/iris-runtime/lib/templates/assets/
cp crates/iris-jetcrab/pkg/jetcrab.d.ts crates/iris-runtime/lib/templates/assets/
```

---

## 📁 文件结构

```
crates/iris-runtime/lib/templates/
├── index.html                    # 主页面（加载 WASM）
└── assets/
    ├── jetcrab.js                # WASM JS 绑定
    ├── jetcrab_bg.wasm           # WASM 二进制
    └── jetcrab.d.ts              # TypeScript 类型
```

---

## 🚀 运行时流程

### 1. 用户启动开发服务器

```bash
iris dev
```

### 2. dev-server.js 提供 index.html

```javascript
// 访问 http://localhost:3000/
// 返回 lib/templates/index.html
```

### 3. 浏览器加载 index.html

```html
<script type="module">
  import initJetCrab, { JetCrabRuntime } from '/@iris/jetcrab.js';
  
  // 初始化 WASM
  await initJetCrab();
  
  // 创建运行时
  const runtime = new JetCrabRuntime();
  await runtime.init();
  
  // 加载 Vue 项目
  await runtime.loadEntry('/src/main.js');
  await runtime.startApp();
</script>
```

### 4. dev-server 提供 WASM 文件

```javascript
// 请求 /@iris/jetcrab.js
// 返回 lib/templates/assets/jetcrab.js

// 请求 /@iris/jetcrab_bg.wasm
// 返回 lib/templates/assets/jetcrab_bg.wasm
```

### 5. JetCrab 运行时执行 Vue 应用

```
JetCrabRuntime.init()
    ↓
加载 Vue 3 运行时
    ↓
编译 src/main.js
    ↓
编译 src/App.vue
    ↓
执行 JavaScript
    ↓
渲染到 #app 容器
```

---

## ⚠️ 注意事项

### 1. WASM 编译目标

- **iris-runtime**: 使用 `--target nodejs`（在 Node.js 中运行）
- **iris-jetcrab**: 使用 `--target web`（在浏览器中运行）

### 2. CORS 配置

WASM 模块需要正确的 MIME 类型：

```javascript
const mimeTypes = {
  '.wasm': 'application/wasm',
  '.js': 'application/javascript',
};
```

### 3. 异步初始化

WASM 模块加载是异步的，需要显示 loading 界面：

```javascript
await initJetCrab();  // 异步
const runtime = new JetCrabRuntime();
await runtime.init();  // 异步
```

### 4. 错误处理

必须捕获 WASM 加载错误：

```javascript
try {
  await initJetCrab();
} catch (error) {
  console.error('WASM loading failed:', error);
  // 显示错误界面
}
```

---

## 📝 开发工作流

### 开发模式

```bash
# 1. 编译 iris-jetcrab WASM
cd crates/iris-jetcrab
wasm-pack build --target web --dev

# 2. 复制到模板目录
cp pkg/* ../iris-runtime/lib/templates/assets/

# 3. 启动开发服务器
cd ../iris-runtime
npm run dev
```

### 生产模式

```bash
# 1. 编译 iris-jetcrab WASM (优化)
cd crates/iris-jetcrab
wasm-pack build --target web --release

# 2. 复制到模板目录
cp pkg/* ../iris-runtime/lib/templates/assets/

# 3. 编译 iris-runtime WASM
cd ../iris-runtime
wasm-pack build --target nodejs --release

# 4. 发布 npm 包
npm publish
```

---

## 🔧 自动化脚本

创建 `build-all-wasm.sh`:

```bash
#!/bin/bash
set -e

echo "🔧 Building all WASM modules..."

# 1. 编译 iris-jetcrab (web target)
echo "📦 Building iris-jetcrab..."
cd crates/iris-jetcrab
wasm-pack build --target web --release
cp pkg/* ../iris-runtime/lib/templates/assets/

# 2. 编译 iris-runtime (nodejs target)
echo "📦 Building iris-runtime..."
cd ../iris-runtime
wasm-pack build --target nodejs --release

echo "✅ All WASM modules built successfully!"
```

---

## 📊 当前状态

| 组件 | 状态 | 说明 |
|------|------|------|
| **index.html 模板** | ✅ 已完成 | lib/templates/index.html |
| **dev-server.js 支持** | ✅ 已完成 | 提供 WASM 和 HTML |
| **iris-jetcrab WASM 接口** | ❌ 待实现 | 需要创建 wasm_api.rs |
| **iris-jetcrab WASM 编译** | ❌ 待执行 | wasm-pack build |
| **WASM 文件复制** | ❌ 待实现 | 手动或自动化 |

---

## 🎯 下一步

1. 为 iris-jetcrab 添加 WASM 支持
2. 创建 `wasm_api.rs` 导出接口
3. 编译 WASM 模块
4. 复制到模板目录
5. 测试端到端流程

---

**文档创建日期**: 2026-04-28  
**状态**: 待实施
