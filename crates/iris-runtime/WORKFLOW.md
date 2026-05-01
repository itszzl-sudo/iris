# Iris CLI (@irisverse/iris) 完整工作流

> Iris CLI 的完整工作流

---

## 🎯 两个角色，两个流程

### 角色 1: Iris 开发者（您）

**职责**: 开发和维护 @irisverse/iris npm 包

**工作目录**: `crates/iris-runtime/`

### 角色 2: Vue 开发者（用户）

**职责**: 在自己的 Vue 项目中使用 Iris CLI

**工作目录**: 他们的 Vue 项目目录（如 `~/projects/my-vue-app/`）

---

## 📦 角色 1: Iris 开发者工作流

### 步骤 1: 开发 Rust/WASM 代码

```bash
# 进入 iris-runtime 目录
cd crates/iris-runtime

# 编辑源码
vim src/lib.rs          # WASM 接口
vim src/compiler.rs     # Vue SFC 编译器
vim src/hmr.rs          # 热更新
vim bin/iris.js       # CLI 入口
vim lib/dev-server.js   # 开发服务器
```

### 步骤 2: 编译 WASM

```bash
# 生产构建（发布到 npm）
wasm-pack build --target nodejs --release

# 或使用 npm script
npm run build:wasm

# 或使用构建脚本
./build-wasm.sh          # Linux/macOS
.\build-wasm.ps1         # Windows
```

**输出**:

```
pkg/
├── iris_runtime_bg.wasm       # WASM 二进制 (~5MB)
├── iris_runtime_bg.wasm.d.ts  # TypeScript 类型
├── iris_runtime.js            # JavaScript 绑定
└── iris_runtime.d.ts          # TypeScript 声明
```

### 步骤 3: 本地测试（可选）

```bash
# 创建 npm 包
npm pack

# 输出: iris-runtime-0.1.0.tgz

# 在测试项目中安装
cd /tmp/test-vue-app
npm install /path/to/@irisverse/iris-0.1.0.tgz
iris dev
```

### 步骤 4: 发布到 npm

```bash
# 登录 npm（首次）
npm login

# 更新版本号
npm version patch  # 0.1.0 → 0.1.1
# 或
npm version minor  # 0.1.0 → 0.2.0
# 或
npm version major  # 0.1.0 → 1.0.0

# 发布
npm publish --access public
```

**完成！** 现在任何人都可以 `npm install -g @irisverse/iris`

---

## 🚀 角色 2: Vue 开发者工作流

### 步骤 1: 创建 Vue 项目

```bash
# 使用官方工具创建
npm create vue@latest my-app
cd my-app

# 或使用其他工具
npm create vite@latest my-app -- --template vue
cd my-app
```

### 步骤 2: 全局安装 Iris CLI

```bash
npm install -g @irisverse/iris
```

**发生了什么？**

```
node_modules/
└── @irisverse/iris/
    ├── pkg/
    │   ├── iris_runtime_bg.wasm    ← WASM 运行时（已编译好）
    │   └── iris_runtime.js         ← JS 绑定
    ├── bin/
    │   └── iris.js                 ← CLI 工具
    └── lib/
        └── dev-server.js           ← 开发服务器
```

### 步骤 3: 启动开发服务器

```bash
iris dev
```

**输出**:

```
🚀 Starting Iris Runtime dev server...

  ➜ Local: http://localhost:3000
  ➜ Network: use --host to expose
  ➜ Ready in 234ms
```

### 步骤 4: 开发 Vue 应用

```
浏览器访问: http://localhost:3000

文件变更 → 自动热更新 → 浏览器实时预览
```

### 步骤 5: 生产构建（使用其他工具）

```bash
# 使用 vite 或其他工具
npm run build

# iris-runtime 只负责开发环境！
```

---

## 📊 完整流程图

