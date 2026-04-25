# Iris SFC TypeScript 编译架构分析

## 问题背景

在集成测试中发现 `test_full_vue3_sfc_compilation` 失败，错误信息：
```
TypeScript compilation failed: Expression expected
```

这引发了对 swc TypeScript 处理能力的深入分析。

---

## 1. swc 的 TypeScript 处理能力

### 1.1 swc 能做什么

**swc (Speedy Web Compiler)** 是一个高性能的 JavaScript/TypeScript 编译器，核心能力：

| 能力 | 支持 | 说明 |
|------|------|------|
| **类型擦除** | ✅ | 移除类型注解 (`: string`, `: number`) |
| **接口转换** | ✅ | 移除 `interface` 声明 |
| **泛型处理** | ✅ | 移除泛型参数 `<T>` |
| **枚举转换** | ✅ | 转换为 IIFE 对象 |
| **命名空间** | ✅ | 转换为 IIFE |
| **类型别名** | ✅ | 移除 `type` 声明 |
| **装饰器** | ⚠️ | 需要特殊配置 (`decorators: true`) |
| **JSX/TSX** | ⚠️ | 需要配置转换目标 |
| **模块系统** | ✅ | ESM ↔ CJS 转换 |
| **语法错误检测** | ✅ | 捕获真正的语法错误 |

### 1.2 swc **不能**做什么

| 能力 | 支持 | 说明 |
|------|------|------|
| **类型检查** | ❌ | 不做类型安全检查（需要 tsc） |
| **Vue 编译器宏** | ❌ | 不认识 `defineProps`, `defineEmits` |
| **SFC 解析** | ❌ | 不解析 `.vue` 文件格式 |
| **模板编译** | ❌ | 不编译 Vue 模板语法 |

---

## 2. 当前 `ts_compiler.rs` 的角色

### 2.1 架构定位

```
┌─────────────────────────────────────────────────┐
│              Iris SFC Compiler                  │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌──────────────┐      ┌──────────────────┐    │
│  │ script_setup │─────▶│  ts_compiler.rs  │    │
│  │   (自定义)   │      │   (swc 封装)     │    │
│  └──────────────┘      └──────────────────┘    │
│         │                        │              │
│         │ 转换编译器宏           │ 类型擦除     │
│         │                        │              │
│         ▼                        ▼              │
│  defineProps<T>()        const props = {...}    │
│  defineEmits<T>()        export default {...}   │
│                                                 │
└─────────────────────────────────────────────────┘
```

### 2.2 职责分工

#### `script_setup.rs` - Vue 编译器宏转换
```rust
// 输入（Vue 编译器宏）
const props = defineProps<{
  title: string
  count?: number
}>()

// 输出（标准 JavaScript）
/* props injected */
// + 在组件对象中注入:
// props: {
//   title: { type: String, required: true },
//   count: { type: Number, required: false }
// }
```

**职责**：
- ✅ 解析 TypeScript 接口定义
- ✅ 转换为运行时 props/emits 验证
- ✅ 提取顶层声明并生成 `return` 语句
- ✅ 处理 `withDefaults` 默认值

#### `ts_compiler.rs` - TypeScript 类型擦除
```rust
// 输入（TypeScript）
function greet(user: { name: string }): string {
  return `Hello, ${user.name}!`;
}

// 输出（JavaScript）
function greet(user) {
  return `Hello, ${user.name}!`;
}
```

**职责**：
- ✅ 移除类型注解
- ✅ 移除接口/类型别名
- ✅ 转换枚举和命名空间
- ✅ 目标版本降级（ES2020 → ES5）
- ❌ **不做**类型检查（可选，通过 tsc）

---

## 3. 编译流程详解

### 3.1 完整流程

```
.vue 文件
    ↓
┌─────────────────────────────────────┐
│ 1. parse_sfc()                      │
│    分离 template/script/style       │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 2. compile_script()                 │
│                                     │
│    if <script setup>:               │
│      ↓                              │
│      transform_script_setup()       │
│      - 解析 defineProps<T>()        │
│      - 解析 defineEmits<T>()        │
│      - 生成标准组件对象             │
│      ↓                              │
│    TS_COMPILER.compile()            │
│    - 使用 swc 擦除类型              │
│    - 生成纯 JavaScript              │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 3. compile_template()               │
│    生成 render() 函数               │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 4. compile_styles()                 │
│    作用域化 / CSS Modules           │
└─────────────────────────────────────┘
    ↓
SfcModule { render_fn, script, styles }
```

### 3.2 关键代码路径

```rust
// lib.rs: compile_script()
fn compile_script(...) -> Result<String, SfcError> {
    // 步骤 1: script_setup 转换（自定义逻辑）
    let processed_script = if attrs.setup {
        script_setup::transform_script_setup(script)?
        // ↓
        // 输入:  const props = defineProps<{ title: string }>()
        // 输出:  export default { props: {...}, setup() {...} }
    } else {
        script.to_string()
    };

    // 步骤 2: swc 类型擦除
    let result = TS_COMPILER.compile(&processed_script, file_name)?;
    // ↓
    // 输入:  function greet(user: User): string { ... }
    // 输出:  function greet(user) { ... }
    
    Ok(result.code)
}
```

