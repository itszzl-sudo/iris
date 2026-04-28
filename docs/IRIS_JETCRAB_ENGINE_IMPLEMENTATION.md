# Iris JetCrab Engine 模块实现总结

> **创建日期**: 2026-04-28  
> **状态**: ✅ Phase 1 完成  
> **测试**: 11/11 通过

---

## 📋 概述

`iris-jetcrab-engine` 是 JetCrab 方案的核心编排层，负责协调 Vue 项目的加载、编译、执行和渲染。

### 架构定位

```
iris-jetcrab-cli (未来)
    ↓
iris-jetcrab-engine ← 本次实现
    ├─ iris-jetcrab (JS 运行时)
    ├─ iris-sfc (SFC 编译器)
    ├─ iris-dom (DOM 系统)
    ├─ iris-layout (布局引擎)
    └─ iris-gpu (GPU 渲染)
```

---

## 📦 模块结构

```
crates/iris-jetcrab-engine/
├── Cargo.toml                     # 依赖配置
└── src/
    ├── lib.rs                     # 模块导出
    ├── engine.rs                  # 核心编排器 (361 行)
    ├── project_scanner.rs         # 项目扫描器 (268 行)
    ├── module_graph.rs            # 模块依赖图 (222 行 + 测试)
    └── hmr.rs                     # HMR 管理器 (232 行 + 测试)
```

**总计**: ~1,083 行代码 + 11 个测试

---

## 🔧 核心功能

### 1. JetCrabEngine (核心编排器)

**职责**: Vue 项目运行时编排

**主要功能**:
- ✅ 引擎初始化和配置
- ✅ Vue 项目目录加载
- ✅ index.html 解析
- ✅ 模块依赖图构建
- ✅ Vue SFC 编译集成
- ✅ JetCrab 运行时执行
- ✅ HMR 支持

**API 示例**:
```rust
let mut engine = JetCrabEngine::new();
engine.initialize().await?;
engine.load_project("/path/to/vue-project").await?;
engine.run().await?;
```

---

### 2. ProjectScanner (项目扫描器)

**职责**: 扫描和解析 Vue 项目结构

**主要功能**:
- ✅ 查找 index.html（根目录或 public/）
- ✅ 定位 src 目录
- ✅ 识别入口文件（main.js/main.ts 等）
- ✅ 检测构建工具（Vite/Vue CLI）
- ✅ 提取 Vue 版本信息
- ✅ 解析 package.json

**输出**: `ProjectInfo` 结构体
```rust
pub struct ProjectInfo {
    pub root_dir: PathBuf,
    pub index_html_path: PathBuf,
    pub src_dir: PathBuf,
    pub entry_file: PathBuf,
    pub package_json_path: Option<PathBuf>,
    pub build_tool: Option<BuildTool>,
    pub vue_version: Option<String>,
}
```

---

### 3. ModuleGraph (模块依赖图)

**职责**: 管理模块依赖关系，支持循环依赖检测

**主要功能**:
- ✅ 添加模块和依赖
- ✅ DFS 循环依赖检测
- ✅ 拓扑排序
- ✅ 获取模块依赖列表

**测试覆盖**: 5 个测试
- ✅ test_add_module
- ✅ test_detect_no_cycles
- ✅ test_detect_cycles
- ✅ test_topological_sort
- ✅ test_topological_sort_with_cycle

**API 示例**:
```rust
let mut graph = ModuleGraph::new();
graph.add_module("A.vue".to_string(), vec!["B.vue".to_string()]);
graph.add_module("B.vue".to_string(), vec!["C.vue".to_string()]);

// 检测循环依赖
if let Some(cycles) = graph.detect_cycles() {
    warn!("Circular dependencies: {:?}", cycles);
}

// 获取加载顺序
let sorted = graph.topological_sort()?;
```

---

### 4. HMRManager (热更新管理器)

**职责**: 管理 Vue 组件的热模块替换

**主要功能**:
- ✅ 文件修改检测（时间戳对比）
- ✅ Vue 组件重载补丁生成
- ✅ CSS 更新补丁生成
- ✅ 完整页面重载
- ✅ 补丁队列管理

**测试覆盖**: 6 个测试
- ✅ test_check_file_change
- ✅ test_generate_vue_reload_patch
- ✅ test_generate_css_update_patch
- ✅ test_generate_full_reload_patch
- ✅ test_pending_patches
- ✅ test_clear_timestamps

**补丁类型**:
```rust
pub enum PatchType {
    VueReload,      // Vue 组件重载
    CSSUpdate,      // CSS 更新
    FullReload,     // 完整页面重载
}
```

