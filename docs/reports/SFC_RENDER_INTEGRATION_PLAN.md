# SFC 编译与渲染集成实现计划

## 📋 概述

本计划详细说明如何将 Iris Engine 的 SFC 编译器与完整的渲染管线（DOM → 布局 → GPU）集成，实现从 Vue SFC 文件到屏幕渲染的完整流程。

---

## 🎯 目标

实现完整的渲染流程：
```
Vue SFC (.vue)
  ↓
iris-sfc 编译 (template → render_fn, script, styles)
  ↓
JavaScript 执行 render_fn → 创建 VNode 树
  ↓
VNode → DOMNode (DOM 树构建)
  ↓
DOMNode → Layout (布局计算)
  ↓
Layout → GPU 渲染 (Batch Renderer)
  ↓
屏幕显示
```

---

## 📊 当前状态分析

### ✅ 已完成的功能

1. **SFC 编译器** (iris-sfc)
   - ✅ 模板解析（html5ever 集成）
   - ✅ Vue 指令支持（v-if, v-for, v-bind, v-on, v-model 等 14 个指令）
   - ✅ Render 函数生成（h() 函数格式）
   - ✅ TypeScript 编译（SWC 集成）
   - ✅ Script Setup 宏解析
   - ✅ CSS 处理（Scoped CSS, CSS Modules）
   - ✅ 缓存机制（LRU + 源码哈希）
   - ✅ 测试覆盖：19 个单元测试

2. **DOM 系统** (iris-layout/dom.rs, domtree.rs)
   - ✅ DOMNode 结构体
   - ✅ DOMTree 管理
   - ✅ DOM 操作 API（17 个方法）
   - ✅ Virtual DOM（VNode, Diff/Patch）
   - ✅ 事件系统
   - ✅ 测试覆盖：191 个测试

3. **布局引擎** (iris-layout)
   - ✅ Flexbox 布局
   - ✅ Block 布局
   - ✅ 绝对/固定定位
   - ✅ Grid 布局（部分）
   - ✅ 测试覆盖：完整

4. **GPU 渲染管线** (iris-gpu)
   - ✅ Batch Renderer
   - ✅ 顶点/索引缓冲区
   - ✅ 纹理系统
   - ✅ 字体渲染
   - ✅ 脏矩形优化
   - ✅ 测试覆盖：25 个测试

### ⚠️ 需要实现的集成

1. **Render 函数执行环境**
   - ❌ JavaScript 运行时中的 Vue 3 API（h, text, comment）
   - ❌ VNode 创建和管理的运行时支持

2. **VNode → DOMNode 转换**
   - ⚠️ 已有基础转换逻辑，需要完善
   - ❌ 样式计算和应用
   - ❌ 事件绑定

3. **DOMNode → Layout 集成**
   - ⚠️ 已有布局计算，需要与 DOM 树同步
   - ❌ 布局更新触发机制

4. **Layout → GPU 渲染**
   - ⚠️ 已有批渲染器，需要连接布局数据
   - ❌ 样式到渲染属性的映射
   - ❌ 纹理和字体资源管理

5. **端到端流程**
   - ❌ 完整的渲染循环
   - ❌ 数据更新 → 重新渲染
   - ❌ 响应式系统基础

---

## 🚀 实现步骤

### Phase A: JavaScript 运行时集成（预计 4-6 小时）

#### A.1 实现 Vue 3 运行时 API

**文件**: `crates/iris-js/src/vue_runtime.rs`

```rust
/// Vue 3 运行时 API 实现
pub struct VueRuntime {
    js_context: JsContext,
    vnode_registry: VNodeRegistry,
}

impl VueRuntime {
    /// 注册 h() 函数（创建 VNode）
    pub fn register_h_function(&mut self);
    
    /// 注册 text() 函数（创建文本 VNode）
    pub fn register_text_function(&mut self);
    
    /// 注册 comment() 函数（创建注释 VNode）
    pub fn register_comment_function(&mut self);
    
    /// 执行 render 函数并返回 VNode 树
    pub fn execute_render(&mut self, render_fn: &str) -> Result<VTree, JsError>;
}
```

**关键功能**:
- 在 JavaScript 环境中注册 `h()`, `text()`, `comment()` 函数
- 这些函数调用 Rust 端的 VNode 创建逻辑
- 将 JavaScript 返回值转换为 Rust VTree

#### A.2 VNode 注册表

**文件**: `crates/iris-dom/src/vnode_registry.rs`

```rust
/// 管理 JavaScript 创建的 VNode
pub struct VNodeRegistry {
    nodes: HashMap<u32, VNode>,
    next_id: u32,
}

impl VNodeRegistry {
    /// 创建元素 VNode 并返回 ID
    pub fn create_element(&mut self, tag: &str, props: Option<Value>, children: Vec<u32>) -> u32;
    
    /// 创建文本 VNode 并返回 ID
    pub fn create_text(&mut self, content: &str) -> u32;
    
    /// 根据 ID 获取 VNode
    pub fn get_node(&self, id: u32) -> Option<&VNode>;
    
    /// 获取根 VNode 并构建完整树
    pub fn build_tree(&mut self, root_id: u32) -> VTree;
}
```

