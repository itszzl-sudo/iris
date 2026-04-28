# 双运行时架构设计文档

> **创建日期**: 2026-04-28  
> **状态**: 实施中 (Phase 1)  
> **版本**: v0.1.0

---

## 📋 概述

Iris 项目采用**双运行时架构**，支持两种 JavaScript 执行引擎：

1. **现有方案（Boa Engine）**：轻量级、快速启动
2. **JetCrab 方案**：完整 npm 生态支持、生产环境就绪

两套方案**共享核心模块**，独立演进，互不干扰。

---

## 🎯 架构设计理念

### 核心理念：共享内核 + 独立运行时

```
┌─────────────────────────────────────────────────────┐
│              共享核心层（Shared Core）                  │
│                                                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐      │
│  │ iris-cssom│  │ iris-gpu │  │ iris-layout  │      │
│  └──────────┘  └──────────┘  └──────────────┘      │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐      │
│  │ iris-dom │  │ iris-sfc │  │ iris-core    │      │
│  └──────────┘  └──────────┘  └──────────────┘      │
└───────────────┬─────────────────┬───────────────────┘
                │                 │
                ▼                 ▼
┌──────────────────────┐  ┌──────────────────────┐
│  现有方案（Current）    │  │  JetCrab 方案        │
│                        │  │                       │
│  ┌──────────────┐    │  │  ┌──────────────┐    │
│  │  iris-js     │    │  │  │ iris-jetcrab │    │
│  │  (Boa Engine)│    │  │  │ (JetCrab)    │    │
│  └──────────────┘    │  │  └──────────────┘    │
│  ┌──────────────┐    │  │  ┌──────────────┐    │
│  │ iris-engine  │    │  │  │iris-jetcrab- │    │
│  │ (集成层)      │    │  │  │  engine      │    │
│  └──────────────┘    │  │  └──────────────┘    │
│  ┌──────────────┐    │  │  ┌──────────────┐    │
│  │ iris-cli     │    │  │  │iris-jetcrab- │    │
│  │ iris-app     │    │  │  │  cli         │    │
│  └──────────────┘    │  │  └──────────────┘    │
└──────────────────────┘  └──────────────────────┘
```

---

## 📦 Crate 架构

### 共享核心层（100% 复用）

| Crate | 功能 | 代码量 | 依赖 |
|-------|------|--------|------|
| **iris-core** | 基础工具、错误处理 | ~500 行 | - |
| **iris-cssom** | CSSOM API 完整实现 | ~1,700 行 | iris-core |
| **iris-gpu** | WebGPU 渲染引擎 | ~3,000 行 | wgpu, winit |
| **iris-layout** | Flexbox 布局引擎 | ~2,500 行 | iris-cssom |
| **iris-dom** | DOM/BOM 抽象层 | ~1,500 行 | iris-layout |
| **iris-sfc** | Vue SFC 编译器 | ~2,000 行 | swc, iris-cssom |

**总计**：~11,200 行核心代码，**完全共享**

---

### 现有方案（Boa Engine）

| Crate | 功能 | 代码量 | 状态 |
|-------|------|--------|------|
| **iris-js** | Boa Engine 集成 | ~1,500 行 | ✅ 完成 |
| **iris-engine** | 引擎集成层 | ~2,500 行 | ✅ 完成 |
| **iris-cli** | CLI 工具 | ~800 行 | ✅ 完成 |
| **iris-app** | 示例应用 | ~200 行 | ✅ 完成 |

**总计**：~5,000 行

---

### JetCrab 方案（新增）

| Crate | 功能 | 代码量 | 状态 |
|-------|------|--------|------|
| **iris-jetcrab** | JetCrab 运行时集成 | ~500 行 | 🚧 Phase 1 |
| **iris-jetcrab-engine** | JetCrab 引擎集成 | ~300 行 | ⏳ Phase 2 |
| **iris-jetcrab-cli** | JetCrab CLI 工具 | ~400 行 | ⏳ Phase 3 |

**总计**：~1,200 行（预估）

---

## 🔄 依赖关系

### 现有方案依赖链

```
iris-app
  └─ iris-cli
       └─ iris-engine
            ├─ iris-js (Boa Engine)
            ├─ iris-gpu
            ├─ iris-layout
            │   └─ iris-cssom
            ├─ iris-dom
            └─ iris-sfc
```

### JetCrab 方案依赖链

```
iris-jetcrab-app (未来)
  └─ iris-jetcrab-cli
       └─ iris-jetcrab-engine
            ├─ iris-jetcrab (JetCrab Runtime)
            ├─ iris-gpu (共享)
            ├─ iris-layout (共享)
            │   └─ iris-cssom (共享)
            ├─ iris-dom (共享)
            └─ iris-sfc (共享)
```

---

## 📊 技术栈对比

