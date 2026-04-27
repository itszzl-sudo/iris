# Iris 架构文档

## 📊 当前架构依赖关系

### ✅ 无循环依赖（Phase 0 已完成）

```
iris-core (基础工具)
    ├─→ iris-gpu (WebGPU 渲染)
    │     └─→ iris-core
    │
    ├─→ iris-layout (布局引擎)
    │     └─→ iris-core
    │
    ├─→ iris-dom (DOM 抽象)
    │     ├─→ iris-core
    │     └─→ iris-layout
    │
    ├─→ iris-js (JS 运行时)
    │     ├─→ iris-core
    │     └─→ iris-dom
    │           └─→ iris-layout
    │
    └─→ iris-sfc (SFC 编译器)
          └─→ iris-js (可选)

顶层应用
    └─→ iris-engine / iris-app
          ├─→ iris-gpu
          ├─→ iris-layout
          ├─→ iris-dom
          ├─→ iris-js
          └─→ iris-sfc
```

### 依赖链

**单向依赖，无循环**：
```
iris-core → iris-layout → iris-dom → iris-js → iris-sfc
                ↓
            iris-gpu (独立)
```

---

## 🏗️ 模块职责

### iris-core
**职责**: 基础工具、事件循环抽象、窗口管理
**依赖**: 无（最底层）
**被依赖**: 所有其他模块

```rust
// 核心功能
- 事件循环抽象
- 窗口管理接口
- 通用工具函数
- 配置管理
```

### iris-gpu
**职责**: WebGPU 硬件渲染管线
**依赖**: iris-core
**独立性**: 独立于布局引擎，只接收渲染命令

```rust
// 核心功能
- 批渲染系统 (BatchRenderer)
- GPU 管线管理
- 字体图集 (FontAtlas)
- 纹理管理
- 动画插值
- 脏矩形优化
```

### iris-layout
**职责**: HTML/CSS 解析与布局计算
**依赖**: iris-core
**独立性**: 独立于渲染器，输出布局数据

```rust
// 核心功能
- HTML 解析 (html5ever)
- CSS 解析 (cssparser)
- 样式计算与继承
- 盒模型计算
- Flex 布局
- 选择器匹配
```

### iris-dom
**职责**: 跨平台 DOM/BOM 抽象与事件系统
**依赖**: iris-core, iris-layout
**职责边界**: 管理虚拟 DOM 树，不关心如何渲染

```rust
// 核心功能
- 虚拟 DOM (VNode)
- DOM 操作 API
- 事件系统 (EventDispatcher)
- BOM API 模拟 (Window, Document)
- 布局集成 (使用 iris-layout 计算布局)
```

### iris-js
**职责**: JavaScript 运行时（Boa Engine 集成）
**依赖**: iris-core, iris-dom
**职责边界**: 执行 JS 代码，注入 DOM API

```rust
// 核心功能
- Boa Engine 集成
- ESM 模块系统
- Vue 运行时注入
- DOM API 桥接
- TypeScript 编译
```

### iris-sfc
**职责**: Vue 单文件组件编译器
**依赖**: 无强制依赖（可选依赖 iris-js）
**职责边界**: 编译 .vue 文件为 JS 代码

```rust
// 核心功能
- SFC 解析（template/script/style）
- script setup 编译
- CSS Modules 作用域
- 模板指令编译
- 缓存与热重载
```

---

## 🔄 数据流

### 完整渲染流程

```
1. SFC 编译
   App.vue ──[iris-sfc]──→ JavaScript 代码

2. JS 执行
   JS 代码 ──[iris-js]──→ 创建 Vue 组件实例
                         → 生成虚拟 DOM

3. DOM 更新
   VNode 树 ──[iris-dom]──→ diff 算法
                           → 应用布局计算 (iris-layout)
                           → 生成渲染命令

4. GPU 渲染
   渲染命令 ──[iris-gpu]──→ 批处理
                          → GPU 绘制
                          → 显示帧
```

### 事件流

```
用户输入 (鼠标/键盘)
    ↓
winit 事件
    ↓
iris-core 事件循环
    ↓
iris-dom 事件分发
    ↓
iris-js 事件处理 (Vue 事件监听器)
    ↓
状态更新 → 重新渲染
```

---

## 📦 Cargo.toml 配置

### iris-layout
```toml
[dependencies]
iris-core.workspace = true
html5ever.workspace = true
markup5ever_rcdom.workspace = true
cssparser.workspace = true
# 注意：不依赖 iris-gpu
```

### iris-dom
```toml
[dependencies]
iris-core.workspace = true
iris-layout.workspace = true
# 注意：不直接依赖 iris-gpu
```

### iris-js
```toml
[dependencies]
iris-core.workspace = true
iris-dom.workspace = true
boa_engine = "0.20"
boa_gc = "0.20"
```

### iris-gpu
```toml
[dependencies]
iris-core.workspace = true
wgpu.workspace = true
winit.workspace = true
# 注意：不依赖 iris-layout 或 iris-dom
```

---

## ✅ Phase 0 重构完成清单

- [x] 移除 `iris-layout` 对 `iris-gpu` 的依赖
- [x] 确认代码层面无循环引用
- [x] 验证 `cargo check --workspace` 通过
- [x] 验证 `cargo tree` 无循环依赖
- [x] 创建架构文档（本文档）

---

## 🎯 下一步

### Phase 1: 增强 iris-layout
- 完善 CSS 选择器匹配
- 实现完整的 Flex 布局
- 添加 Grid 布局支持
- 优化样式计算性能

### Phase 2: 增强 iris-dom
- 完善虚拟 DOM diff 算法
- 优化事件系统性能
- 添加更多 BOM API

### Phase 3: 增强 iris-js
- 完善 Boa Engine 集成
- 优化 ESM 模块解析
- 添加 Vue 3 完整运行时

### Phase 4: 运行时集成
- 打通完整渲染链路
- 实现事件循环
- 性能优化

### Phase 5: 最小 Demo
- 创建计数器应用
- 验证完整流程
- 编写教程

---

## 📝 设计原则

1. **单向依赖**: 底层模块不依赖高层模块
2. **职责单一**: 每个模块只负责一个领域
3. **接口清晰**: 模块间通过明确的接口通信
4. **可测试性**: 每个模块可独立测试
5. **可扩展性**: 新功能通过添加模块而非修改现有模块实现

---

## 🔍 常见问题

### Q: 为什么 iris-gpu 不依赖 iris-layout？
A: 渲染器应该独立于布局引擎。iris-gpu 只接收渲染命令（坐标、颜色、纹理等），不关心这些数据如何计算。这样可以：
- 支持多种布局引擎（不仅是 iris-layout）
- 更容易测试
- 更容易替换或升级

### Q: iris-dom 为什么依赖 iris-layout？
A: DOM 元素需要布局信息（位置、尺寸）来确定如何渲染。iris-dom 使用 iris-layout 计算布局，但将渲染工作交给 iris-gpu。

### Q: 如何添加新的渲染特性？
A: 
1. 如果是布局相关 → 修改 iris-layout
2. 如果是渲染效果 → 修改 iris-gpu
3. 如果是 DOM API → 修改 iris-dom
4. 如果是 JS 功能 → 修改 iris-js

---

**最后更新**: 2026-04-24  
**Phase 0 状态**: ✅ 完成
