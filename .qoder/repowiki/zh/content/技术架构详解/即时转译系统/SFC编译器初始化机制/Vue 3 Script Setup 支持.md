# Vue 3 Script Setup 支持

<cite>
**本文档引用的文件**
- [crates/iris-sfc/src/lib.rs](file://crates/iris-sfc/src/lib.rs)
- [crates/iris-sfc/src/script_setup.rs](file://crates/iris-sfc/src/script_setup.rs)
- [crates/iris-sfc/src/cache.rs](file://crates/iris-sfc/src/cache.rs)
- [crates/iris-sfc/src/template_compiler.rs](file://crates/iris-sfc/src/template_compiler.rs)
- [crates/iris-sfc/src/ts_compiler.rs](file://crates/iris-sfc/src/ts_compiler.rs)
- [crates/iris-sfc/src/css_modules.rs](file://crates/iris-sfc/src/css_modules.rs)
- [crates/iris-sfc/Cargo.toml](file://crates/iris-sfc/Cargo.toml)
- [Cargo.toml](file://Cargo.toml)
- [crates/iris-sfc/examples/sfc_demo.rs](file://crates/iris-sfc/examples/sfc_demo.rs)
- [TestComponent.vue](file://TestComponent.vue)
- [QUICK-START.md](file://QUICK-START.md)
- [SWC-IMPLEMENTATION-FEASIBILITY.md](file://SWC-IMPLEMENTATION-FEASIBILITY.md)
- [crates/iris-sfc/tests/integration_test.rs](file://crates/iris-sfc/tests/integration_test.rs)
</cite>

## 更新摘要
**变更内容**
- 新增数组形式的 defineProps 和 defineEmits 宏支持
- 扩展编译器宏转换器功能，支持更灵活的组件属性和事件定义
- 更新测试用例以验证新功能的正确性

## 目录
1. [项目概述](#项目概述)
2. [项目结构](#项目结构)
3. [核心组件](#核心组件)
4. [架构概览](#架构概览)
5. [详细组件分析](#详细组件分析)
6. [依赖关系分析](#依赖关系分析)
7. [性能考虑](#性能考虑)
8. [故障排除指南](#故障排除指南)
9. [结论](#结论)
10. [附录](#附录)

## 项目概述

Iris SFC（Single File Component）编译器是一个专为 Vue 3 设计的即时转译层，旨在实现"零编译直接运行源码"的目标。该项目的核心使命是解析 .vue 文件，提取 template/script/style，编译为可执行模块，支持 Vue 3 的 `<script setup>` 语法和编译器宏。

### 主要特性

- **零编译运行**：直接编译 .vue 文件，无需预构建步骤
- **Vue 3 Script Setup 支持**：完整支持 `<script setup>` 语法和编译器宏
- **即时转译**：TypeScript 到 JavaScript 的实时转换
- **热重载支持**：基于 LRU 缓存的毫秒级热重载
- **多格式支持**：支持 CSS Modules、SCSS、TypeScript 等多种样式和脚本格式

## 项目结构

```mermaid
graph TB
subgraph "Iris 工作空间"
subgraph "核心库"
A[iris-core]
B[iris-js]
C[iris-dom]
D[iris-gpu]
E[iris-layout]
end
subgraph "SFC 编译器"
F[iris-sfc]
G[iris-app]
end
end
subgraph "外部依赖"
H[swc 62]
I[html5ever]
J[lru]
K[xxhash-rust]
L[regex]
end
F --> H
F --> I
F --> J
F --> K
F --> L
F --> A
F --> B
```

**图表来源**
- [Cargo.toml:1-29](file://Cargo.toml#L1-L29)
- [crates/iris-sfc/Cargo.toml:1-38](file://crates/iris-sfc/Cargo.toml#L1-L38)

**章节来源**
- [Cargo.toml:1-29](file://Cargo.toml#L1-L29)
- [crates/iris-sfc/Cargo.toml:1-38](file://crates/iris-sfc/Cargo.toml#L1-L38)

## 核心组件

### SFC 编译器主模块

SFC 编译器的核心入口提供了完整的 .vue 文件编译功能，包括模板解析、脚本转换、样式编译和缓存管理。

```mermaid
classDiagram
class SfcModule {
+String name
+String render_fn
+String script
+Vec~StyleBlock~ styles
+u64 source_hash
}
class SfcDescriptor {
+Option~String~ template
+Option~String~ script
+ScriptAttrs script_attrs
+Vec~StyleRaw~ styles
}
class ScriptAttrs {
+String lang
+bool setup
}
class SfcCache {
+SfcCacheConfig config
+LruCache~CacheKey~ cache
+CacheStats stats
+get_or_compile() SfcModule
}
class TsCompiler {
+TsCompilerConfig config
+Arc~Compiler~ compiler
+compile() TsCompileResult
+type_check() TypeCheckResult
}
SfcModule --> StyleBlock
SfcDescriptor --> ScriptAttrs
SfcCache --> SfcModule
SfcCache --> CacheKey
SfcCache --> CacheStats
TsCompiler --> TsCompileResult
```

**图表来源**
- [crates/iris-sfc/src/lib.rs:86-114](file://crates/iris-sfc/src/lib.rs#L86-L114)
- [crates/iris-sfc/src/lib.rs:116-136](file://crates/iris-sfc/src/lib.rs#L116-L136)
- [crates/iris-sfc/src/script_setup.rs:62-69](file://crates/iris-sfc/src/script_setup.rs#L62-L69)
- [crates/iris-sfc/src/cache.rs:94-101](file://crates/iris-sfc/src/cache.rs#L94-L101)
- [crates/iris-sfc/src/ts_compiler.rs:140-144](file://crates/iris-sfc/src/ts_compiler.rs#L140-L144)

### 编译器宏转换器

专门处理 Vue 3 `<script setup>` 语法的编译器宏转换器，支持 `defineProps`、`defineEmits`、`defineExpose` 和 `withDefaults` 等宏。

```mermaid
flowchart TD
Start([开始: <script setup> 源码]) --> ParseMacros["解析编译器宏"]
ParseMacros --> DefineProps{"检测 defineProps?"}
DefineProps --> |是| PropsInterface["解析 Props 接口"]
DefineProps --> |否| DefineEmits{"检测 defineEmits?"}
PropsInterface --> RemoveProps["移除 defineProps 调用"]
RemoveProps --> DefineEmits
DefineEmits --> |是| EmitsInterface["解析 Emits 接口"]
DefineEmits --> |否| ExtractDeclarations["提取顶层声明"]
EmitsInterface --> RemoveEmits["移除 defineEmits 调用"]
RemoveEmits --> ExtractDeclarations
ExtractDeclarations --> GenerateSetup["生成 setup 函数"]
GenerateSetup --> WrapAsDefault["包装为标准组件"]
WrapAsDefault --> End([结束: 标准组件代码])
```

**图表来源**
- [crates/iris-sfc/src/script_setup.rs:157-198](file://crates/iris-sfc/src/script_setup.rs#L157-L198)
- [crates/iris-sfc/src/script_setup.rs:360-388](file://crates/iris-sfc/src/script_setup.rs#L360-L388)

**章节来源**
- [crates/iris-sfc/src/lib.rs:1-800](file://crates/iris-sfc/src/lib.rs#L1-L800)
- [crates/iris-sfc/src/script_setup.rs:1-483](file://crates/iris-sfc/src/script_setup.rs#L1-L483)

## 架构概览

Iris SFC 编译器采用模块化架构设计，每个组件负责特定的功能领域，通过清晰的接口进行交互。

```mermaid
graph TB
subgraph "编译器层"
A[SFC 编译器主模块]
B[模板编译器]
C[脚本编译器]
D[样式编译器]
end
subgraph "转换器层"
E[Script Setup 转换器]
F[TypeScript 编译器]
G[CSS Modules 处理器]
end
subgraph "基础设施层"
H[LRU 缓存]
I[正则表达式缓存]
J[全局编译器实例]
end
subgraph "外部依赖"
K[swc 62]
L[html5ever]
M[regex]
N[lru]
O[xxhash-rust]
end
A --> B
A --> C
A --> D
C --> E
C --> F
D --> G
A --> H
A --> I
C --> J
B --> L
C --> K
D --> M
H --> N
A --> O
```

**图表来源**
- [crates/iris-sfc/src/lib.rs:17-55](file://crates/iris-sfc/src/lib.rs#L17-L55)
- [crates/iris-sfc/src/cache.rs:20-26](file://crates/iris-sfc/src/cache.rs#L20-L26)

### 数据流图

```mermaid
sequenceDiagram
participant Dev as 开发者
participant SFC as SFC 编译器
participant Cache as LRU 缓存
participant Parser as 解析器
participant Transformer as 转换器
participant Generator as 生成器
Dev->>SFC : 编译 .vue 文件
SFC->>Cache : 检查缓存
alt 缓存命中
Cache-->>SFC : 返回编译结果
SFC-->>Dev : 返回缓存模块
else 缓存未命中
SFC->>Parser : 解析 SFC 结构
Parser-->>SFC : 返回描述符
SFC->>Transformer : 转换脚本和模板
Transformer-->>SFC : 返回转换结果
SFC->>Generator : 生成最终模块
Generator-->>SFC : 返回 SfcModule
SFC->>Cache : 缓存结果
SFC-->>Dev : 返回新模块
end
```

**图表来源**
- [crates/iris-sfc/src/lib.rs:218-259](file://crates/iris-sfc/src/lib.rs#L218-L259)
- [crates/iris-sfc/src/cache.rs:178-259](file://crates/iris-sfc/src/cache.rs#L178-L259)

**章节来源**
- [crates/iris-sfc/src/lib.rs:199-341](file://crates/iris-sfc/src/lib.rs#L199-L341)
- [crates/iris-sfc/src/cache.rs:136-302](file://crates/iris-sfc/src/cache.rs#L136-L302)

## 详细组件分析

### SFC 编译器主模块

SFC 编译器主模块是整个系统的中枢，负责协调各个子组件的工作流程。

#### 核心功能

1. **文件编译**：编译 .vue 文件为可执行模块
2. **缓存管理**：基于源码哈希的 LRU 缓存
3. **错误处理**：统一的错误类型和位置信息
4. **性能监控**：编译时间和缓存统计

#### 编译流程

```mermaid
flowchart TD
A[读取 .vue 文件] --> B[提取组件名]
B --> C[检查缓存]
C --> |命中| D[返回缓存结果]
C --> |未命中| E[解析 SFC 结构]
E --> F[编译模板]
F --> G[编译脚本]
G --> H[类型检查]
H --> I[编译样式]
I --> J[创建 SfcModule]
J --> K[缓存结果]
K --> L[返回模块]
D --> L
```

**图表来源**
- [crates/iris-sfc/src/lib.rs:218-341](file://crates/iris-sfc/src/lib.rs#L218-L341)

**章节来源**
- [crates/iris-sfc/src/lib.rs:199-341](file://crates/iris-sfc/src/lib.rs#L199-L341)

### Script Setup 转换器

Script Setup 转换器是 Vue 3 的核心创新，它将编译器宏转换为标准的组合式 API 代码。

#### 编译器宏支持

| 宏名称 | 功能 | 支持形式 | 示例 |
|--------|------|----------|------|
| `defineProps<T>()` | 定义组件 props | TypeScript 接口 | `defineProps<{ title: string }>()` |
| `defineProps<T>()` | 定义组件 props | 数组形式 | `defineProps(['title'])` |
| `defineEmits<T>()` | 定义组件 emits | TypeScript 接口 | `defineEmits<{ change: [number] }>()` |
| `defineEmits<T>()` | 定义组件 emits | 数组形式 | `defineEmits(['change'])` |
| `defineExpose()` | 暴露组件属性 | 标准形式 | `defineExpose({ publicMethod })` |
| `withDefaults()` | 设置 props 默认值 | 标准形式 | `withDefaults(defineProps<...>(), { count: 0 })` |

**更新** 新增数组形式的 defineProps 和 defineEmits 宏支持，允许开发者使用更简单的字符串数组语法。

#### 转换过程

```mermaid
sequenceDiagram
participant Input as 输入 : <script setup>
participant Parser as 宏解析器
participant Converter as 代码转换器
participant Output as 输出 : 标准组件
Input->>Parser : 解析 defineProps
Parser->>Converter : 检测接口形式
Parser->>Converter : 检测数组形式
Input->>Parser : 解析 defineEmits
Parser->>Converter : 检测接口形式
Parser->>Converter : 检测数组形式
Input->>Parser : 解析顶层声明
Parser->>Converter : 生成 setup 函数
Converter->>Output : 生成标准组件代码
```

**图表来源**
- [crates/iris-sfc/src/script_setup.rs:128-155](file://crates/iris-sfc/src/script_setup.rs#L128-L155)
- [crates/iris-sfc/src/script_setup.rs:157-198](file://crates/iris-sfc/src/script_setup.rs#L157-L198)

**章节来源**
- [crates/iris-sfc/src/script_setup.rs:119-388](file://crates/iris-sfc/src/script_setup.rs#L119-L388)

### TypeScript 编译器

基于 swc 62 的 TypeScript 编译器提供了高性能的 TypeScript 到 JavaScript 转译功能。

#### 编译配置

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `jsx` | false | 是否启用 JSX/TSX 支持 |
| `keep_decorators` | false | 是否保留装饰器 |
| `source_map` | true | 是否生成 source map |
| `target` | ES2020 | 目标 ECMAScript 版本 |

#### 编译流程

```mermaid
flowchart TD
A[TypeScript 源码] --> B[创建 SourceFile]
B --> C[解析 TypeScript]
C --> D[应用转换]
D --> E[生成 JavaScript]
E --> F[输出结果]
G[错误处理] --> H[try_with_handler]
H --> I[错误收集]
I --> J[错误报告]
```

**图表来源**
- [crates/iris-sfc/src/ts_compiler.rs:169-257](file://crates/iris-sfc/src/ts_compiler.rs#L169-L257)

**章节来源**
- [crates/iris-sfc/src/ts_compiler.rs:139-443](file://crates/iris-sfc/src/ts_compiler.rs#L139-L443)

### CSS Modules 处理器

CSS Modules 处理器实现了作用域化类名生成，支持 `<style module>` 语法。

#### 功能特性

1. **类名作用域化**：`.button` → `.button__hash123`
2. **类名映射生成**：`{ "button": "button__hash123" }`
3. **`:local()` 和 `:global()` 支持**
4. **短哈希生成**：基于内容的 8 位十六进制哈希

#### 处理流程

```mermaid
flowchart TD
A[原始 CSS] --> B[处理 :global() 伪类]
B --> C[处理 :local() 伪类]
C --> D[作用域化普通类名]
D --> E[生成类名映射]
E --> F[输出作用域化 CSS]
G[哈希生成] --> H[XXH3 算法]
H --> I[短哈希 (8位) ]
I --> D
```

**图表来源**
- [crates/iris-sfc/src/css_modules.rs:79-125](file://crates/iris-sfc/src/css_modules.rs#L79-L125)
- [crates/iris-sfc/src/css_modules.rs:137-165](file://crates/iris-sfc/src/css_modules.rs#L137-L165)

**章节来源**
- [crates/iris-sfc/src/css_modules.rs:1-287](file://crates/iris-sfc/src/css_modules.rs#L1-L287)

### LRU 缓存系统

LRU 缓存系统提供了基于源码内容哈希的内存缓存，支持毫秒级的热重载。

#### 缓存策略

| 特性 | 描述 |
|------|------|
| **缓存键** | 基于 SFC 源码字符串的 XXH3 哈希 |
| **缓存容量** | 默认 100 项，可配置 |
| **淘汰策略** | LRU 最久未使用淘汰 |
| **线程安全** | 使用 Mutex 保护缓存 |
| **内存管理** | 所有操作在内存中完成 |

#### 性能对比

| 场景 | 无缓存 | 有缓存 | 提升 |
|------|--------|--------|------|
| 首次编译 | 5-10 ms | 5-10 ms | - |
| 重复编译 | 5-10 ms | <0.01 ms | 500-1000x |
| 热重载 | 5-10 ms | <0.01 ms | 500-1000x |

**章节来源**
- [crates/iris-sfc/src/cache.rs:1-485](file://crates/iris-sfc/src/cache.rs#L1-L485)

## 依赖关系分析

### 内部依赖关系

```mermaid
graph TB
subgraph "内部模块"
A[iris-sfc] --> B[iris-core]
A --> C[iris-js]
A --> D[iris-dom]
A --> E[iris-gpu]
A --> F[iris-layout]
end
subgraph "外部库"
A --> G[swc 62]
A --> H[html5ever]
A --> I[regex]
A --> J[lru]
A --> K[xxhash-rust]
A --> L[serde]
A --> M[thiserror]
A --> N[tracing]
end
subgraph "工具链"
A --> O[cargo]
A --> P[rustc]
end
```

**图表来源**
- [crates/iris-sfc/Cargo.toml:11-38](file://crates/iris-sfc/Cargo.toml#L11-L38)
- [Cargo.toml:13-29](file://Cargo.toml#L13-L29)

### 外部依赖分析

| 依赖库 | 版本 | 用途 | 关键特性 |
|--------|------|------|----------|
| `swc` | 62 | TypeScript 编译 | 高性能，完整的 TS 转译 |
| `html5ever` | 0.27 | HTML 解析 | 标准 HTML5 解析 |
| `regex` | 1.10 | 正则表达式 | 预编译优化 |
| `lru` | 0.12 | LRU 缓存 | 线程安全，高效淘汰 |
| `xxhash-rust` | 0.8 | 哈希算法 | XXH3 快速哈希 |
| `serde` | 1.0 | 序列化 | JSON 序列化支持 |
| `tracing` | 0.1 | 日志系统 | 结构化日志 |

**章节来源**
- [crates/iris-sfc/Cargo.toml:11-38](file://crates/iris-sfc/Cargo.toml#L11-L38)
- [Cargo.toml:13-29](file://Cargo.toml#L13-L29)

## 性能考虑

### 编译性能优化

1. **预编译正则表达式**：避免每次调用时重新编译
2. **全局编译器实例**：复用 swc 编译器实例
3. **LRU 缓存**：毫秒级热重载
4. **源码哈希**：快速缓存键生成

### 内存使用优化

1. **内存缓存**：所有编译结果存储在内存中
2. **智能淘汰**：LRU 策略自动管理内存
3. **源码哈希**：避免重复存储相同内容
4. **配置化容量**：可调整缓存大小

### 编译时间统计

```mermaid
graph LR
A[编译开始] --> B[文件读取: ~1-5ms]
B --> C[正则解析: ~0.1-1ms]
C --> D[模板编译: ~1-10ms]
D --> E[脚本编译: ~2-15ms]
E --> F[样式编译: ~0.5-5ms]
F --> G[编译结束]
H[缓存命中] --> I[直接返回: ~0.01ms]
```

**图表来源**
- [crates/iris-sfc/src/lib.rs:236-259](file://crates/iris-sfc/src/lib.rs#L236-L259)
- [crates/iris-sfc/src/cache.rs:434-483](file://crates/iris-sfc/src/cache.rs#L434-L483)

## 故障排除指南

### 常见问题及解决方案

#### 1. 编码问题

**症状**：控制台显示乱码（如 `鎷掔粷璁块棶`）

**解决方案**：
```powershell
# 一键自动配置 UTF-8
.\Auto-Setup-UTF8.ps1

# 或临时启用 UTF-8
$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001 | Out-Null
```

**章节来源**
- [QUICK-START.md:1-77](file://QUICK-START.md#L1-L77)

#### 2. TypeScript 编译错误

**症状**：TypeScript 编译失败或类型检查错误

**解决方案**：
1. 检查 TypeScript 版本兼容性
2. 验证 swc 62 是否正确安装
3. 检查编译器配置

#### 3. 缓存问题

**症状**：缓存未更新或内存占用过高

**解决方案**：
```rust
// 清除缓存
cache.clear();

// 检查缓存统计
let stats = cache.stats();
println!("缓存命中率: {:.2}%", stats.hit_rate() * 100);
```

#### 4. 模板编译问题

**症状**：模板解析失败或渲染函数生成错误

**解决方案**：
1. 验证 HTML 语法正确性
2. 检查 Vue 指令使用
3. 确认模板编译器配置

**章节来源**
- [crates/iris-sfc/src/lib.rs:138-188](file://crates/iris-sfc/src/lib.rs#L138-L188)
- [crates/iris-sfc/src/cache.rs:261-302](file://crates/iris-sfc/src/cache.rs#L261-L302)

## 结论

Iris SFC 编译器是一个功能完整、性能优异的 Vue 3 单文件组件编译解决方案。通过精心设计的模块化架构和多项性能优化技术，它实现了真正的"零编译直接运行源码"目标。

### 主要成就

1. **完整的 Vue 3 支持**：完美支持 `<script setup>` 语法和所有编译器宏
2. **高性能编译**：毫秒级编译时间和热重载响应
3. **智能缓存系统**：基于源码哈希的 LRU 缓存，大幅提升开发体验
4. **类型安全保证**：基于 swc 的 TypeScript 编译，确保类型安全
5. **易于使用**：简洁的 API 设计，便于集成到现有项目中
6. **灵活的宏支持**：支持 TypeScript 接口形式和数组形式的 defineProps 和 defineEmits 宏

### 未来发展方向

1. **完整 swc 集成**：实现完整的 TypeScript 编译功能
2. **更多指令支持**：扩展模板编译器以支持更多 Vue 指令
3. **性能进一步优化**：探索更多编译时优化技术
4. **开发工具集成**：提供更好的 IDE 和调试支持

## 附录

### 快速开始

```powershell
# 运行演示程序
cargo run -p iris-sfc --example sfc_demo

# 运行测试
cargo test -p iris-sfc

# 构建项目
cargo build -p iris-sfc
```

### 配置选项

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `IRIS_SOURCE_MAP` | false | 是否生成 Source Map |
| `IRIS_CACHE_CAPACITY` | 100 | 缓存容量 |
| `IRIS_CACHE_ENABLED` | true | 是否启用缓存 |
| `IRIS_TYPE_CHECK` | false | 是否启用类型检查 |
| `IRIS_TYPE_CHECK_STRICT` | false | 是否启用严格类型检查 |

### 示例组件

```vue
<template>
  <div class="container">
    <h1>{{ message }}</h1>
    <button @click="increment">
      Count: {{ count }}
    </button>
  </div>
</template>

<script setup>
import { ref } from 'vue'

// TypeScript 接口形式
const props1 = defineProps<{
  title: string
  count?: number
}>()

// 数组形式（新增）
const props2 = defineProps(['title', 'count'])

// TypeScript 接口形式
const emit1 = defineEmits<{
  change: [value: number]
  update: []
}>()

// 数组形式（新增）
const emit2 = defineEmits(['change', 'update'])

const count = ref(0)
const message: string = "Hello, Iris!"
</script>

<style scoped>
.container {
  padding: 20px;
}
</style>
```

**章节来源**
- [crates/iris-sfc/examples/sfc_demo.rs:128-293](file://crates/iris-sfc/examples/sfc_demo.rs#L128-L293)
- [TestComponent.vue:1-37](file://TestComponent.vue#L1-L37)
- [crates/iris-sfc/tests/integration_test.rs:20-23](file://crates/iris-sfc/tests/integration_test.rs#L20-L23)