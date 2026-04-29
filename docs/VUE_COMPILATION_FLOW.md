# Vue 项目反向依赖编译流程

## 概述

iris-jetcrab-engine 采用**自底向上**的反向依赖编译策略，确保所有依赖模块在 App.vue 编译前都已就绪。

## 核心设计理念

### 1. 忽略构建工具

不依赖 package.json 中配置的构建工具（vite/webpack/nuxt），直接使用 iris-jetcrab-engine 自带的编译器处理 Vue 项目。

**优势：**
- ✅ 统一编译流程
- ✅ 减少外部依赖
- ✅ 更好的可控性
- ✅ 支持最小 demo

### 2. 反向依赖解析

从 App.vue 入口文件开始，递归分析所有 import 依赖，构建完整的依赖图。

```
App.vue
  ├── Header.vue
  │   ├── Logo.vue
  │   └── NavMenu.vue
  ├── Sidebar.vue
  │   └── MenuItem.vue
  └── Footer.vue
```

### 3. 拓扑排序编译

按照依赖关系的拓扑排序进行编译，确保：
- 叶子节点（无依赖的模块）先编译
- 根节点（App.vue）最后编译
- 所有依赖在使用前已编译完成

---

## 完整编译流程

### 步骤 1: 接收 Vue 项目目录

```rust
let project_root = PathBuf::from("/path/to/vue-project");
let mut engine = JetCrabEngine::new();
engine.initialize().await?;
engine.load_project(&project_root).await?;
```

**检测策略：**
1. 扫描 package.json 中的 Vue 依赖
2. 查找 .vue 文件（支持最小 demo）
3. 检测 Vue 配置文件
4. 检查 index.html 中的 Vue CDN

### 步骤 2: 确定入口文件

优先级顺序：
1. `src/main.js` / `src/main.ts`
2. `main.js` / `main.ts`
3. `src/App.vue`
4. `App.vue`
5. `app.vue` (Nuxt)

### 步骤 3: 构建依赖图

```rust
let mut compiler = VueProjectCompiler::new(project_root);
let dependency_graph = compiler.build_dependency_graph(&entry_path)?;
```

**过程：**
1. 读取 App.vue 内容
2. 使用 iris-sfc 编译获取 script 部分
3. 解析 import 语句：
   - 静态导入：`import Foo from './components/Foo.vue'`
   - 动态导入：`import('./lazy.vue')`
   - CommonJS：`require('./common.js')`
4. 对每个依赖递归执行步骤 1-3
5. 构建完整的依赖图（HashMap）

**示例依赖图：**
```json
{
  "src/App.vue": ["src/components/Header.vue", "src/components/Footer.vue"],
  "src/components/Header.vue": ["src/components/Logo.vue"],
  "src/components/Logo.vue": [],
  "src/components/Footer.vue": []
}
```

### 步骤 4: 拓扑排序

```rust
let compilation_order = compiler.topological_sort(&dependency_graph)?;
```

**排序结果（从叶子到根）：**
```
1. src/components/Logo.vue     (无依赖)
2. src/components/Footer.vue   (无依赖)
3. src/components/Header.vue   (依赖 Logo.vue)
4. src/App.vue                 (依赖 Header.vue, Footer.vue)
```

### 步骤 5: 按顺序编译

```rust
for module_path in &compilation_order {
    let compiled = compiler.compile_single_module(module_path)?;
    compiled_modules.insert(module_path.clone(), compiled);
}
```

**编译过程：**
1. 读取 .vue 文件内容
2. 使用 iris-sfc 编译：
   - 提取 `<template>` → 编译为渲染函数
   - 提取 `<script>` → 转换为 JavaScript
   - 提取 `<style>` → 处理 CSS
3. 解析依赖列表
4. 缓存编译结果

### 步骤 6: 执行模块

```rust
// 按编译顺序执行（从叶子到根）
for module_path in &compilation_order {
    if let Some(compiled) = result.compiled_modules.get(module_path) {
        runtime.eval(&compiled.script)?;
    }
}
```

**执行顺序：**
1. Logo.vue 的 script → 注册组件
2. Footer.vue 的 script → 注册组件
3. Header.vue 的 script → 使用 Logo.vue，注册组件
4. App.vue 的 script → 使用 Header.vue 和 Footer.vue，创建应用实例

### 步骤 7: 渲染应用

```rust
// TODO: 使用 iris-gpu 启动渲染循环
// - 创建 GPU 渲染上下文
// - 构建虚拟 DOM 树
// - 计算布局（Flex/Grid）
// - 渲染到屏幕
```

---

## 核心模块

### 1. VueProjectCompiler

**位置:** `crates/iris-jetcrab-engine/src/vue_compiler.rs`

**主要功能:**
- `compile_project()` - 编译整个项目
- `build_dependency_graph()` - 构建依赖图
- `topological_sort()` - 拓扑排序
- `compile_single_module()` - 编译单个模块

**关键数据结构:**
```rust
pub struct CompilationResult {
    pub compiled_modules: HashMap<String, CompiledModule>,
    pub compilation_order: Vec<String>,
    pub entry_file: String,
}
```

### 2. CompiledModule

**位置:** `crates/iris-jetcrab-engine/src/sfc_compiler.rs`

```rust
pub struct CompiledModule {
    pub script: String,           // 转换后的 JavaScript
    pub styles: Vec<StyleBlock>,  // 样式块
    pub deps: Vec<String>,        // 依赖列表
}
```

### 3. JetCrabEngine

**位置:** `crates/iris-jetcrab-engine/src/engine.rs`

