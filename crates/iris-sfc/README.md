# Iris SFC 编译器

> Vue 3 单文件组件 (SFC) 的高性能 Rust 编译器

Iris SFC 是一个功能完整的 Vue 3 SFC 编译器，使用 Rust 编写，提供快速的编译速度和完整的 Vue 3 特性支持。

## ✨ 特性

### 🎯 完整的 Vue 3 支持

- **模板编译器**: 13+ 个 Vue 指令支持
- **CSS Modules**: `<style module>` 完全支持
- **TypeScript**: 基于 swc 62 的快速转译
- **Composition API**: `<script setup>` 和编译器宏
- **热重载缓存**: XXH3 + LRU 智能缓存

### 🚀 高性能

- **快速编译**: 平均 1-3ms 编译时间
- **智能缓存**: 1000-3000x 缓存命中加速
- **内存优化**: 可选的 Source Map 禁用
- **并行处理**: 线程安全的编译器实例

### 🔧 灵活配置

- **环境变量**: 运行时配置
- **可选类型检查**: 集成 tsc
- **自定义容量**: 缓存大小可调
- **严格模式**: 开发/生产环境适配

---

## 📦 安装

### 作为依赖添加

```toml
[dependencies]
iris-sfc = { path = "../crates/iris-sfc" }
```

### 快速开始

```rust
use iris_sfc::compile;

// 编译 .vue 文件
let module = compile("src/components/App.vue")?;

println!("组件名称: {}", module.name);
println!("渲染函数: {}", module.render_fn);
println!("脚本代码: {}", module.script);
println!("样式数量: {}", module.styles.len());
```

---

## 📖 使用指南

### 基本用法

#### 1. 编译单个 .vue 文件

```rust
use iris_sfc::compile;

fn main() {
    let result = compile("MyComponent.vue");
    
    match result {
        Ok(module) => {
            println!("编译成功!");
            println!("组件: {}", module.name);
            println!("渲染函数长度: {}", module.render_fn.len());
            println!("脚本长度: {}", module.script.len());
        }
        Err(e) => {
            eprintln!("编译失败: {}", e);
        }
    }
}
```

#### 2. 从字符串编译（用于测试）

```rust
use iris_sfc::compile_from_string;

let vue_source = r#"
<template>
  <div>{{ message }}</div>
</template>

<script setup lang="ts">
const message = "Hello Iris!"
</script>
"#;

let module = compile_from_string("TestComponent", vue_source)?;
```

---

## 🎨 支持的 Vue 功能

### 模板指令

Iris SFC 支持以下所有 Vue 3 指令：

#### 条件渲染
```vue
<template>
  <div v-if="isVisible">显示</div>
  <div v-else-if="isOther">其他</div>
  <div v-else>默认</div>
</template>
```

#### 列表渲染
```vue
<template>
  <ul>
    <li v-for="(item, index) in items" :key="item.id">
      {{ index }}: {{ item.name }}
    </li>
  </ul>
</template>
```

#### 数据绑定
```vue
<template>
  <!-- v-bind -->
  <img :src="imageUrl" :alt="title">
  
  <!-- v-on -->
  <button @click="handleClick" @mouseover="onHover">
    点击
  </button>
  
  <!-- v-model -->
  <input v-model="username" />
</template>
```

#### 内容渲染
```vue
<template>
  <!-- v-text: 设置 textContent -->
  <span v-text="message"></span>
  
  <!-- v-html: 设置 innerHTML (注意 XSS 风险) -->
  <div v-html="rawHtml"></div>
  
  <!-- v-show: 切换 display -->
  <div v-show="isVisible">内容</div>
</template>
```

