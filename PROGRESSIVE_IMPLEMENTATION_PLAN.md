# Iris 渐进式实现计划

## 📊 当前架构依赖关系

```
iris-core (窗口/事件循环)
    ↓
iris-gpu (WebGPU 渲染)
    ↓
iris-layout (布局引擎)
    ↓
iris-dom (DOM 抽象)
    ↓
iris-js (JS 运行时)
    ↓
iris-sfc (SFC 编译器) ← 已完成 ✅
    ↓
iris-app (应用入口)
```

### 依赖循环问题 ⚠️

当前存在循环依赖：
- `iris-layout` → `iris-gpu`
- `iris-dom` → `iris-layout`
- `iris-js` → `iris-dom`

**需要重构为单向依赖**。

---

## 🎯 实现策略

### 原则
1. **自底向上**：从底层基础设施开始
2. **最小可用**：每个阶段都可独立测试
3. **渐进增强**：核心功能优先，特性后续添加
4. **测试驱动**：每个模块都有完整测试

### 阶段划分

| 阶段 | 模块 | 优先级 | 预计时间 | 状态 |
|------|------|--------|----------|------|
| Phase 0 | 架构重构 | 🔴 必须 | 1-2 天 | ⏳ 待开始 |
| Phase 1 | iris-layout 基础 | 🔴 必须 | 2-3 天 | ⏳ 待开始 |
| Phase 2 | iris-dom 核心 | 🔴 必须 | 2-3 天 | ⏳ 待开始 |
| Phase 3 | iris-js QuickJS | 🔴 必须 | 3-4 天 | ⏳ 待开始 |
| Phase 4 | 运行时集成 | 🔴 必须 | 2-3 天 | ⏳ 待开始 |
| Phase 5 | 最小 Demo | 🟡 重要 | 1-2 天 | ⏳ 待开始 |

---

## 📋 Phase 0: 架构重构

### 目标
消除循环依赖，建立清晰的模块边界。

### 新的依赖关系

```
iris-core (基础)
    ├─→ iris-gpu (渲染)
    ├─→ iris-layout (布局)
    │     └─→ 无依赖其他 Iris 模块
    ├─→ iris-dom (DOM)
    │     └─→ iris-layout (仅依赖布局计算)
    └─→ iris-js (JS 运行时)
          └─→ iris-dom (仅依赖 DOM API)
```

### 任务清单

- [ ] 修改 `iris-layout/Cargo.toml`
  - 移除 `iris-gpu` 依赖
  - 布局引擎应该独立于渲染器
  
- [ ] 修改 `iris-dom/Cargo.toml`
  - 保持依赖 `iris-layout`
  - DOM 需要布局信息
  
- [ ] 修改 `iris-js/Cargo.toml`
  - 保持依赖 `iris-dom`
  - JS 需要 DOM API

- [ ] 创建架构文档
  - 绘制新的依赖图
  - 说明各模块职责

---

## 📋 Phase 1: iris-layout 布局引擎

### 目标
实现基础的 HTML/CSS 解析和布局计算。

### 核心功能

#### 1.1 HTML 解析 ✅ 部分完成
- 使用 `html5ever` 解析 HTML
- 构建 DOM 树
- 支持基本标签

```rust
pub struct DOMNode {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<DOMNode>,
    pub text_content: Option<String>,
}
```

#### 1.2 CSS 解析
- 使用 `cssparser` 解析 CSS
- 构建样式规则表
- 支持基本选择器

```rust
pub struct CSSRule {
    pub selector: String,
    pub properties: HashMap<String, String>,
}
```

#### 1.3 样式计算
- 选择器匹配
- 样式继承
- 层叠规则

```rust
pub fn compute_styles(node: &DOMNode, rules: &[CSSRule]) -> ComputedStyles {
    // 匹配选择器 → 应用样式 → 处理继承
}
```

#### 1.4 布局计算
- 盒模型计算
- Flex 布局基础
- 节点尺寸和位置

```rust
pub struct LayoutBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

### 测试策略
- HTML 解析测试
- CSS 选择器匹配测试
- 布局计算测试
- 集成测试

---

## 📋 Phase 2: iris-dom DOM 抽象

### 目标
实现虚拟 DOM 和事件系统。

### 核心功能

#### 2.1 虚拟 DOM
```rust
pub struct VNode {
    pub id: u64,
    pub tag: String,
    pub props: HashMap<String, String>,
    pub children: Vec<VNode>,
    pub style: ComputedStyles,
    pub layout: LayoutBox,
}