---

## 4. 集成测试失败原因分析

### 4.1 错误信息

```
TypeScript compilation failed: Expression expected
span: [168..173]
```

### 4.2 根本原因

**问题**：`script_setup.rs` 生成的代码包含 Vue 编译器宏，**但未被完全移除**。

#### 场景 1: 泛型形式 `defineProps<{...}>()`

```typescript
// 输入
const props = defineProps<{
  title: string
}>()

// script_setup.rs 转换后（期望）
/* props injected */
export default {
  props: { title: { type: String, required: true } },
  setup(props, { emit }) { ... }
}

// 实际情况（可能）
// 如果正则匹配失败，defineProps 仍保留在代码中
// swc 看到 defineProps<{...}>() 无法识别，报错 "Expression expected"
```

**原因**：
- `defineProps<{...}>()` 是 **Vue 特有的编译器宏**
- 不是合法的 JavaScript/TypeScript 语法
- swc 不认识它，解析时报错

#### 场景 2: 数组形式 `defineProps([...])`

```javascript
// 输入
const props = defineProps(['title', 'count'])

// script_setup.rs 应该转换为
/* props injected */
export default {
  props: ['title', 'count'],
  setup(props, { emit }) { ... }
}

// 如果正则匹配失败
// swc 看到 defineProps(...) 调用
// 由于没有定义这个函数，可能报错
```

### 4.3 正则匹配问题

查看 `script_setup.rs` 中的正则：

```rust
// 泛型形式
static PROPS_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"defineProps<\{([^}]+)\}>\(\)"#).unwrap()
});

// 数组形式（新增）
static PROPS_ARRAY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"defineProps\(\[([^\]]+)\]\)"#).unwrap()
});
```

**潜在问题**：
1. **多行格式**：正则可能不匹配跨行的 props 定义
2. **空格/换行**：正则对空白字符敏感
3. **嵌套类型**：`{ props: { nested: string } }` 会导致 `[^}]+` 提前结束

---

## 5. swc 与 tsc 的关系

### 5.1 功能对比

| 特性 | swc | tsc (TypeScript Compiler) |
|------|-----|---------------------------|
| **类型擦除** | ✅ 快速 | ✅ |
| **类型检查** | ❌ | ✅ 完整 |
| **编译速度** | ⚡ 极快 (Rust) | 🐌 较慢 |
| **错误报告** | 基础 | 详细 |
| **类型推导** | ❌ | ✅ 完整 |
| **类型守卫** | ❌ | ✅ |
| **模块解析** | 有限 | ✅ 完整 |

### 5.2 在 Iris 中的使用

```rust
// ts_compiler.rs

// swc: 用于类型擦除（必须）
pub fn compile(&self, source: &str, filename: &str) -> Result<TsCompileResult, String> {
    // 使用 swc 编译，移除类型注解
    // 不做类型检查
}

// tsc: 用于类型检查（可选）
pub fn type_check(&self, source: &str, filename: &str, config: &TypeCheckConfig) -> TypeCheckResult {
    if !config.enabled {
        return TypeCheckResult::Skipped;
    }
    
    // 调用外部 tsc 命令
    // npm install -g typescript
    Command::new("tsc")
        .arg("--noEmit")
        .arg(file_path)
        .output()
}
```

### 5.3 为什么需要两者？

**swc 的优势**：
- ⚡ 速度快 10-20 倍
- 🦀 Rust 实现，内存安全
- 📦 易于集成（Rust crate）
- 🎯 专注代码转换

**tsc 的优势**：
- 🔍 完整的类型检查
- 🛡️ 捕获类型错误
- 📊 详细的错误报告
- 🌐 官方 TypeScript 编译器

**Iris 的策略**：
- **开发时**：可选启用 tsc 类型检查（`IRIS_TYPE_CHECK=true`）
- **生产时**：只使用 swc 快速编译（默认）
- **灵活配置**：通过环境变量控制

---

## 6. 解决方案

### 6.1 短期方案：修复正则匹配

**问题**：正则表达式不能处理所有格式

**解决**：
```rust
// 改进正则，支持多行和空格
static PROPS_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)defineProps\s*<\s*\{([^}]+)\}\s*>\s*\(\s*\)"#).unwrap()
});
// (?s) 启用 DOTALL 模式，. 可以匹配换行符
```

### 6.2 中期方案：使用 AST 解析

**问题**：正则表达式脆弱，难以维护

**解决**：使用 swc 的 AST 解析器
```rust
use swc_ecma_parser::{Parser, StringInput, Syntax};

// 解析为 AST
let mut parser = Parser::new(
    Syntax::Typescript(TsSyntax::default()),
    StringInput::from(&script),
    None,
);

let module = parser.parse_module()?;

// 遍历 AST 查找 defineProps 调用
// 提取类型参数
// 转换为运行时 props
```

