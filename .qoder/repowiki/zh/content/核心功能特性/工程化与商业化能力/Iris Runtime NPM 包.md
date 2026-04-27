# Iris Runtime NPM 包

<cite>
**本文档引用的文件**
- [package.json](file://iris-runtime/package.json)
- [iris-runtime.js](file://iris-runtime/bin/iris-runtime.js)
- [install.js](file://iris-runtime/scripts/install.js)
- [prepare-binaries.js](file://iris-runtime/scripts/prepare-binaries.js)
- [README.md](file://iris-runtime/README.md)
- [Cargo.toml](file://Cargo.toml)
- [Cargo.toml](file://crates/iris-cli/Cargo.toml)
- [Cargo.toml](file://crates/iris-engine/Cargo.toml)
- [Cargo.toml](file://crates/iris-gpu/Cargo.toml)
- [main.rs](file://crates/iris-cli/src/main.rs)
- [lib.rs](file://crates/iris-engine/src/lib.rs)
- [orchestrator.rs](file://crates/iris-engine/src/orchestrator.rs)
- [lib.rs](file://crates/iris-core/src/lib.rs)
- [lib.rs](file://crates/iris-gpu/src/lib.rs)
- [lib.rs](file://crates/iris-layout/src/lib.rs)
- [lib.rs](file://crates/iris-dom/src/lib.rs)
- [lib.rs](file://crates/iris-sfc/src/lib.rs)
</cite>

## 更新摘要
**变更内容**
- 完全重构安装机制：从网络下载改为本地打包分发
- 新增prepare-binaries脚本，专门用于维护者构建多平台二进制
- 移除网络下载依赖，实现真正的离线安装
- 强化"无需编译、无需Rust"的用户体验
- 简化安装流程，提升用户友好度

## 目录
1. [简介](#简介)
2. [项目结构](#项目结构)
3. [核心组件](#核心组件)
4. [架构总览](#架构总览)
5. [详细组件分析](#详细组件分析)
6. [依赖关系分析](#依赖关系分析)
7. [性能考虑](#性能考虑)
8. [故障排除指南](#故障排除指南)
9. [结论](#结论)

## 简介
Iris Runtime 是一个由 Rust + WebGPU 驱动的 Vue 3 开发服务器 NPM 包。经过重大架构升级，该包现已完全迁移到本地打包分发模式，无需用户安装 Rust 工具链即可使用。通过 Node.js 包装器调用预编译的 iris-cli 二进制文件，提供开发服务器、生产构建以及运行时信息查询功能。该包的核心价值在于将高性能的 Rust/WebGPU 渲染后端与 Vue 3 的即时编译能力结合，为开发者提供快速热重载和 GPU 加速的开发体验。

**更新** 安装机制已完全重构，采用本地打包分发模式，用户无需任何网络访问或编译操作。

## 项目结构
Iris Runtime 采用 monorepo 结构，包含一个 NPM 包和多个 Rust crate。经过架构升级，现在主要依赖预编译的二进制文件：

```mermaid
graph TB
subgraph "NPM 包层"
A[iris-runtime 包]
A1[bin/iris-runtime.js]
A2[scripts/install.js]
A3[scripts/prepare-binaries.js]
A4[package.json]
end
subgraph "预编译二进制层"
B[预编译二进制文件]
B1[iris-runtime.exe (Windows)]
B2[iris-runtime (macOS Intel)]
B3[iris-runtime-aarch64 (macOS ARM64)]
B4[iris-runtime-x86_64 (Linux)]
end
subgraph "Rust 工作区"
C[crates/]
subgraph "CLI 层"
C1[iris-cli]
C1a[src/main.rs]
end
subgraph "引擎层"
C2[iris-engine]
C2a[src/lib.rs]
C2b[src/orchestrator.rs]
end
subgraph "核心层"
C3[iris-core]
C3a[src/lib.rs]
end
subgraph "渲染层"
C4[iris-gpu]
C4a[src/lib.rs]
end
subgraph "布局层"
C5[iris-layout]
C5a[src/lib.rs]
end
subgraph "DOM 层"
C6[iris-dom]
C6a[src/lib.rs]
end
subgraph "编译层"
C7[iris-sfc]
C7a[src/lib.rs]
end
end
A --> B
A --> C1
B --> C1
C1 --> C2
C2 --> C3
C2 --> C4
C2 --> C5
C2 --> C6
C2 --> C7
```

**图表来源**
- [Cargo.toml:1-32](file://Cargo.toml#L1-L32)
- [iris-runtime.js:1-131](file://iris-runtime/bin/iris-runtime.js#L1-L131)
- [install.js:1-94](file://iris-runtime/scripts/install.js#L1-L94)
- [prepare-binaries.js:1-146](file://iris-runtime/scripts/prepare-binaries.js#L1-L146)
- [main.rs:1-96](file://crates/iris-cli/src/main.rs#L1-L96)

**章节来源**
- [package.json:1-60](file://iris-runtime/package.json#L1-L60)
- [README.md:1-164](file://iris-runtime/README.md#L1-L164)
- [Cargo.toml:1-32](file://Cargo.toml#L1-L32)

## 核心组件
Iris Runtime 由以下核心组件构成，现已完全基于预编译二进制：

### NPM 包管理层
- **iris-runtime 包**: 提供用户友好的 CLI 接口，无需 Rust 工具链
- **安装脚本**: 从包中复制预编译二进制文件，无需网络下载
- **维护者脚本**: 专门用于构建和准备多平台二进制文件
- **包装器脚本**: 将 Node.js 命令转发给预编译二进制

### 预编译二进制层
- **iris-runtime 二进制**: 针对不同平台的预编译可执行文件
- **本地打包分发**: 所有二进制文件预先构建并包含在包中
- **自动检测机制**: 智能识别用户平台并选择对应二进制文件

### Rust CLI 层
- **iris-cli**: 主要的命令行工具，支持 dev/build/info 子命令
- **命令解析**: 使用 clap 库处理命令行参数和子命令
- **日志系统**: 集成 tracing 和 colored 日志输出

### 引擎编排层
- **RuntimeOrchestrator**: 协调各个引擎模块的初始化和交互
- **模块注册**: 管理 JavaScript 模块的注册和执行
- **生命周期管理**: 控制整个运行时的启动、运行和关闭过程

### 核心渲染层
- **WebGPU 渲染器**: 基于 wgpu 的硬件加速渲染
- **批渲染系统**: 优化图形绘制性能
- **纹理缓存**: 管理图像资源的缓存和复用

### 布局和 DOM 层
- **浏览器级布局**: 实现 Flexbox、Grid、表格等布局算法
- **虚拟 DOM**: 轻量级 DOM 抽象，支持事件系统
- **BOM API**: 提供 Window、Document 等浏览器 API 的模拟

### 编译层
- **SFC 编译器**: 即时编译 Vue 单文件组件
- **TypeScript 转译**: 支持 TypeScript 到 JavaScript 的转换
- **样式处理器**: 支持 CSS Modules、Scoped CSS、SCSS/Less

**更新** 安装流程已完全自动化，用户不再需要手动编译 Rust 代码或进行网络下载。

**章节来源**
- [iris-runtime.js:1-131](file://iris-runtime/bin/iris-runtime.js#L1-L131)
- [install.js:1-94](file://iris-runtime/scripts/install.js#L1-L94)
- [prepare-binaries.js:1-146](file://iris-runtime/scripts/prepare-binaries.js#L1-L146)
- [main.rs:1-96](file://crates/iris-cli/src/main.rs#L1-L96)
- [lib.rs:1-109](file://crates/iris-engine/src/lib.rs#L1-L109)

## 架构总览
Iris Runtime 采用分层架构设计，现已完全基于预编译二进制：

```mermaid
graph TB
subgraph "用户界面"
U[Vue 应用程序]
end
subgraph "编译层"
SFC[Iris SFC<br/>单文件组件编译器]
TS[TypeScript 转译器]
CSS[样式处理器]
end
subgraph "运行时层"
JS[Iris JS<br/>JavaScript 运行时]
DOM[Iris DOM<br/>DOM/BOM 抽象]
LAYOUT[Iris Layout<br/>布局引擎]
end
subgraph "渲染层"
GPU[Iris GPU<br/>WebGPU 渲染器]
BATCH[批渲染系统]
TEXT[文本渲染]
end
subgraph "核心层"
CORE[Iris Core<br/>异步运行时]
WIN[窗口管理]
IO[文件 IO]
end
subgraph "预编译二进制层"
BIN[预编译二进制]
LOCAL[本地打包分发]
PREPARE[prepare-binaries.js]
end
U --> SFC
SFC --> JS
JS --> DOM
DOM --> LAYOUT
LAYOUT --> GPU
GPU --> CORE
CORE --> WIN
CORE --> IO
BIN --> LOCAL
LOCAL --> PREPARE
```

**图表来源**
- [lib.rs:1-109](file://crates/iris-engine/src/lib.rs#L1-L109)
- [lib.rs:1-167](file://crates/iris-core/src/lib.rs#L1-L167)
- [lib.rs:1-200](file://crates/iris-gpu/src/lib.rs#L1-L200)
- [install.js:46-94](file://iris-runtime/scripts/install.js#L46-L94)
- [prepare-binaries.js:92-146](file://iris-runtime/scripts/prepare-binaries.js#L92-L146)

## 详细组件分析

### 本地打包分发安装组件
安装脚本现在负责从包中复制预编译二进制文件，无需网络下载：

```mermaid
sequenceDiagram
participant User as 用户
participant NPM as NPM 安装
participant Install as install.js
participant Package as NPM 包
participant Binary as 预编译二进制
User->>NPM : npm install iris-runtime
NPM->>Install : postinstall 脚本
Install->>Install : 检测平台和架构
Install->>Package : 从包中复制二进制
Package-->>Install : 返回二进制文件
Install->>Install : 设置执行权限
Install->>Binary : 验证二进制完整性
Binary-->>Install : 返回验证结果
Install-->>NPM : 安装完成
NPM-->>User : 可以使用 iris-runtime
```

**图表来源**
- [install.js:46-94](file://iris-runtime/scripts/install.js#L46-L94)

**更新** 安装流程现在完全离线，无需任何网络访问。

**章节来源**
- [install.js:1-94](file://iris-runtime/scripts/install.js#L1-L94)

### 维护者二进制构建组件
prepare-binaries脚本专门用于维护者构建多平台二进制文件：

```mermaid
sequenceDiagram
participant Maintainer as 维护者
participant Prep as prepare-binaries.js
participant Rust as Rust 工具链
participant Cargo as Cargo
participant Targets as 目标平台
Maintainer->>Prep : npm run prepare-binaries
Prep->>Prep : 检查 Rust 工具链
Prep->>Rust : 添加交叉编译目标
Rust-->>Prep : 返回目标状态
Prep->>Cargo : 为每个目标构建二进制
Cargo-->>Prep : 返回构建结果
Prep->>Targets : 复制二进制到 binaries/
Targets-->>Prep : 返回复制结果
Prep-->>Maintainer : 生成构建摘要
```

**图表来源**
- [prepare-binaries.js:92-146](file://iris-runtime/scripts/prepare-binaries.js#L92-L146)

**更新** 新增专门的维护者脚本，用于构建和准备多平台二进制文件。

**章节来源**
- [prepare-binaries.js:1-146](file://iris-runtime/scripts/prepare-binaries.js#L1-L146)

### CLI 包装器组件
CLI 包装器负责将 Node.js 命令转发给预编译的二进制文件：

```mermaid
sequenceDiagram
participant User as 用户
participant CLI as iris-runtime.js
participant Installer as install.js
participant Binary as 预编译二进制
participant Engine as Iris 引擎
User->>CLI : npx iris-runtime dev/build/info
CLI->>CLI : 解析命令行参数
CLI->>Binary : execFileSync 调用
Binary->>Engine : 初始化运行时
Engine-->>Binary : 返回执行结果
Binary-->>CLI : 输出日志
CLI-->>User : 显示结果
```

**图表来源**
- [iris-runtime.js:53-131](file://iris-runtime/bin/iris-runtime.js#L53-L131)

**更新** 现在直接调用预编译二进制，无需查找本地构建的版本。

**章节来源**
- [iris-runtime.js:1-131](file://iris-runtime/bin/iris-runtime.js#L1-L131)

### 运行时编排器组件
运行时编排器是整个系统的协调中心：

```mermaid
classDiagram
class RuntimeOrchestrator {
-JsRuntime js_runtime
-ModuleRegistry module_registry
-Option~VNode~ root_vnode
-bool initialized
+new() RuntimeOrchestrator
+initialize() Result~(), String~
+load_vue_app(Path) Result~(), String~
-compile_sfc(Path) Result~SfcModule, String~
-execute_sfc_module(&SfcModule) Result~(), String~
+root_vnode() Option~&VNode~
+js_runtime() &mut JsRuntime
+is_initialized() bool
}
class JsRuntime {
+new() JsRuntime
+eval(code) Result~(), String~
+inject_bom(width, height) Result~(), String~
}
class ModuleRegistry {
+new() ModuleRegistry
+register(name, code) Result~(), String~
}
class SfcModule {
+String name
+String render_fn
+String script
+Vec~StyleBlock~ styles
+u64 source_hash
}
RuntimeOrchestrator --> JsRuntime : 使用
RuntimeOrchestrator --> ModuleRegistry : 管理
RuntimeOrchestrator --> SfcModule : 编译
```

**图表来源**
- [orchestrator.rs:40-162](file://crates/iris-engine/src/orchestrator.rs#L40-L162)

**章节来源**
- [orchestrator.rs:1-200](file://crates/iris-engine/src/orchestrator.rs#L1-L200)

### GPU 渲染组件
GPU 渲染系统提供硬件加速的图形渲染能力：

```mermaid
classDiagram
class Renderer {
-Surface surface
-Device device
-Queue queue
-SurfaceConfiguration surface_config
-PhysicalSize size
-Window window
-BatchRenderer batch_renderer
-TextureCache texture_cache
-Option~FileWatcher~ file_watcher
+new(Window) async Result~Self, Error~
+render() Result~(), Error~
+resize(u32, u32) Result~(), Error~
}
class BatchRenderer {
-Vec~DrawCommand~ draw_commands
-Vec~Vertex~ vertices
-u32 vertex_count
+add_draw_command(DrawCommand) void
+flush() Result~(), Error~
}
class TextureCache {
-HashMap~String, TextureEntry~ cache
-usize capacity
+get(key) Option~&TextureEntry~
+insert(key, entry) void
+clear() void
}
Renderer --> BatchRenderer : 包含
Renderer --> TextureCache : 使用
```

**图表来源**
- [lib.rs:82-162](file://crates/iris-gpu/src/lib.rs#L82-L162)

**章节来源**
- [lib.rs:1-200](file://crates/iris-gpu/src/lib.rs#L1-L200)

### SFC 编译组件
SFC 编译器负责将 Vue 单文件组件编译为可执行代码：

```mermaid
flowchart TD
Start([开始编译]) --> ReadFile["读取 .vue 文件"]
ReadFile --> ParseSFC["解析 SFC 结构"]
ParseSFC --> ExtractTemplate["提取 template 块"]
ParseSFC --> ExtractScript["提取 script 块"]
ParseSFC --> ExtractStyles["提取 style 块"]
ExtractTemplate --> CompileTemplate["编译模板为渲染函数"]
ExtractScript --> CheckLang{"检查语言类型"}
CheckLang --> |TypeScript| TransformTS["TypeScript 转译"]
CheckLang --> |JavaScript| SkipTS["跳过转译"]
TransformTS --> CompileScript["编译脚本"]
SkipTS --> CompileScript
CompileTemplate --> ProcessStyles["处理样式"]
CompileScript --> ProcessStyles
ProcessStyles --> ApplyCSSModules["应用 CSS Modules"]
ApplyCSSModules --> ApplyScopedCSS["应用 Scoped CSS"]
ApplyScopedCSS --> ApplySCSS["编译 SCSS/Less"]
ApplySCSS --> CreateModule["创建 SFC 模块"]
CreateModule --> End([编译完成])
```

**图表来源**
- [lib.rs:289-430](file://crates/iris-sfc/src/lib.rs#L289-L430)

**章节来源**
- [lib.rs:1-800](file://crates/iris-sfc/src/lib.rs#L1-L800)

## 依赖关系分析
Iris Runtime 的依赖关系已简化为仅依赖预编译二进制：

```mermaid
graph TB
subgraph "外部依赖"
NPM[NPM 包依赖]
Node[Node.js 运行时]
end
subgraph "NPM 包层"
IR[iris-runtime]
CMD[commander]
CHALK[chalk]
WHICH[which]
end
subgraph "预编译二进制层"
BIN[预编制二进制文件]
LOCAL[本地打包分发]
PREPARE[prepare-binaries.js]
end
subgraph "Rust 工作区"
WS[Workspace]
subgraph "CLI 依赖"
CLAP[clap]
COLORED[colored]
TRACING[tracing]
END
subgraph "渲染依赖"
WGPU[wgpu 24]
WINIT[winit 0.30]
END
subgraph "编译依赖"
SERDE[serde]
REGEX[regex]
TOKIO[tokio]
END
end
NPM --> IR
Node --> IR
IR --> CMD
IR --> CHALK
IR --> WHICH
IR --> BIN
BIN --> LOCAL
LOCAL --> PREPARE
WS --> CLAP
WS --> COLORED
WS --> TRACING
WS --> WGPU
WS --> WINIT
WS --> SERDE
WS --> REGEX
WS --> TOKIO
```

**图表来源**
- [package.json:45-49](file://iris-runtime/package.json#L45-L49)
- [Cargo.toml:13-32](file://Cargo.toml#L13-L32)

**更新** 移除了对 Rust 工具链的直接依赖，现在仅依赖预编译的二进制文件。

**章节来源**
- [package.json:1-60](file://iris-runtime/package.json#L1-L60)
- [Cargo.toml:1-32](file://Cargo.toml#L1-L32)

## 性能考虑
Iris Runtime 在多个层面实现了性能优化，包括预编译二进制的优势：

### 本地打包分发性能优化
- **立即可用**: 无需编译等待，安装即使用
- **优化构建**: 预编译版本针对目标平台进行优化
- **减少资源消耗**: 避免了本地编译所需的 CPU 和内存资源
- **离线安装**: 完全不需要网络访问，提升安装速度和可靠性

### 编译性能优化
- **缓存机制**: 使用 LRU 缓存存储编译结果，避免重复编译
- **正则表达式预编译**: 使用 LazyLock 避免重复编译正则表达式
- **全局编译器实例**: 复用 TypeScript 编译器实例，减少内存分配

### 渲染性能优化
- **批渲染系统**: 合并多个绘制命令，减少 GPU 状态切换
- **纹理缓存**: 复用纹理资源，避免重复上传
- **脏矩形管理**: 只重绘发生变化的区域

### 内存管理
- **异步运行时**: 使用 Tokio 多线程运行时处理并发任务
- **智能指针**: 广泛使用 Arc 和 Rc 管理共享资源
- **零拷贝设计**: 在可能的情况下避免不必要的数据复制

**更新** 本地打包分发显著提升了安装和启动性能。

## 故障排除指南

### 安装和分发问题
1. **二进制文件缺失**
   - 症状: 安装后提示预编译二进制文件不存在
   - 解决方案: 确保使用完整版本的 iris-runtime 包，检查 binaries/ 目录是否包含对应平台的二进制文件

2. **平台不支持**
   - 症状: 提示不支持当前平台架构
   - 解决方案: 检查操作系统和架构兼容性，确认对应的二进制文件是否存在

3. **权限问题**
   - 症状: 在 Unix 系统上无法执行二进制文件
   - 解决方案: 确保二进制文件具有执行权限

### 运行时问题
1. **WebGPU 不支持**
   - 症状: 渲染器初始化失败
   - 解决方案: 确保浏览器或系统支持 WebGPU

2. **GPU 设备初始化失败**
   - 症状: 找不到合适的 GPU 设备
   - 解决方案: 检查显卡驱动和 WebGPU 支持情况

### 维护者构建问题
1. **Rust 工具链缺失**
   - 症状: prepare-binaries 脚本提示 Rust 工具链未找到
   - 解决方案: 安装 Rust 工具链并确保交叉编译目标已安装

2. **构建失败**
   - 症状: 某个或多个平台构建失败
   - 解决方案: 检查对应平台的开发环境配置，确保所有依赖都正确安装

### 编译问题
1. **SFC 编译错误**
   - 症状: 模板或脚本编译失败
   - 解决方案: 检查 .vue 文件格式和语法

2. **TypeScript 类型检查失败**
   - 症状: 类型检查报错但编译继续
   - 解决方案: 修复 TypeScript 类型错误

**更新** 移除了源码编译相关的故障排除步骤，新增了本地打包分发和维护者构建特有的问题处理。

**章节来源**
- [install.js:52-59](file://iris-runtime/scripts/install.js#L52-L59)
- [prepare-binaries.js:97-102](file://iris-runtime/scripts/prepare-binaries.js#L97-L102)
- [lib.rs:134-278](file://crates/iris-sfc/src/lib.rs#L134-L278)

## 结论
Iris Runtime NPM 包成功地将 Rust 的高性能特性与 Vue 3 的开发体验相结合，并通过完全迁移到本地打包分发模式，进一步简化了用户的使用体验。该包现在无需任何 Rust 工具链即可使用，为现代前端开发提供了创新且易于使用的解决方案。

**更新** 最大的改进是完全消除了对 Rust 工具链的需求和网络访问，使更多开发者能够轻松使用 Iris Runtime。

主要优势包括：
- **零安装复杂度**: 无需安装 Rust 工具链，自动从包中复制预编译二进制
- **离线安装**: 完全不需要网络访问，提升安装速度和可靠性
- **跨平台支持**: 完整支持 Windows、macOS 和 Linux 平台
- **高性能**: 基于 Rust 和 WebGPU 的硬件加速渲染
- **易用性**: 简洁的 CLI 接口和自动化的安装流程
- **可靠性**: 本地打包分发确保二进制文件的完整性和一致性
- **可扩展性**: 模块化的架构设计便于功能扩展
- **维护友好**: 专门的维护者脚本简化了多平台二进制构建流程

**更新** 最大的改进是完全消除了对 Rust 工具链的需求，使更多开发者能够轻松使用 Iris Runtime。

未来发展方向可能包括完善 TypeScript 编译器集成、增强热重载功能、优化构建性能等。对于希望获得更快开发体验和更好性能的 Vue 3 开发者来说，Iris Runtime 是一个值得考虑的选择，特别是对于那些不想或无法安装 Rust 工具链的用户。

**更新** 本地打包分发模式为用户提供了前所未有的便利性，真正实现了"开箱即用"的开发体验。