#### 其他指令
```vue
<template>
  <!-- v-once: 一次性渲染 -->
  <div v-once>{{ staticContent }}</div>
  
  <!-- v-pre: 跳过编译 -->
  <div v-pre>{{ 不编译 }}</div>
  
  <!-- v-cloak: 隐藏未编译内容 -->
  <div v-cloak>{{ message }}</div>
  
  <!-- v-memo: 记忆优化 -->
  <div v-memo="[count, text]">{{ content }}</div>
  
  <!-- v-slot: 插槽 -->
  <my-component>
    <template #header="{ title }">
      <h1>{{ title }}</h1>
    </template>
  </my-component>
</template>
```

---

### CSS Modules

#### 基本用法

```vue
<template>
  <div :class="$style.container">
    <button :class="$style.button">点击</button>
  </div>
</template>

<style module>
.container {
  padding: 20px;
  background: white;
}

.button {
  color: blue;
  cursor: pointer;
}
</style>
```

**编译输出**:
```javascript
{
  styles: [{
    css: ".container__a1b2c3d4 { padding: 20px; } .button__a1b2c3d4 { color: blue; }",
    scoped: true,
    module: true,
    class_mapping: {
      "container": "container__a1b2c3d4",
      "button": "button__a1b2c3d4"
    }
  }]
}
```

#### :global() 和 :local()

```vue
<style module>
/* 局部作用域（默认） */
.local-class {
  color: red;
}

/* 全局类名 */
:global(.global-class) {
  font-size: 14px;
}

/* 显式局部 */
:local(.explicit-local) {
  background: blue;
}
</style>
```

#### 混合使用

```vue
<template>
  <div>Test</div>
</template>

<!-- 普通 scoped 样式 -->
<style scoped>
.normal {
  color: blue;
}
</style>

<!-- CSS Modules -->
<style module>
.module-class {
  color: red;
}
</style>
```

---

### TypeScript 编译

#### 基本 TypeScript

```vue
<script lang="ts">
interface User {
  name: string;
  age: number;
}

const user: User = {
  name: "Iris",
  age: 1
};

function greet(user: User): string {
  return `Hello, ${user.name}!`;
}
</script>
```

#### 泛型和接口

```vue
<script lang="ts">
interface ApiResponse<T> {
  data: T;
  status: number;
  message: string;
}

async function fetchData<T>(url: string): Promise<ApiResponse<T>> {
  const response = await fetch(url);
  return response.json();
}

type User = { id: number; name: string };

const users = await fetchData<User[]>("/api/users");
</script>
```

#### 装饰器

```vue
<script lang="ts">
function log(target: any, key: string, descriptor: PropertyDescriptor) {
  console.log(`Method ${key} called`);
  return descriptor;
}

class MyClass {
  @log
  myMethod() {
    console.log("executing");
  }
}
</script>
```

---

### `<script setup>` 和编译器宏

#### 基本用法

```vue
<script setup lang="ts">
import { ref, computed } from 'vue'

const count = ref(0)
const doubled = computed(() => count.value * 2)

function increment() {
  count.value++
}
</script>

<template>
  <button @click="increment">
    Count: {{ count }} (Doubled: {{ doubled }})
  </button>
</template>
```

#### defineProps

```vue
<script setup lang="ts">
const props = defineProps<{
  title: string
  count?: number
  disabled: boolean
}>()

// 使用 props
console.log(props.title)
console.log(props.count ?? 0)
</script>
```

**编译输出**:
```javascript
export default {
  props: {
    title: { type: String, required: true },
    count: { type: Number, required: false },
    disabled: { type: Boolean, required: true }
  },
  setup(props, { emit }) {
    console.log(props.title)
    console.log(props.count ?? 0)
    return { props }
  }
}
```

#### defineEmits

```vue
<script setup lang="ts">
const emit = defineEmits<{
  change: [value: number]
  update: []
  delete: [id: string, reason: string]
}>()

function handleChange(newValue: number) {
  emit('change', newValue)
}
</script>
```

**编译输出**:
```javascript
export default {
  emits: ['change', 'update', 'delete'],
  setup(props, { emit }) {
    function handleChange(newValue) {
      emit('change', newValue)
    }
    return { emit, handleChange }
  }
}
```

