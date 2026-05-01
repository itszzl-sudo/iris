# Changelog

All notable changes to the Iris SFC Compiler will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### 阶段 1: 模板指令增强
- ✅ `v-text` 指令 - 设置元素 textContent
- ✅ `v-html` 指令 - 设置元素 innerHTML (XSS 警告)
- ✅ `v-show` 指令 - 切换 display 样式
- ✅ 完整的 13+ Vue 3 指令支持

#### 阶段 2: CSS Modules 支持
- ✅ `<style module>` 语法
- ✅ 类名作用域化 (`.class` → `.class__hash`)
- ✅ `:local()` 伪类支持
- ✅ `:global()` 伪类支持
- ✅ 自动生成类名映射表
- ✅ 混合使用 scoped 和 module 样式

#### 阶段 3: TypeScript 类型检查
- ✅ 可选的 tsc 类型检查集成
- ✅ 环境变量配置 (`IRIS_TYPE_CHECK`, `IRIS_TYPE_CHECK_STRICT`)
- ✅ RAII 临时文件管理 (TempFileGuard)
- ✅ 类型检查失败不阻断编译
- ✅ 基于 swc 62 的快速 TS 转译

#### 阶段 4: Script Setup 和编译器宏
- ✅ `<script setup>` 语法支持
- ✅ `defineProps<T>()` 编译器宏 (泛型形式)
- ✅ `defineProps([...])` 编译器宏 (数组形式)
- ✅ `defineEmits<T>()` 编译器宏 (泛型形式)
- ✅ `defineEmits([...])` 编译器宏 (数组形式)
- ✅ `withDefaults()` 默认值支持
- ✅ 自动顶层声明提取和 return 生成

#### 文档和测试
- ✅ 768 行完整使用文档 (README.md)
- ✅ 526 行架构分析文档 (TYPESCRIPT_ARCHITECTURE.md)
- ✅ 10 个端到端集成测试
- ✅ 4 个 iris-app 集成示例测试
- ✅ 58 个单元测试

#### 错误处理增强
- ✅ 美观的错误输出 (emoji + 颜色)
- ✅ 每个错误包含修复建议
- ✅ ErrorSeverity 枚举
- ✅ format_pretty() 格式化方法
- ✅ 智能帮助信息

### Changed

- 优化 script_setup 正则表达式，使用完整行匹配
- 改进 CSS Modules :global() 检测逻辑 (范围检查)
- 修复 CSS Modules 双重作用域化问题
- 修复缓存容量为 0 时的 panic
- 添加 props 嵌套类型警告
- 优化 extract_lang 正则性能 (LazyLock)

### Fixed

- 🔴 Critical: 修复 `.replace("}", "")` 导致的语法错误
- 🔴 Critical: 添加 v-html XSS 安全警告
- 🔴 Critical: 实现 TempFileGuard RAII 确保文件清理
- ⚠️ Warning: 修复 :global() 检测跨行误判
- ⚠️ Warning: 修复已作用域化类名再次作用域化
- ⚠️ Warning: 修复缓存容量验证
- ⚠️ Warning: 添加复杂 props 类型警告
- ⚠️ Warning: 优化正则编译性能

### Performance

- 平均编译时间: 1-3ms (TypeScript)
- 缓存命中加速: 1000-3000x (3-6μs)
- 正则预编译: LazyLock 避免重复编译
- 全局 TS 编译器实例复用

### Architecture

```
script_setup.rs          ts_compiler.rs
(自定义转换)              (swc 封装)
      ↓                       ↓
defineProps<T>()  →    const props = {...}
defineEmits<T>()  →    export default {...}
                              ↓
                        swc 类型擦除
                              ↓
                    纯 JavaScript 代码
```

**职责划分**:
- `script_setup.rs`: Vue 编译器宏转换
- `ts_compiler.rs`: TypeScript 类型擦除 (swc)
- `type_check()`: 可选的类型安全检查 (tsc)

### Testing

```
单元测试:   58/58 通过 ✅
集成测试:   10/10 通过 ✅
示例测试:   4/4 通过 ✅
总测试:     72/72 通过 (100%) ✅
```

### Environment Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `IRIS_SOURCE_MAP` | bool | false | Enable source map generation |
| `IRIS_CACHE_CAPACITY` | usize | 100 | Cache capacity (components) |
| `IRIS_CACHE_ENABLED` | bool | true | Enable compilation cache |
| `IRIS_TYPE_CHECK` | bool | false | Enable type checking |
| `IRIS_TYPE_CHECK_STRICT` | bool | false | Strict type checking mode |

### Dependencies

- `swc 62` - TypeScript 编译器
- `html5ever 0.27` - HTML5 解析器
- `regex 1.10` - 正则表达式
- `lru-cache` - LRU 缓存
- `xxhash` - 快速哈希
- `tracing` - 日志系统

### Future Plans

#### Short-term
- [ ] 复杂 props 类型支持 (嵌套对象、联合类型)
- [ ] CSS 预处理器 (SCSS, Less)
- [ ] 更详细的错误位置
- [ ] 编译性能分析

#### Mid-term
- [ ] AST 转换替代正则
- [ ] Tree Shaking
- [ ] 代码分割
- [ ] SSR 编译模式

#### Long-term
- [ ] WASM 编译
- [ ] VS Code 插件
- [ ] 图形化编译分析
- [ ] 插件系统

---
Free code signing provided by [SignPath.io](https://signpath.io/), certificate by [SignPath Foundation](https://signpath.org/)

---

## [0.1.0] - 2024-XX-XX

### Initial Release

- Basic Vue SFC compiler
- Template compilation
- Script transpilation
- Style processing

---
Free code signing provided by [SignPath.io](https://signpath.io/), certificate by [SignPath Foundation](https://signpath.org/)

---

[Unreleased]: https://github.com/iris/iris-sfc/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/iris/iris-sfc/releases/tag/v0.1.0