impl VNode {
    pub fn create(tag: &str) -> Self { ... }
    pub fn append_child(&mut self, child: VNode) { ... }
    pub fn set_attribute(&mut self, key: &str, value: &str) { ... }
}
```

#### 2.2 事件系统
```rust
pub enum EventType {
    Click,
    MouseMove,
    KeyPress,
    Scroll,
    // ...
}

pub struct Event {
    pub event_type: EventType,
    pub target: u64,  // VNode ID
    pub data: EventData,
}

pub struct EventDispatcher {
    listeners: HashMap<u64, Vec<Box<dyn Fn(&Event)>>>,
}
```

#### 2.3 BOM API 模拟
```rust
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub document: Document,
}

pub struct Document {
    pub root: VNode,
}

impl Document {
    pub fn create_element(&self, tag: &str) -> VNode { ... }
    pub fn query_selector(&self, selector: &str) -> Option<&VNode> { ... }
}
```

### 测试策略
- VNode 操作测试
- 事件分发测试
- BOM API 测试

---

## 📋 Phase 3: iris-js QuickJS 集成

### 目标
集成 QuickJS 作为 JavaScript 执行引擎。

### 技术选型

**QuickJS Rust 绑定**：
- `quick-js`: 成熟稳定，文档完善
- `rquickjs`: 更现代，异步支持

**推荐**: `quick-js` (v0.4+)

### 核心功能

#### 3.1 QuickJS 运行时
```rust
use quick_js::{Context, JsValue};

pub struct JsRuntime {
    context: Context,
}

impl JsRuntime {
    pub fn new() -> Self {
        Self {
            context: Context::new(),
        }
    }
    
    pub fn eval(&self, code: &str) -> Result<JsValue, String> {
        self.context.eval(code).map_err(|e| e.to_string())
    }
    
    pub fn set_global(&self, name: &str, value: JsValue) {
        self.context.set_global(name, value).unwrap();
    }
}
```

#### 3.2 DOM API 注入
```rust
pub fn inject_dom_api(runtime: &mut JsRuntime, dom: &DomEnvironment) {
    // 注入 document
    runtime.set_global("document", JsValue::Object(dom.document()));
    
    // 注入 window
    runtime.set_global("window", JsValue::Object(dom.window()));
    
    // 注入 console
    runtime.set_global("console", JsValue::Object(console_api()));
}
```

#### 3.3 Vue 运行时预加载
```rust
pub fn inject_vue_runtime(runtime: &mut JsRuntime) {
    let vue_code = include_str!("../vendor/vue.runtime.esm.js");
    runtime.eval(vue_code).expect("Failed to load Vue runtime");
}
```

#### 3.4 ESM 模块系统
```rust
pub struct ModuleRegistry {
    modules: HashMap<String, JsValue>,
}

impl ModuleRegistry {
    pub fn register(&mut self, specifier: &str, module: JsValue) {
        self.modules.insert(specifier.to_string(), module);
    }
    
    pub fn resolve(&self, specifier: &str) -> Option<&JsValue> {
        self.modules.get(specifier)
    }
}
```

### 测试策略
- JS 执行测试
- DOM API 注入测试
- Vue 运行时加载测试
- 模块系统测试

---

## 📋 Phase 4: 运行时集成

### 目标
将所有模块连接起来，形成完整的 Vue 3 运行时。

### 集成流程

```
1. 初始化
   iris-core::init()
   → iris-gpu::init()
   → iris-layout::init()
   → iris-dom::init()
   → iris-js::init()

2. 编译 SFC
   iris-sfc::compile("App.vue")
   → 输出 JS 代码

3. 执行 JS
   iris-js::eval(js_code)
   → Vue 创建组件实例

4. 渲染循环
   loop {
     处理事件
     更新 DOM
     计算布局
     GPU 渲染
   }