---

### Phase B: VNode → DOMNode 转换（预计 3-4 小时）

#### B.1 完善 VTree 到 DOMNode 的转换

**文件**: `crates/iris-layout/src/vdom.rs`（已有，需扩展）

```rust
/// 将 VTree 转换为 DOMNode 树
pub fn vtree_to_dom(vtree: &VTree) -> DOMNode {
    match &vtree.root {
        VNode::Element { tag, props, children } => {
            let mut dom_node = DOMNode::new(NodeType::Element(tag.clone()));
            
            // 应用属性
            if let Some(props) = props {
                for (key, value) in props.iter() {
                    dom_node.set_attribute(key, &value.to_string());
                }
            }
            
            // 递归转换子节点
            for child_vnode in children {
                let child_dom = vtree_to_dom(child_vnode);
                dom_node.append_child(child_dom);
            }
            
            dom_node
        }
        VNode::Text { content } => {
            DOMNode::new_text(content)
        }
        VNode::Comment { content } => {
            DOMNode::new_comment(content)
        }
    }
}
```

#### B.2 样式计算和应用

**文件**: `crates/iris-layout/src/style.rs`（已有，需集成）

```rust
/// 为 DOM 节点计算和应用样式
pub fn apply_styles(dom_node: &mut DOMNode, css_rules: &[CssRule]) {
    // 1. 解析 CSS 规则
    // 2. 匹配选择器
    // 3. 计算优先级
   4. 应用样式到节点
}
```

---

### Phase C: DOM → Layout 集成（预计 3-4 小时）

#### C.1 布局触发机制

**文件**: `crates/iris-layout/src/layout.rs`

```rust
/// 执行布局计算
pub struct LayoutEngine {
    viewport_width: f32,
    viewport_height: f32,
}

impl LayoutEngine {
    /// 对整个 DOM 树执行布局计算
    pub fn compute_layout(&self, dom_tree: &DOMTree) -> LayoutTree {
        // 1. 从根节点开始递归计算
        // 2. 应用 Flexbox/Block 算法
        3. 返回布局结果
    }
    
    /// 增量布局更新（仅重新计算变化部分）
    pub fn update_layout(&self, dom_tree: &DOMTree, dirty_nodes: &[NodeId]) -> LayoutTree {
        // 优化：仅重新计算标记为脏的节点
    }
}
```

---

### Phase D: Layout → GPU 渲染（预计 4-5 小时）

#### D.1 布局数据到渲染命令

**文件**: `crates/iris-gpu/src/batch_renderer.rs`

```rust
/// 将布局树转换为渲染命令
impl BatchRenderer {
    /// 提交布局树进行渲染
    pub fn submit_layout(&mut self, layout_tree: &LayoutTree) {
        // 1. 遍历布局树
        for node in layout_tree.traverse() {
            // 2. 提取渲染属性
            let rect = node.layout_rect();
            let styles = node.computed_styles();
            
            // 3. 转换为渲染命令
            self.push_rect(rect, styles);
            self.push_text(rect, &styles);
        }
    }
    
    /// 执行渲染并提交到 GPU
    pub fn render(&mut self) -> Result<(), GpuError> {
        // 1. 编码命令
        // 2. 提交到 GPU
        // 3.  Present
    }
}
```

#### D.2 样式到渲染属性映射

**文件**: `crates/iris-gpu/src/style_mapper.rs`（新建）

```rust
/// 将 CSS 样式映射到 GPU 渲染属性
pub struct StyleMapper;

impl StyleMapper {
    /// 背景颜色
    pub fn map_background(styles: &ComputedStyle) -> Option<[f32; 4]>;
    
    /// 边框
    pub fn map_border(styles: &ComputedStyle) -> Option<BorderParams>;
    
    /// 文本样式
    pub fn map_text_styles(styles: &ComputedStyle) -> TextParams;
    
    /// 变换（transform）
    pub fn map_transform(styles: &ComputedStyle) -> Matrix4;
}
```

---

### Phase E: 完整渲染循环（预计 4-5 小时）

#### E.1 渲染循环实现

**文件**: `crates/iris-engine/src/render_loop.rs`（新建）

```rust
/// 主渲染循环
pub struct RenderLoop {
    vue_runtime: VueRuntime,
    dom_tree: DOMTree,
    layout_engine: LayoutEngine,
    gpu_renderer: BatchRenderer,
    running: bool,
}

impl RenderLoop {
    /// 启动渲染循环
    pub async fn run(&mut self, sfc_module: &SfcModule) -> Result<(), Error> {
        // 1. 执行 SFC script 初始化 Vue 应用
        self.vue_runtime.execute_script(&sfc_module.script)?;
        
        // 2. 执行 render 函数创建 VNode 树
        let vtree = self.vue_runtime.execute_render(&sfc_module.render_fn)?;
        
        // 3. 转换为 DOM 树
        self.dom_tree = self.build_dom_tree(&vtree);
        
        // 4. 应用样式
        self.apply_styles(&sfc_module.styles);
        
        // 5. 渲染循环
        while self.running {
            // 计算布局
            let layout = self.layout_engine.compute_layout(&self.dom_tree);
            
            // GPU 渲染
            self.gpu_renderer.submit_layout(&layout);
            self.gpu_renderer.render()?;
            
            // 处理事件
            self.process_events()?;
            
            // 等待下一帧
            self.wait_for_next_frame().await;
        }
        
        Ok(())
    }
}
```