#### withDefaults

```vue
<script setup lang="ts">
const props = withDefaults(defineProps<{
  title: string
  count?: number
  theme?: 'light' | 'dark'
}>(), {
  count: 0,
  theme: 'light'
})
</script>
```

**编译输出**:
```javascript
export default {
  props: {
    title: { type: String, required: true },
    count: { type: Number, default: 0 },
    theme: { type: null, default: 'light' }
  },
  setup(props, { emit }) {
    return { props }
  }
}
```

#### 完整示例

```vue
<template>
  <div :class="$style.container" v-show="isVisible">
    <h1 v-text="title"></h1>
    <p>Count: {{ count }}</p>
    <button 
      :class="$style.button"
      @click="increment"
      :disabled="count >= maxCount"
    >
      Increment
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{
  title: string
  initialCount?: number
  maxCount?: number
}>()

const emit = defineEmits<{
  change: [value: number]
  maxReached: []
}>()

const count = ref(props.initialCount ?? 0)
const maxCount = ref(props.maxCount ?? 100)
const isVisible = ref(true)

function increment() {
  if (count.value < maxCount.value) {
    count.value++
    emit('change', count.value)
    
    if (count.value >= maxCount.value) {
      emit('maxReached')
    }
  }
}
</script>

<style module>
.container {
  padding: 20px;
  border: 1px solid #ccc;
}

.button {
  padding: 8px 16px;
  background: blue;
  color: white;
}

.button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

:global(.external-helper) {
  position: absolute;
  top: 0;
}
</style>
```

---

## ⚙️ 配置选项

### 环境变量

| 变量 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `IRIS_SOURCE_MAP` | `bool` | `false` | 是否生成 Source Map |
| `IRIS_CACHE_CAPACITY` | `usize` | `100` | 缓存容量（组件数量） |
| `IRIS_CACHE_ENABLED` | `bool` | `true` | 是否启用缓存 |
| `IRIS_TYPE_CHECK` | `bool` | `false` | 是否启用类型检查 |
| `IRIS_TYPE_CHECK_STRICT` | `bool` | `false` | 类型检查严格模式 |

### 使用示例

#### 启用 Source Map（用于浏览器调试）

```bash
IRIS_SOURCE_MAP=true cargo run
```

#### 调整缓存大小

```bash
# 缓存 500 个组件
IRIS_CACHE_CAPACITY=500 cargo run

# 禁用缓存
IRIS_CACHE_ENABLED=false cargo run
```

#### 启用类型检查

```bash
# 基本类型检查
IRIS_TYPE_CHECK=true cargo run

# 严格模式
IRIS_TYPE_CHECK=true IRIS_TYPE_CHECK_STRICT=true cargo run
```

**注意**: 类型检查需要系统安装 TypeScript：
```bash
npm install -g typescript
```

---

## 🔍 错误处理

### 编译错误

Iris SFC 提供详细的错误信息：

```rust
use iris_sfc::compile;

match compile("Broken.vue") {
    Ok(module) => {
        println!("编译成功");
    }
    Err(e) => {
        // 错误包含文件位置和描述
        eprintln!("编译失败: {}", e);
        // 输出: "Script compile error: TypeScript compilation failed: ..."
    }
}
```

### 类型检查错误

当启用类型检查时，类型错误会作为警告输出（不阻断编译）：

```
[WARN] Type check failed (non-fatal) - error_count: 3
```

如需阻断编译，可修改 `lib.rs` 中的错误处理逻辑。

---

## 📊 性能

### 基准测试

| 操作 | 时间 | 说明 |
|------|------|------|
| 首次编译 | 1-3ms | 包含 TS 转译 |
| 缓存命中 | 3-6μs | 1000-3000x 加速 |
| 模板编译 | <1ms | 取决于复杂度 |
| CSS Modules | <1ms | 取决于样式数量 |

### 内存使用