```

### 核心集成点

#### 4.1 SFC → JS 运行时
```rust
pub fn run_sfc_component(sfc_path: &str) {
    // 编译 SFC
    let module = iris_sfc::compile_from_file(sfc_path)?;
    
    // 创建 JS 运行时
    let mut runtime = iris_js::JsRuntime::new();
    
    // 注入 DOM API
    let dom = iris_dom::DomEnvironment::new();
    runtime.inject_dom(&dom);
    
    // 加载 Vue 运行时
    runtime.load_vue_runtime();
    
    // 执行组件代码
    runtime.eval(&module.render_fn)?;
    
    // 启动渲染循环
    start_render_loop(&dom)?;
}
```

#### 4.2 DOM → GPU 渲染
```rust
pub fn render_dom_to_gpu(dom: &DomEnvironment, renderer: &mut BatchRenderer) {
    // 遍历 DOM 树
    for node in dom.traverse() {
        // 获取布局信息
        let layout = node.layout();
        
        // 转换为 GPU 绘制命令
        renderer.draw_rect(
            layout.x, layout.y,
            layout.width, layout.height,
            node.background_color(),
        );
        
        // 渲染文本
        if let Some(text) = node.text_content() {
            renderer.draw_text(text, layout);
        }
    }
}
```

#### 4.3 事件循环
```rust
pub fn event_loop(window: &Window, dom: &DomEnvironment) {
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // 更新 DOM
                dom.process_pending_updates();
                
                // 计算布局
                iris_layout::compute_layout(dom.root());
                
                // 渲染
                renderer.render_frame();
                
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                // 绘制下一帧
            }
            Event::WindowEvent { event, .. } => {
                // 分发到 DOM 事件系统
                dom.dispatch_event(&event);
            }
            _ => {}
        }
    });
}
```

---

## 📋 Phase 5: 最小可运行 Demo

### 目标
创建一个简单的 Vue 3 应用，验证完整流程。

### Demo 功能

**App.vue**:
```vue
<template>
  <div class="app">
    <h1>{{ message }}</h1>
    <button @click="count++">
      Count: {{ count }}
    </button>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const message = ref('Hello Iris!')
const count = ref(0)
</script>

<style>
.app {
  padding: 20px;
  font-family: sans-serif;
}
</style>
```

### 运行方式

```bash
cargo run --bin iris-app
```

### 预期效果
- 窗口显示 "Hello Iris!"
- 按钮显示 "Count: 0"
- 点击按钮，计数增加
- 样式正确应用

---

## 🔧 实施建议

### 1. 使用特性开关
```toml
# Cargo.toml
[features]
default = ["layout", "dom", "js"]
layout = []
dom = []
js = ["quick-js"]
```

### 2. 分步测试
每个阶段都有独立的测试：
```bash
cargo test -p iris-layout
cargo test -p iris-dom
cargo test -p iris-js
```

### 3. 文档同步
每个模块更新时同步更新文档。

### 4. Git 分支策略
```
main
├─ feature/layout-engine
├─ feature/dom-abstraction
├─ feature/js-runtime
└─ feature/runtime-integration
```

---

## ⏱️ 时间估算

| 阶段 | 预计时间 | 累计时间 |
|------|----------|----------|
| Phase 0 | 1-2 天 | 1-2 天 |
| Phase 1 | 2-3 天 | 3-5 天 |
| Phase 2 | 2-3 天 | 5-8 天 |
| Phase 3 | 3-4 天 | 8-12 天 |
| Phase 4 | 2-3 天 | 10-15 天 |
| Phase 5 | 1-2 天 | 11-17 天 |

**总计**: 约 2-3 周

---

## ✅ 成功标准

- [ ] 所有测试通过
- [ ] 无循环依赖
- [ ] Demo 可以运行
- [ ] 性能可接受（60 FPS）
- [ ] 内存使用合理
- [ ] 文档完整

---

## 📝 注意事项

1. **QuickJS 许可证**: LGPL，需注意合规性
2. **WebGPU 兼容性**: wgpu 版本锁定
3. **内存管理**: JS 垃圾回收 + Rust 所有权
4. **错误处理**: 跨语言错误传播
5. **性能优化**: 避免不必要的 DOM 重建

---

## 🚀 下一步

**立即开始**: Phase 0 - 架构重构
1. 修复循环依赖
2. 更新 Cargo.toml
3. 验证编译通过

然后逐步实现 Phase 1-5。