---

## 📊 依赖关系

### 共享核心层
- ✅ iris-core
- ✅ iris-gpu
- ✅ iris-layout
- ✅ iris-dom
- ✅ iris-sfc
- ✅ iris-cssom

### JetCrab 运行时
- ✅ iris-jetcrab

### 工具库
- ✅ tokio (异步运行时)
- ✅ notify (文件监听)
- ✅ walkdir (目录遍历)
- ✅ html5ever (HTML 解析)
- ✅ anyhow (错误处理)
- ✅ serde/serde_json (序列化)
- ✅ tracing (日志)

---

## 🎯 工作流程

### Vue 项目加载流程

```
1. 创建引擎实例
   JetCrabEngine::new()
   ↓
2. 初始化引擎
   engine.initialize()
   ├─ 初始化日志
   ├─ 初始化共享核心层
   └─ 初始化 JetCrab 运行时
   ↓
3. 加载 Vue 项目
   engine.load_project("/path/to/project")
   ├─ 扫描项目目录 (ProjectScanner)
   ├─ 解析 index.html
   └─ 构建模块依赖图 (ModuleGraph)
   ↓
4. 运行 Vue 应用
   engine.run()
   ├─ 创建 JetCrab 运行时
   ├─ 加载入口文件
   └─ 执行 JavaScript
   ↓
5. HMR 监听（可选）
   HMRManager::check_file_change()
   ├─ 检测文件修改
   └─ 生成热更新补丁
```

---

## ✅ 测试覆盖

### 测试统计

| 模块 | 测试数量 | 状态 |
|------|---------|------|
| module_graph | 5 个 | ✅ 全部通过 |
| hmr | 6 个 | ✅ 全部通过 |
| **总计** | **11 个** | **✅ 100% 通过** |

### 测试执行

```bash
cargo test -p iris-jetcrab-engine --lib

running 11 tests
test hmr::tests::test_check_file_change ... ok
test hmr::tests::test_clear_timestamps ... ok
test hmr::tests::test_generate_css_update_patch ... ok
test hmr::tests::test_generate_full_reload_patch ... ok
test hmr::tests::test_generate_vue_reload_patch ... ok
test hmr::tests::test_pending_patches ... ok
test module_graph::tests::test_add_module ... ok
test module_graph::tests::test_detect_cycles ... ok
test module_graph::tests::test_detect_no_cycles ... ok
test module_graph::tests::test_topological_sort ... ok
test module_graph::tests::test_topological_sort_with_cycle ... ok

test result: ok. 11 passed; 0 failed; 0 ignored
```

---

## 🚀 下一步工作

### Phase 2: 功能完善（预估 2 周）

| 任务 | 工作量 | 优先级 |
|------|--------|--------|
| 完善 index.html 解析（html5ever） | 8h | 🔴 |
| 实现 Vue SFC 依赖提取 | 6h | 🔴 |
| JetCrab WASM 导出接口 | 12h | 🔴 |
| 浏览器端 HMR 客户端 | 10h | 🔴 |
| 模块加载器增强 | 8h | 🟡 |

### Phase 3: 集成测试（预估 1 周）

| 任务 | 工作量 | 优先级 |
|------|--------|--------|
| 端到端集成测试 | 10h | 🔴 |
| 性能基准测试 | 6h | 🟡 |
| 文档完善 | 4h | 🟡 |

---

## 📝 关键设计决策

### 1. 模块依赖图使用 DFS

**原因**: 
- 简单高效
- 易于实现循环检测
- 支持拓扑排序

**备选方案**: 
- Kahn's 算法（BFS）
- Tarjan 算法（强连通分量）

### 2. HMR 使用时间戳检测

**原因**:
- 跨平台兼容性好
- 实现简单
- 性能开销小

**备选方案**:
- 文件内容哈希
- inotify（Linux 专用）

### 3. 项目扫描器独立模块

**原因**:
- 职责单一
- 易于测试
- 可复用

---

## 🎉 总结

`iris-jetcrab-engine` 模块已成功实现 Phase 1 的核心功能：

✅ **4 个核心模块**  
✅ **1,083 行代码**  
✅ **11 个测试全部通过**  
✅ **完整的 Vue 项目加载流程**  
✅ **HMR 热更新支持**  
✅ **循环依赖检测**  

为后续的 JetCrab WASM 导出和浏览器端集成打下了坚实的基础！

---

**文档创建日期**: 2026-04-28  
**模块状态**: ✅ Phase 1 完成  
**下一步**: JetCrab WASM 导出接口实现