| 配置 | 内存占用 | 说明 |
|------|----------|------|
| 默认 | 中等 | Source Map 禁用 |
| Source Map 启用 | +30-50% | 用于调试 |
| 缓存 100 项 | ~5MB | 取决于组件大小 |

### 优化建议

1. **生产环境**: 禁用 Source Map
2. **开发环境**: 启用缓存和类型检查
3. **大型项目**: 增加缓存容量到 500-1000
4. **热重载**: 保持缓存启用以获得最佳性能

---

## 🧪 测试

### 运行测试

```bash
# 运行所有测试
cargo test -p iris-sfc

# 运行特定模块测试
cargo test -p iris-sfc template_compiler
cargo test -p iris-sfc css_modules
cargo test -p iris-sfc script_setup

# 带输出
cargo test -p iris-sfc -- --nocapture
```

### 测试覆盖

- ✅ 58 个单元测试
- ✅ 模板编译器测试（17 个）
- ✅ CSS Modules 测试（7 个）
- ✅ Script Setup 测试（6 个）
- ✅ TypeScript 编译测试（11 个）
- ✅ 缓存系统测试（6 个）
- ✅ 集成测试（11 个）

---

## 🏗️ 架构

### 模块结构

```
iris-sfc/
├── src/
│   ├── lib.rs                  # 主入口和编译流程
│   ├── template_compiler.rs    # Vue 模板编译器
│   ├── ts_compiler.rs          # TypeScript 编译器 (swc)
│   ├── css_modules.rs          # CSS Modules 处理器
│   ├── script_setup.rs         # Script Setup 转换器
│   └── cache.rs                # 热重载缓存系统
├── Cargo.toml
└── README.md
```

### 编译流程

```
.vue 文件
    ↓
parse_sfc() - 解析 SFC
    ↓
┌─────────────┬──────────────┬─────────────┐
│   Template  │    Script    │    Styles   │
│   Compiler  │   Compiler   │   Compiler  │
└─────────────┴──────────────┴─────────────┘
    ↓              ↓              ↓
┌─────────────┬──────────────┬─────────────┐
│ Render Fn   │  JS Code     │  CSS Blocks │
└─────────────┴──────────────┴─────────────┘
    ↓              ↓              ↓
    └──────────────┴──────────────┘
                   ↓
              SfcModule
```

### 缓存流程

```
compile("App.vue")
    ↓
计算源码哈希 (XXH3)
    ↓
检查缓存
    ├─ 命中 → 返回缓存结果 (3-6μs)
    └─ 未命中 → 执行编译 → 存入缓存 → 返回结果 (1-3ms)
```

---

## 🔮 未来计划

### 短期
- [ ] 更复杂的 props 类型支持（嵌套对象、联合类型）
- [ ] CSS 预处理器支持（SCSS, Less）
- [ ] 更详细的错误位置和修复建议
- [ ] 编译性能分析和优化报告

### 中期
- [ ] 完整的 AST 转换（替代正则解析）
- [ ] Tree Shaking 优化
- [ ] 代码分割支持
- [ ] SSR 编译模式

### 长期
- [ ] WASM 编译（浏览器端使用）
- [ ] VS Code 插件（实时编译预览）
- [ ] 图形化编译分析工具
- [ ] 插件系统

---

## 📝 许可证

MIT License

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

### 开发设置

```bash
# 克隆仓库
git clone https://gitee.com/wanquanbuhuime/iris.git
cd iris

# 运行测试
cargo test -p iris-sfc

# 格式化代码
cargo fmt

# 代码检查
cargo clippy
```

---

## 📞 支持

- **文档**: 查看本 README
- **问题**: 提交 [Issue](https://gitee.com/wanquanbuhuime/iris/issues)
- **讨论**: 参与 [Discussions](https://gitee.com/wanquanbuhuime/iris/discussions)

---

**Iris SFC** - 让 Vue 3 编译更快、更简单 🚀