| 特性 | 现有方案（Boa） | JetCrab 方案 |
|------|----------------|-------------|
| **JS 引擎** | Boa Engine 0.20 | JetCrab (Chitin/QuickJS WASM) |
| **npm 包支持** | 手动实现 | CPM 原生支持 |
| **WASM 集成** | 需要手动配置 | 原生支持 |
| **包大小** | ~15MB | ~20MB（含 JetCrab） |
| **启动速度** | 快（<50ms） | 中等（<100ms） |
| **ES6+ 支持** | 90% | 95% |
| **异步 I/O** | Tokio | Tokio（共享） |
| **调试支持** | 基本 | 完整（Source Map） |
| **适用场景** | 轻量级、快速启动 | 完整 npm 生态、生产环境 |

---

## 🚀 实施路线图

### Phase 1: 基础设施（2 周）🚧

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 创建 `iris-jetcrab` crate | 4h | 🔴 | ✅ 完成 |
| 实现 JetCrab 运行时集成 | 12h | 🔴 | ⏳ 进行中 |
| 创建 `iris-jetcrab-engine` crate | 4h | 🔴 | ⏳ |
| 实现引擎集成层 | 10h | 🔴 | ⏳ |
| 创建 `iris-jetcrab-cli` crate | 4h | 🟡 | ⏳ |
| 实现 CLI 工具 | 8h | 🟡 | ⏳ |

**总计**：42 小时（约 1 周）

---

### Phase 2: 功能完善（2 周）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| ESM 模块加载 | 8h | 🔴 | ⏳ |
| CPM 包管理集成 | 10h | 🔴 | ⏳ |
| Web API 适配层 | 12h | 🔴 | ⏳ |
| WASM 桥接 | 8h | 🟡 | ⏳ |
| 测试覆盖 | 10h | 🔴 | ⏳ |

**总计**：48 小时（约 1.5 周）

---

### Phase 3: 优化与文档（1 周）

| 任务 | 工作量 | 优先级 | 状态 |
|------|--------|--------|------|
| 性能优化 | 8h | 🔴 | ⏳ |
| 错误处理 | 6h | 🟡 | ⏳ |
| 文档编写 | 10h | 🔴 | ⏳ |
| 示例项目 | 8h | 🟡 | ⏳ |

**总计**：32 小时（约 1 周）

---

## 💡 使用示例

### 方案 A：使用 Boa Engine（现有）

```toml
# Cargo.toml
[dependencies]
iris-engine = "0.1"
iris-cli = "0.1"
```

```bash
# 使用 CLI
iris-cli dev ./my-vue-app
```

---

### 方案 B：使用 JetCrab（新增）

```toml
# Cargo.toml
[dependencies]
iris-jetcrab-engine = "0.1"
iris-jetcrab-cli = "0.1"
```

```bash
# 使用 CLI
iris-jetcrab-cli dev ./my-vue-app
```

---

### 方案 C：混合使用（高级用户）

```toml
# Cargo.toml
[dependencies]
iris-engine = "0.1"           # 开发环境（快速启动）
iris-jetcrab-engine = "0.1"   # 生产环境（完整 npm 支持）
```

---

## 📈 代码复用率

| 模块 | 复用率 | 说明 |
|------|--------|------|
| iris-cssom | 100% | CSSOM API 完全共享 |
| iris-gpu | 100% | WebGPU 渲染完全共享 |
| iris-layout | 100% | 布局引擎完全共享 |
| iris-dom | 100% | DOM 抽象完全共享 |
| iris-sfc | 100% | SFC 编译器完全共享 |
| iris-engine | 90% | JetCrab 引擎复用 90% 代码 |
| iris-cli | 80% | CLI 工具复用 80% 代码 |

**总体代码复用率**：**~85%**

---

## ⚠️ 注意事项

### 循环依赖避免

```
✅ 正确依赖方向：
  iris-jetcrab → iris-cssom (OK)
  iris-jetcrab → iris-layout (OK)
  iris-jetcrab → iris-dom (OK)

❌ 禁止的依赖方向：
  iris-cssom → iris-jetcrab (禁止！会导致循环)
  iris-layout → iris-jetcrab (禁止！会导致循环)
```

### 命名空间隔离

```rust
// 现有方案
use iris_js::JsRuntime;
use iris_engine::RuntimeOrchestrator;

// JetCrab 方案
use iris_jetcrab::JetCrabRuntime;
use iris_jetcrab_engine::JetCrabOrchestrator;
```

---

## 🎯 优势分析

1. ✅ **代码复用率极高**：85%+ 核心代码共享
2. ✅ **用户选择灵活**：根据需求选择技术栈
3. ✅ **独立演进**：两套方案互不干扰
4. ✅ **市场竞争力**：覆盖更广的用户群体
5. ✅ **风险可控**：渐进式迁移，随时回滚

---

## 📝 更新日志

### v0.1.0 (2026-04-28)

- ✅ 创建双运行时架构设计文档
- ✅ 创建 `iris-jetcrab` crate
- 🚧 开始 Phase 1 实施

---

## 🔗 相关文档

- [CSSOM API 实现总结](./CSSOM_IMPLEMENTATION.md)
- [模块迁移总结](./CSS_MIGRATION.md)
- [项目 ROADMAP](../../ROADMAP.md)

---

**文档维护者**: Iris Team  
**最后更新**: 2026-04-28