**优势**：
- ✅ 准确识别所有语法变体
- ✅ 处理嵌套类型
- ✅ 支持复杂表达式

**劣势**：
- ❌ 实现复杂度高
- ❌ 需要处理 AST 遍历
- ❌ 代码量增加 3-5 倍

### 6.3 长期方案：集成 Vue 官方编译器

**问题**：自己实现编译器宏转换容易出错

**解决**：使用 Vue 官方的 `@vue/compiler-sfc`

```rust
// 通过 Node.js FFI 调用
use nodejs_sys::eval;

pub fn compile_script_setup(script: &str) -> String {
    let js_code = format!(r#"
        const {{ compileScript }} = require('@vue/compiler-sfc');
        const descriptor = {{ scriptSetup: {{ content: `{}` }} }};
        return compileScript(descriptor, {{ id: 'test' }}).content;
    "#, script);
    
    eval(&js_code)
}
```

**优势**：
- ✅ 100% 兼容 Vue 3
- ✅ 支持所有编译器宏
- ✅ 官方维护

**劣势**：
- ❌ 需要 Node.js 运行时
- ❌ 性能下降（FFI 开销）
- ❌ 增加部署复杂度

---

## 7. 当前问题的具体修复

### 7.1 测试用例失败原因

```rust
// 测试中的代码
const props = defineProps(['title'])
const emit = defineEmits(['change'])
```

**执行流程**：
1. `script_setup::transform_script_setup()` 调用
2. `parse_macros()` 使用 `PROPS_ARRAY_RE` 匹配
3. **正则匹配成功** → `result.props = Some("['title']")`
4. 替换宏调用 → `/* props injected */`
5. 生成组件对象 → `export default { props: ['title'], ... }`
6. **但是**：原始代码中的 `const props = ...` 行**未被删除**！

### 7.2 根本问题

`parse_macros()` 替换了宏调用，但**没有删除变量声明**：

```rust
// 转换前
const props = defineProps(['title'])
const emit = defineEmits(['change'])

// 转换后（错误）
const props = /* props injected */   // ← 这行仍然保留！
const emit = /* emits injected */    // ← 这行也保留！

export default {
  props: ['title'],
  emits: ['change'],
  setup(props, { emit }) { ... }
}
```

**swc 看到的代码**：
```javascript
const props = /* props injected */
```

这实际上是合法的 JavaScript（注释），但可能在某些情况下导致解析问题。

### 7.3 正确的修复

需要**整行删除**宏调用，而不仅仅是替换：

```rust
fn parse_macros(script: &str) -> Result<MacroResult, String> {
    let mut result = MacroResult::default();
    let mut lines: Vec<&str> = script.lines().collect();
    
    // 查找并删除 defineProps 行
    lines.retain(|line| {
        if PROPS_ARRAY_RE.is_match(line) || PROPS_TYPE_RE.is_match(line) {
            false  // 删除这一行
        } else {
            true   // 保留
        }
    });
    
    // 同样处理 defineEmits
    lines.retain(|line| {
        !EMITS_ARRAY_RE.is_match(line) && !EMITS_TYPE_RE.is_match(line)
    });
    
    let transformed = lines.join("\n");
    // ... 继续处理
}
```

---

## 8. 总结

### 8.1 架构合理性

当前的双层架构是**合理的**：

| 层级 | 职责 | 实现 |
|------|------|------|
| **script_setup** | Vue 编译器宏转换 | 自定义 Rust 代码 |
| **ts_compiler** | TypeScript 类型擦除 | swc 封装 |
| **type_check** | 类型安全检查 | tsc（可选） |

**优势**：
- ✅ 职责清晰
- ✅ 性能最优（swc 快速）
- ✅ 灵活配置（可选类型检查）
- ✅ 无外部依赖（tsc 可选）

### 8.2 需要改进的地方

1. **编译器宏解析**：从正则升级到 AST 解析
2. **错误处理**：提供更详细的错误位置
3. **测试覆盖**：添加更多边界情况测试
4. **文档**：明确说明 swc 和 tsc 的分工

### 8.3 swc 的定位

**swc 是 Iris SFC 的核心编译器**，负责：
- ✅ TypeScript → JavaScript 转换
- ✅ 类型注解擦除
- ✅ 目标版本降级
- ✅ 代码优化

**但 swc 不处理**：
- ❌ Vue 特有语法（编译器宏）
- ❌ 类型检查（需要 tsc）
- ❌ SFC 文件解析

---

## 9. 参考资源

- [swc 官方文档](https://swc.rs/docs/)
- [Vue 3 编译器宏文档](https://vuejs.org/api/sfc-script-setup.html#defineprops-defineemits)
- [TypeScript 编译器 API](https://github.com/microsoft/TypeScript/wiki/Using-the-Compiler-API)
- [swc vs tsc 性能对比](https://swc.rs/blog/2021/06/03/swc-reaches-production-readiness)

---

**结论**：当前架构设计合理，需要修复的是 `script_setup.rs` 中正则替换的逻辑问题，而非 swc 本身的能力不足。