---

### Phase F: 端到端集成测试（预计 3-4 小时）

#### F.1 完整流程测试

**文件**: `crates/iris-engine/tests/e2e_render_test.rs`

```rust
#[test]
fn test_sfc_to_screen() {
    // 1. 编译 SFC
    let sfc = compile("test_component.vue").unwrap();
    
    // 2. 创建渲染循环
    let mut loop = RenderLoop::new();
    
    // 3. 执行一次渲染
    loop.render_once(&sfc).unwrap();
    
    // 4. 验证渲染结果
    assert!(loop.gpu_renderer.has_commands());
}

#[test]
fn test_reactive_update() {
    // 1. 编译 SFC（包含响应式数据）
    let sfc = compile("reactive_component.vue").unwrap();
    
    // 2. 初始渲染
    let mut loop = RenderLoop::new();
    loop.render_once(&sfc).unwrap();
    let initial_state = loop.gpu_renderer.get_state();
    
    // 3. 触发数据更新
    loop.vue_runtime.set_data("count", 42);
    
    // 4. 重新渲染
    loop.render_once(&sfc).unwrap();
    let updated_state = loop.gpu_renderer.get_state();
    
    // 5. 验证状态变化
    assert_ne!(initial_state, updated_state);
}
```

---

### Phase G: 文档和示例（预计 2-3 小时）

#### G.1 创建完整的示例项目

**文件**: `examples/sfc-render-demo/`

```
sfc-render-demo/
├── App.vue              # 主组件
├── main.rs              # 入口
├── Cargo.toml
└── README.md            # 使用说明
```

#### G.2 编写集成文档

**文件**: `docs/SFC_RENDER_INTEGRATION.md`

内容包括：
- 架构概览
- 数据流说明
- API 文档
- 性能优化建议
- 故障排查指南

---

## 📈 预期成果

### 功能指标

- ✅ 完整的 SFC → 屏幕渲染流程
- ✅ 支持基础 Vue 3 组件渲染
- ✅ 响应式数据更新触发重新渲染
- ✅ 端到端集成测试覆盖
- ✅ 完整的文档和示例

### 性能指标

- SFC 编译时间：< 100ms（带缓存）
- 首次渲染时间：< 500ms
- 帧率：≥ 60 FPS
- 内存占用：< 100MB

---

## ⚠️ 风险和缓解措施

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| JavaScript 运行时集成复杂 | 高 | 中 | 分阶段实现，先支持基础 API |
| GPU 渲染性能不足 | 中 | 低 | 使用批渲染优化，延迟着色 |
| 布局计算性能瓶颈 | 中 | 中 | 实现增量布局，缓存布局结果 |
| 内存泄漏 | 高 | 低 | 严格的资源管理，定期测试 |

---

## 📅 时间估算

| 阶段 | 预计时间 | 优先级 |
|------|---------|--------|
| Phase A: JS 运行时集成 | 4-6 小时 | 🔴 高 |
| Phase B: VNode → DOM | 3-4 小时 | 🔴 高 |
| Phase C: DOM → Layout | 3-4 小时 | 🟡 中 |
| Phase D: Layout → GPU | 4-5 小时 | 🔴 高 |
| Phase E: 渲染循环 | 4-5 小时 | 🔴 高 |
| Phase F: 集成测试 | 3-4 小时 | 🟡 中 |
| Phase G: 文档示例 | 2-3 小时 | 🟢 低 |
| **总计** | **23-31 小时** | |

---

## 🎯 成功标准

1. **功能完整性**
   - [ ] 能编译并渲染简单的 Vue SFC 组件
   - [ ] 支持响应式数据更新
   - [ ] 支持事件处理

2. **测试覆盖**
   - [ ] ≥ 10 个端到端集成测试
   - [ ] 所有测试 100% 通过

3. **文档完整性**
   - [ ] 架构文档
   - [ ] API 文档
   - [ ] 使用示例

4. **性能达标**
   - [ ] 帧率 ≥ 60 FPS
   - [ ] 内存占用 < 100MB

---

## 🚀 下一步行动

1. ✅ 完成本计划文档
2. ⏳ 开始 Phase A: JavaScript 运行时集成
3. ⏳ 创建 `vue_runtime.rs` 模块
4. ⏳ 实现 `h()`, `text()`, `comment()` 函数注册
5. ⏳ 编写单元测试

---

**文档版本**: 1.0  
**创建日期**: 2026-04-27  
**最后更新**: 2026-04-27  
**状态**: 待执行