```
┌─────────────────────────────────────────┐
│  Iris 开发者 (you)                      │
│                                         │
│  1. 编辑 Rust 代码                      │
│     crates/iris-runtime/src/*.rs        │
│                                         │
│  2. 编译 WASM                           │
│     wasm-pack build --target nodejs     │
│                                         │
│  3. 发布到 npm                          │
│     npm publish                         │
└──────────────┬──────────────────────────┘
               │
               │ npm registry
               │
               ▼
┌─────────────────────────────────────────┐
│  Vue 开发者 (user)                      │
│                                         │
│  1. 创建 Vue 项目                       │
│     npm create vue@latest my-app        │
│                                         │
│  2. 全局安装 Iris CLI                    │
│     npm install -g @irisverse/iris   │
│                                         │
│  3. 启动开发服务器                      │
│     iris dev                │
│                                         │
│  4. 开发 & 热更新                       │
│     编辑 .vue 文件 → 自动刷新           │
└─────────────────────────────────────────┘
```

---

## 🎯 关键理解

### ✅ Iris CLI (@irisverse/iris) 做什么

1. **编译 Vue SFC** (WASM)
2. **提供开发服务器** (Node.js)
3. **热模块替换** (WebSocket)
4. **实时预览** (HTTP)

### ❌ Iris CLI 不做什么

1. **不创建项目** → 使用 `create-vue`
2. **不生产构建** → 使用 `vite build`
3. **不管理依赖** → 使用 `npm install`
4. **不打包发布** → 使用 `vite build` 或其他

---

## 💡 为什么这样设计？

### 优势 1: 专注单一职责

```
Iris CLI 只做好一件事：
→ 快速的开发体验 + 热更新
```

### 优势 2: 生态兼容

```
不重复造轮子：
- 项目创建: create-vue (官方)
- 生产构建: vite (成熟稳定)
- 包管理: npm/yarn/pnpm (标准工具)
```

### 优势 3: WASM 优势

```
Iris CLI 的核心竞争力：
- 基于 WASM 的快速编译
- 跨平台一致性
- 零原生依赖
- 轻量级 (~5MB)
```

---

## 📝 实际示例

### Iris 开发者的一天

```bash
# 早上：修复 bug
cd ~/projects/iris/crates/iris-runtime
vim src/compiler.rs
cargo check  # ✅ 编译通过

# 编译 WASM
wasm-pack build --target nodejs --release
# ✅ WASM 编译成功 (4.8MB)

# 测试
npm pack
cd /tmp/test-app
npm install ~/projects/iris/iris-runtime/irisverse-iris-0.1.1.tgz
iris dev
# ✅ 开发服务器正常，热更新工作

# 下午：发布
cd ~/projects/iris/crates/iris-runtime
npm version patch
npm publish --access public
# ✅ 发布成功！
```

### Vue 开发者的一天

```bash
# 创建新项目
npm create vue@latest my-project
cd my-project

# 安装 Iris CLI
npm install -g @irisverse/iris

# 启动开发
iris dev
# ✅ 浏览器自动打开 http://localhost:3000

# 开发功能
vim src/App.vue  # 编辑
# 保存 → 浏览器自动更新 ✅

# 添加组件
vim src/components/Foo.vue
# 保存 → 热更新 ✅

# 完成开发，准备发布
npm run build  # 使用 vite 生产构建
# ✅ dist/ 生成
```

---

## 🚀 快速开始

### 对于 Iris 开发者

```bash
cd crates/iris-runtime

# 开发
vim src/*.rs

# 编译
wasm-pack build --target nodejs --release

# 发布
npm publish
```

### 对于 Vue 开发者

```bash
# 在 Vue 项目中
npm install -g @irisverse/iris
iris dev

# 开始开发！
```

---

## 📚 相关文档

- [DEVELOPMENT.md](./DEVELOPMENT.md) - 详细开发指南
- [README.md](./README.md) - 使用说明
- [Cargo.toml](./Cargo.toml) - Rust 配置
- [package.json](./package.json) - npm 配置

---

**文档维护者**: Iris Development Team  
**最后更新**: 2026-04-28
