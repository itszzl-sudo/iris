# 运行时API

<cite>
**本文档引用的文件**
- [lib.rs](file://crates/iris/src/lib.rs)
- [error_handling.rs](file://crates/iris/src/error_handling.rs)
- [dev_tools.rs](file://crates/iris/src/dev_tools.rs)
- [doc.txt](file://doc.txt)
- [todo.txt](file://todo.txt)
</cite>

## 更新摘要
**变更内容**
- 新增错误处理系统API文档，包括 ErrorBoundary、ErrorReporter、IrisError 等核心组件
- 新增调试工具系统API文档，包括 DevTools、PerformanceMetrics、ComponentInfo 等组件
- 更新架构概览图，反映新增的错误处理和调试功能层
- 添加错误边界捕获机制和调试工具使用示例

## 目录
1. [简介](#简介)
2. [项目结构](#项目结构)
3. [核心组件](#核心组件)
4. [架构概览](#架构概览)
5. [详细组件分析](#详细组件分析)
6. [错误处理系统API](#错误处理系统api)
7. [调试工具系统API](#调试工具系统api)
8. [依赖关系分析](#依赖关系分析)
9. [性能考虑](#性能考虑)
10. [故障排除指南](#故障排除指南)
11. [结论](#结论)

## 简介

Leivue Runtime是一个基于Rust和WebGPU的下一代无构建前端运行时引擎。该项目旨在提供一套完全脱离Node.js、浏览器DOM和编译打包的原生双端运行解决方案，支持零编译直接执行Vue3 + TypeScript，并完全兼容Element Plus、Ant Design Vue等第三方UI库。

该运行时引擎采用七层分层架构设计，其中JS沙箱运行时层是核心组件之一，负责提供独立隔离的JavaScript执行环境。新增的错误处理系统和调试工具系统为运行时提供了完整的错误隔离、恢复机制和开发时调试能力。

## 项目结构

根据项目文档，Leivue Runtime采用七层分层架构，每层都有明确的职责分工：

```mermaid
graph TB
subgraph "七层分层架构"
subgraph "上层应用层"
AppLayer[应用层]
end
subgraph "SFC/TS即时转译层"
TransLayer[转译层]
end
subgraph "JS沙箱运行时层"
JSLayer[JS沙箱层]
end
subgraph "跨端统一抽象层"
CrossLayer[跨端抽象层]
end
subgraph "浏览器级布局&样式引擎层"
LayoutLayer[布局样式层]
end
subgraph "WebGPU硬件渲染管线层"
GPULayer[WebGPU渲染层]
end
subgraph "Rust底层内核底座"
KernelLayer[Rust内核层]
end
end
```

**图表来源**
- [doc.txt:7-22](file://doc.txt#L7-L22)

**章节来源**
- [doc.txt:7-22](file://doc.txt#L7-L22)

## 核心组件

### JS沙箱运行时层

JS沙箱运行时层是Leivue Runtime的核心执行环境，具有以下关键特性：

- **JS引擎**：使用QuickJS（轻量高性能、Wasm友好、Rust深度绑定）
- **沙箱隔离**：与宿主环境完全隔离，提供安全的脚本执行环境
- **内置运行时**：预加载Vue3完整运行时（runtime-core/runtime-dom）
- **模块系统**：自研ESM解析器，支持import/export、第三方包引入

### 错误处理系统

错误处理系统提供组件级别的错误隔离和恢复机制：

- **ErrorBoundary**：捕获子组件渲染错误，防止错误传播
- **ErrorReporter**：收集和报告错误信息，支持错误统计和分析
- **IrisError**：统一的错误类型，包含消息、来源、严重级别等信息
- **错误恢复策略**：根据错误严重级别决定是否继续渲染

### 调试工具系统

调试工具系统提供开发时的调试和诊断能力：

- **DevTools**：综合调试工具，包含组件树检查、性能分析、错误诊断
- **PerformanceMetrics**：性能指标收集和分析
- **ComponentInfo**：组件信息跟踪和状态监控
- **实时调试**：支持帧计时、渲染性能监控、错误报告打印

### 核心定位

该层的核心使命是：
- 消灭前端工程化，突破浏览器沙箱限制
- 为Vue生态系统提供高性能跨端底座
- 支持零编译直接执行Vue3 + TypeScript
- 完全兼容Element Plus、Ant Design Vue等第三方UI库
- 提供完整的错误处理和调试能力

**章节来源**
- [doc.txt:46-51](file://doc.txt#L46-L51)
- [doc.txt:3-6](file://doc.txt#L3-L6)

## 架构概览

```mermaid
graph TD
subgraph "JS沙箱运行时层"
QuickJS[QuickJS引擎]
VueRuntime[Vue3运行时]
ModuleSystem[模块系统]
Sandbox[沙箱隔离]
end
subgraph "错误处理系统"
ErrorBoundary[错误边界]
ErrorReporter[错误报告器]
IrisError[IrisError类型]
end
subgraph "调试工具系统"
DevTools[调试工具]
PerformanceMetrics[性能指标]
ComponentInfo[组件信息]
end
subgraph "上层应用层"
App[Vue应用]
Components[组件]
Scripts[脚本]
end
subgraph "跨端统一抽象层"
Events[事件系统]
DOMSim[DOM模拟]
BOMSim[BOM模拟]
end
App --> QuickJS
Components --> QuickJS
Scripts --> QuickJS
QuickJS --> VueRuntime
QuickJS --> ModuleSystem
QuickJS --> Sandbox
QuickJS --> Events
QuickJS --> DOMSim
QuickJS --> BOMSim
QuickJS --> ErrorBoundary
QuickJS --> DevTools
ErrorBoundary --> ErrorReporter
DevTools --> PerformanceMetrics
DevTools --> ComponentInfo
```

**图表来源**
- [doc.txt:46-51](file://doc.txt#L46-L51)
- [doc.txt:41-45](file://doc.txt#L41-L45)

**章节来源**
- [doc.txt:41-51](file://doc.txt#L41-L51)

## 详细组件分析

### QuickJS引擎API

QuickJS引擎作为JS沙箱运行时层的核心，提供了以下主要功能：

#### 初始化流程
```mermaid
sequenceDiagram
participant App as 应用程序
participant Engine as QuickJS引擎
participant Runtime as Vue运行时
participant Modules as 模块系统
App->>Engine : 创建JS引擎实例
Engine->>Runtime : 加载Vue3运行时
Engine->>Modules : 初始化模块解析器
Engine->>Engine : 设置沙箱隔离
Engine-->>App : 返回可执行的JS环境
```

**图表来源**
- [doc.txt:46-51](file://doc.txt#L46-L51)

#### 主要API接口

1. **引擎初始化**
   - 功能：创建独立的JS执行环境
   - 参数：无
   - 返回值：JS引擎实例
   - 使用场景：应用启动时的环境准备

2. **脚本执行**
   - 功能：在沙箱环境中执行JavaScript代码
   - 参数：JavaScript源码字符串
   - 返回值：执行结果或错误信息
   - 使用场景：动态代码执行、插件加载

3. **模块加载**
   - 功能：解析和加载ESM模块
   - 参数：模块路径、导入规范
   - 返回值：模块导出对象
   - 使用场景：第三方库集成、组件模块化

**章节来源**
- [doc.txt:46-51](file://doc.txt#L46-L51)

### Vue运行时API

Vue运行时层提供了完整的Vue3运行时能力：

#### 生命周期管理
```mermaid
flowchart TD
Start([组件创建]) --> Setup[setup执行]
Setup --> Mount[挂载阶段]
Mount --> Mounted[mounted回调]
Mounted --> Update[数据更新]
Update --> Updated[updated回调]
Updated --> Unmount[卸载阶段]
Unmount --> Unmounted[unmounted回调]
Unmounted --> End([组件销毁])
```

**图表来源**
- [doc.txt:72-73](file://doc.txt#L72-L73)

#### 核心API接口

1. **组件生命周期**
   - `onMounted`: 组件挂载后执行
   - `onUpdated`: 组件更新后执行  
   - `onUnmounted`: 组件卸载前执行
   - `onBeforeMount`: 组件挂载前执行
   - `onBeforeUpdate`: 组件更新前执行
   - `onBeforeUnmount`: 组件卸载前执行

2. **响应式系统**
   - `ref`: 创建响应式引用
   - `reactive`: 创建响应式对象
   - `computed`: 创建计算属性
   - `watch`: 监听数据变化

3. **组合式API**
   - `defineComponent`: 定义组件
   - `useContext`: 获取组件上下文
   - `getCurrentInstance`: 获取当前实例

**章节来源**
- [doc.txt:72-73](file://doc.txt#L72-L73)

### 模块系统API

自研ESM解析器提供了完整的模块加载能力：

#### 模块加载流程
```mermaid
flowchart TD
Import[import语句] --> Parse[解析模块路径]
Parse --> Resolve[解析相对/绝对路径]
Resolve --> Load[加载模块文件]
Load --> Transform[转换模块代码]
Transform --> Cache[缓存模块]
Cache --> Export[返回模块导出]
Export --> Use[使用模块]
Use --> Import
```

**图表来源**
- [doc.txt:50](file://doc.txt#L50)

#### 核心API接口

1. **模块导入**
   - 功能：动态导入ESM模块
   - 参数：模块路径、导入选项
   - 返回值：Promise包含模块内容
   - 使用场景：按需加载、懒加载

2. **模块导出**
   - 功能：定义模块导出内容
   - 参数：导出名称、导出值
   - 返回值：void
   - 使用场景：组件库开发、工具函数封装

3. **路径解析**
   - 功能：解析模块路径
   - 参数：相对路径、基准路径
   - 返回值：绝对路径
   - 使用场景：内部模块组织、第三方库集成

**章节来源**
- [doc.txt:50](file://doc.txt#L50)

### 事件处理机制

跨端统一抽象层提供了完整的事件处理能力：

#### 事件处理流程
```mermaid
sequenceDiagram
participant User as 用户
participant EventSys as 事件系统
participant DOMSim as DOM模拟
participant Handler as 事件处理器
User->>EventSys : 用户交互事件
EventSys->>DOMSim : 模拟DOM事件
DOMSim->>Handler : 调用事件处理器
Handler->>Handler : 执行用户代码
Handler->>EventSys : 返回处理结果
EventSys->>User : 更新界面状态
```

**图表来源**
- [doc.txt:42-44](file://doc.txt#L42-L44)

#### 核心事件类型

1. **鼠标事件**
   - click: 点击事件
   - mousemove: 鼠标移动
   - mousedown: 鼠标按下
   - mouseup: 鼠标抬起

2. **键盘事件**
   - keydown: 键盘按下
   - keyup: 键盘抬起
   - keypress: 键盘输入

3. **触摸事件**
   - touchstart: 触摸开始
   - touchmove: 触摸移动
   - touchend: 触摸结束

**章节来源**
- [doc.txt:42-44](file://doc.txt#L42-L44)

## 错误处理系统API

### ErrorBoundary（错误边界）

ErrorBoundary提供组件级别的错误隔离，防止错误传播到整个应用。

#### 核心功能
- 捕获子组件渲染错误
- 根据错误严重级别决定渲染策略
- 提供备用内容显示
- 维护错误历史记录

#### 主要API接口

1. **创建错误边界**
   ```rust
   let boundary = ErrorBoundary::new("MyComponent");
   ```

2. **设置备用内容**
   ```rust
   let boundary = ErrorBoundary::new("MyComponent")
       .with_fallback("<div>组件加载失败</div>");
   ```

3. **捕获错误**
   ```rust
   let error = IrisError::new("渲染失败", ErrorSource::Render, ErrorSeverity::Error);
   boundary.catch_error(error);
   ```

4. **错误恢复**
   ```rust
   match boundary.recover() {
       Ok(()) => println!("恢复成功"),
       Err(e) => println!("恢复失败: {:?}", e),
   }
   ```

5. **状态查询**
   ```rust
   println!("是否继续渲染: {}", boundary.should_continue_rendering());
   println!("错误数量: {}", boundary.error_count());
   ```

**章节来源**
- [error_handling.rs:144-273](file://crates/iris/src/error_handling.rs#L144-L273)

### ErrorReporter（错误报告器）

ErrorReporter负责收集和报告错误信息，提供错误统计和分析功能。

#### 核心功能
- 收集错误历史
- 按严重级别和来源分类
- 生成错误报告
- 控制错误输出

#### 主要API接口

1. **创建错误报告器**
   ```rust
   let mut reporter = ErrorReporter::new();
   ```

2. **报告错误**
   ```rust
   let error = IrisError::new("网络请求失败", ErrorSource::Network, ErrorSeverity::Error);
   reporter.report(error);
   ```

3. **查询错误**
   ```rust
   let errors = reporter.errors();
   let warning_count = reporter.errors_by_severity(ErrorSeverity::Warning).len();
   let render_errors = reporter.errors_by_source(&ErrorSource::Render);
   ```

4. **生成报告**
   ```rust
   let report = reporter.generate_report();
   println!("{}", report);
   ```

5. **管理状态**
   ```rust
   reporter.set_enabled(false); // 禁用报告
   reporter.clear(); // 清空错误历史
   ```

**章节来源**
- [error_handling.rs:275-380](file://crates/iris/src/error_handling.rs#L275-L380)

### IrisError（统一错误类型）

IrisError是运行时统一的错误类型，提供丰富的错误信息。

#### 错误属性
- `message`: 错误消息
- `source`: 错误来源
- `severity`: 严重级别
- `component_path`: 组件路径
- `source_error`: 原始错误
- `timestamp`: 时间戳

#### 错误严重级别
- `Warning`: 警告 - 不影响渲染
- `Error`: 错误 - 组件渲染失败
- `Fatal`: 致命 - 整个应用崩溃

#### 错误来源
- `Render`: 渲染错误
- `Layout`: 布局错误
- `Style`: 样式错误
- `Script`: JavaScript错误
- `Network`: 网络错误
- `Unknown`: 未知错误

**章节来源**
- [error_handling.rs:63-131](file://crates/iris/src/error_handling.rs#L63-L131)

## 调试工具系统API

### DevTools（调试工具）

DevTools提供开发时的综合调试能力，包含组件树检查、性能分析和错误诊断。

#### 核心功能
- 组件树注册和跟踪
- 性能指标收集
- 错误报告和诊断
- 实时调试信息输出

#### 主要API接口

1. **创建调试工具**
   ```rust
   let mut devtools = DevTools::new();
   devtools.set_enabled(true); // 启用调试
   devtools.set_profiling(true); // 启用性能分析
   ```

2. **组件树管理**
   ```rust
   // 注册组件
   devtools.register_component("App", "App");
   devtools.register_component("Header", "App/Header");
   
   // 更新组件信息
   devtools.update_component("App", |info| {
       info.children_count = 5;
       info.render_time = Some(Duration::from_millis(10));
   });
   
   // 获取组件信息
   let component = devtools.get_component("App");
   ```

3. **性能分析**
   ```rust
   // 开始帧分析
   devtools.begin_frame();
   
   // 开始渲染计时
   devtools.begin_render();
   // 执行渲染逻辑
   devtools.end_render();
   
   // 结束帧分析
   devtools.end_frame();
   
   // 获取性能指标
   let metrics = devtools.metrics();
   ```

4. **错误调试**
   ```rust
   // 报告错误
   let error = IrisError::new("渲染失败", ErrorSource::Render, ErrorSeverity::Error);
   devtools.report_error(error);
   
   // 打印错误报告
   devtools.print_error_report();
   ```

5. **综合调试**
   ```rust
   // 打印完整调试信息
   devtools.print_debug_info();
   
   // 重置状态
   devtools.reset();
   ```

**章节来源**
- [dev_tools.rs:106-369](file://crates/iris/src/dev_tools.rs#L106-L369)

### PerformanceMetrics（性能指标）

PerformanceMetrics用于收集和分析渲染性能指标。

#### 性能指标
- `total_render_time`: 总渲染时间
- `layout_time`: 布局计算时间
- `gpu_time`: GPU渲染时间
- `fps`: 帧率（FPS）
- `frame_time_ms`: 帧时间（毫秒）
- `memory_usage_kb`: 内存使用（KB）

#### 主要API接口

1. **创建性能指标**
   ```rust
   let mut metrics = PerformanceMetrics::new();
   ```

2. **计算FPS**
   ```rust
   metrics.calculate_fps(frame_count, elapsed_duration);
   ```

3. **格式化输出**
   ```rust
   let formatted = metrics.format();
   println!("{}", formatted);
   ```

**章节来源**
- [dev_tools.rs:42-98](file://crates/iris/src/dev_tools.rs#L42-L98)

### ComponentInfo（组件信息）

ComponentInfo用于跟踪组件的状态和性能信息。

#### 组件属性
- `name`: 组件名称
- `path`: 组件路径
- `children_count`: 子组件数量
- `render_time`: 渲染时间
- `has_error`: 是否有错误

**章节来源**
- [dev_tools.rs:14-40](file://crates/iris/src/dev_tools.rs#L14-L40)

## 依赖关系分析

```mermaid
graph LR
subgraph "外部依赖"
QuickJS[QuickJS引擎]
Vue3[Vue3框架]
WebGPU[WebGPU API]
Rust[Rust标准库]
end
subgraph "内部模块"
JSEnv[JS环境]
ModuleMgr[模块管理器]
EventMgr[事件管理器]
RenderEngine[渲染引擎]
ErrorHandling[错误处理系统]
DevTools[调试工具系统]
end
QuickJS --> JSEnv
Vue3 --> JSEnv
WebGPU --> RenderEngine
Rust --> JSEnv
Rust --> ModuleMgr
Rust --> EventMgr
Rust --> ErrorHandling
Rust --> DevTools
JSEnv --> ModuleMgr
JSEnv --> EventMgr
JSEnv --> RenderEngine
JSEnv --> ErrorHandling
JSEnv --> DevTools
ErrorHandling --> DevTools
```

**图表来源**
- [doc.txt:23-29](file://doc.txt#L23-L29)
- [doc.txt:46-51](file://doc.txt#L46-L51)

**章节来源**
- [doc.txt:23-29](file://doc.txt#L23-L29)
- [doc.txt:46-51](file://doc.txt#L46-L51)

## 性能考虑

### 内存管理
- Rust内存安全保证，无GC停顿
- 内存池优化，减少频繁分配
- 模块缓存机制，避免重复加载
- 错误历史限制，防止内存泄漏

### 执行效率
- QuickJS高性能JS引擎
- WebGPU硬件加速渲染
- 零编译直接执行，消除构建开销
- 调试工具可选择性启用，避免生产环境性能损失

### 资源优化
- 体积极小（MB级别）
- 启动速度快
- CPU开销低
- 错误报告器支持错误数量限制

## 故障排除指南

### 常见问题

1. **模块加载失败**
   - 检查模块路径是否正确
   - 确认模块解析器配置
   - 验证文件权限设置

2. **Vue组件无法渲染**
   - 检查组件生命周期钩子
   - 验证响应式数据更新
   - 确认模板语法正确性

3. **事件处理异常**
   - 检查事件绑定方式
   - 验证事件处理器签名
   - 确认DOM模拟状态

4. **错误边界失效**
   - 检查错误边界配置
   - 验证错误严重级别设置
   - 确认备用内容正确性

5. **调试工具无输出**
   - 检查调试工具启用状态
   - 验证性能分析开关
   - 确认错误报告器状态

### 调试建议

- 使用浏览器开发者工具检查JS执行
- 启用详细日志输出
- 分模块测试功能完整性
- 验证跨端兼容性
- 使用DevTools.print_debug_info()获取完整调试信息
- 监控ErrorReporter的错误统计

## 结论

Leivue Runtime的JS沙箱运行时层通过QuickJS引擎、Vue3运行时和自研模块系统的有机结合，为现代Web应用提供了全新的执行环境。新增的错误处理系统和调试工具系统进一步增强了运行时的可靠性和开发体验。

该架构具有以下优势：

- **安全性**：完全隔离的沙箱环境，防止恶意代码执行
- **可靠性**：组件级别的错误隔离和恢复机制
- **可观测性**：完整的调试工具和性能分析能力
- **性能**：零编译直接执行，WebGPU硬件加速
- **兼容性**：完全兼容Vue3生态系统和第三方UI库
- **跨平台**：统一的跨端抽象，支持浏览器和桌面原生运行

随着错误处理系统和调试工具系统的完善，这套运行时API将为Vue生态系统的现代化提供强有力的技术支撑，推动前端开发模式向更高效、更安全、更可观测的方向演进。