**编排流程:**
```rust
pub async fn run(&mut self) -> Result<()> {
    // 1. 获取入口文件
    let entry_file = self.project_info.entry_file;
    
    // 2. 使用 VueProjectCompiler 编译整个项目
    let mut compiler = VueProjectCompiler::new(project_root);
    let compilation_result = compiler.compile_project(&entry_relative).await?;
    
    // 3. 创建 JetCrab 运行时
    let mut runtime = iris_jetcrab::JetCrabRuntime::new();
    runtime.init()?;
    
    // 4. 按编译顺序执行模块
    for module_path in &compilation_result.compilation_order {
        let compiled = compilation_result.compiled_modules.get(module_path);
        runtime.eval(&compiled.script)?;
    }
    
    // 5. 启动渲染循环
    // TODO
}
```

---

## 示例：完整编译过程

### Vue 项目结构

```
my-vue-app/
├── package.json          # 忽略构建工具
├── index.html
└── src/
    ├── main.js           # 入口文件
    ├── App.vue
    └── components/
        ├── Header.vue
        ├── Footer.vue
        └── Logo.vue
```

### 编译日志

```
[INFO]  Initializing JetCrab Engine...
[INFO]  Loading Vue project from: "/path/to/my-vue-app"
[INFO]  Entry file: "src/main.js"
[INFO]  Compiling project with entry: src/main.js
[DEBUG] Building dependencies for: src/main.js
[DEBUG] Building dependencies for: src/App.vue
[DEBUG] Building dependencies for: src/components/Header.vue
[DEBUG] Building dependencies for: src/components/Logo.vue
[DEBUG] Building dependencies for: src/components/Footer.vue
[INFO]  Dependency graph built with 5 modules
[DEBUG] Compilation order: [
  "src/components/Logo.vue",
  "src/components/Footer.vue",
  "src/components/Header.vue",
  "src/App.vue",
  "src/main.js"
]
[INFO]  Compiling module: src/components/Logo.vue
[INFO]  Compiling module: src/components/Footer.vue
[INFO]  Compiling module: src/components/Header.vue
[INFO]  Compiling module: src/App.vue
[INFO]  Compiling module: src/main.js
[INFO]  Project compilation complete: 5 modules compiled
[DEBUG] Executing module: src/components/Logo.vue
[DEBUG] Executing module: src/components/Footer.vue
[DEBUG] Executing module: src/components/Header.vue
[DEBUG] Executing module: src/App.vue
[DEBUG] Executing module: src/main.js
[INFO]  Vue application started successfully
```

---

## 特性支持

### ✅ 已实现

- [x] 反向依赖解析
- [x] 拓扑排序
- [x] 循环依赖检测（跳过并警告）
- [x] 编译缓存
- [x] Vue 2/3 支持
- [x] 静态导入解析
- [x] 动态导入解析
- [x] CommonJS require 解析
- [x] 模块路径解析（相对路径/绝对路径）
- [x] 最小 demo 支持

### 🚧 待实现

- [ ] TypeScript 完整支持
- [ ] CSS 预处理器（Sass/Less）
- [ ] 别名解析（@/components）
- [ ] 外部依赖处理（node_modules）
- [ ] 热更新（HMR）增量编译
- [ ] Source Map 生成
- [ ] 渲染循环实现

---

## 错误处理

### 循环依赖

```
[WARN] Circular dependency detected: A.vue -> B.vue -> A.vue
[INFO] Skipping circular dependency (continues compilation)
```

### 缺失依赖

```
[WARN] Dependency not found: ./Missing.vue (imported from App.vue)
[INFO] Continues compilation (module will be undefined)
```

### 编译错误

```
[ERROR] Failed to compile App.vue: Invalid template syntax
[ERROR] Compilation aborted
```

---

## 性能优化

### 1. 编译缓存

```rust
// 检查缓存
if let Some(cached) = self.compiled_cache.get(module_path) {
    return Ok(cached.clone());
}
```

### 2. 并行编译（未来）

```rust
// TODO: 使用 tokio 并行编译无依赖关系的模块
let futures: Vec<_> = independent_modules
    .iter()
    .map(|m| compile_module_async(m))
    .collect();
let results = join_all(futures).await;
```

### 3. 增量编译（HMR）

```rust
// TODO: 只重新编译修改的模块及其依赖者
if file_changed("Header.vue") {
    recompile("Header.vue");
    recompile_dependents("Header.vue"); // App.vue
}
```

---

## 与 iris-runtime 集成

### dev-server.js 调用流程

```javascript
// 1. 用户选择 Vue 项目目录
const projectPath = selectDirectory();

// 2. 验证项目
const validation = await fetch('/api/validate-project', {
  method: 'POST',
  body: JSON.stringify({ path: projectPath })
});

// 3. 启动引擎
const result = await fetch('/api/start-engine', {
  method: 'POST',
  body: JSON.stringify({ projectPath })
});

// 4. 引擎内部执行
// JetCrabEngine::load_project()
// JetCrabEngine::run()
//   → VueProjectCompiler::compile_project()
//   → 拓扑排序
//   → 按顺序编译
//   → 执行模块
//   → 渲染应用
```

---

## 总结

通过**反向依赖解析 + 拓扑排序**的编译策略，iris-jetcrab-engine 能够：

1. ✅ **智能识别** Vue 项目（无需构建工具配置）
2. ✅ **自动解析** 完整的依赖关系图
3. ✅ **正确排序** 编译顺序（依赖优先）
4. ✅ **高效编译** 支持缓存和增量更新
5. ✅ **灵活执行** 按顺序执行模块代码
6. ✅ **完整渲染** 为 GPU 渲染做好准备

这确保了整个 Vue 项目的所有依赖都能被正确编译和渲染！
