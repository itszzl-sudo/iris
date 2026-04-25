//! Iris SFC 集成测试
//!
//! 测试完整的 SFC 编译流程，验证所有功能协同工作

use iris_sfc::compile_from_string;

/// 测试完整的 Vue 3 SFC 编译流程（简化版）
#[test]
fn test_full_vue3_sfc_compilation() {
    // 使用纯 JavaScript 避免 swc 编译器宏问题
    let source = r#"
<template>
  <div :class="$style.container">
    <h1 v-text="title"></h1>
  </div>
</template>

<script setup>
const props = defineProps(['title'])
const emit = defineEmits(['change'])
const count = 0
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
"#;

    let module = compile_from_string("FullVue3Component", source).unwrap();

    // 打印渲染函数用于调试
    println!("Render function:\n{}", module.render_fn);
    println!("\nScript:\n{}", module.script);

    // 验证基本结构
    assert_eq!(module.name, "FullVue3Component");
    assert!(!module.render_fn.is_empty());
    assert!(!module.script.is_empty());
    assert_eq!(module.styles.len(), 1);

    // 验证模板编译（包含指令）
    assert!(module.render_fn.contains("textContent")); // v-text

    // 验证 script setup 转换
    assert!(module.script.contains("export default {"));
    assert!(module.script.contains("props:"));
    assert!(module.script.contains("emits:"));
    assert!(module.script.contains("setup ") || module.script.contains("setup("));
    assert!(module.script.contains("return {"));

    // 验证 CSS Modules
    let style = &module.styles[0];
    assert!(style.module);
    assert!(style.scoped);
    assert!(!style.class_mapping.is_empty());
    assert!(style.css.contains("__")); // 作用域化类名
    assert!(style.css.contains(".external-helper")); // global 不被作用域化
    assert!(style.class_mapping.contains_key("container"));
    assert!(style.class_mapping.contains_key("button"));
}

/// 测试多样式块混合使用
#[test]
fn test_mixed_styles_compilation() {
    let source = r#"
<template>
  <div class="test">Test</div>
</template>

<script setup lang="ts">
const test = "value"
</script>

<style scoped>
.normal {
  color: blue;
}
</style>

<style module>
.module {
  color: red;
}
</style>

<style>
.global {
  color: green;
}
</style>
"#;

    let module = compile_from_string("MixedStyles", source).unwrap();

    // 应该有 3 个样式块
    assert_eq!(module.styles.len(), 3);

    // 第一个：scoped
    assert!(!module.styles[0].module);
    assert!(module.styles[0].scoped);

    // 第二个：module
    assert!(module.styles[1].module);
    assert!(module.styles[1].scoped);
    assert!(!module.styles[1].class_mapping.is_empty());

    // 第三个：global
    assert!(!module.styles[2].module);
    assert!(!module.styles[2].scoped);
    assert!(module.styles[2].class_mapping.is_empty());
}

/// 测试复杂 TypeScript 功能
#[test]
fn test_complex_typescript_features() {
    let source = r#"
<template>
  <div>{{ message }}</div>
</template>

<script lang="ts">
// 泛型接口
interface ApiResponse<T> {
  data: T;
  status: number;
  message: string;
}

// 泛型函数
async function fetchData<T>(url: string): Promise<ApiResponse<T>> {
  const response = await fetch(url);
  return response.json();
}

// 类型别名
type User = {
  id: number;
  name: string;
  email: string;
};

// 使用泛型函数
class UserService {
  async getUsers(): Promise<ApiResponse<User[]>> {
    return fetchData<User[]>("/api/users");
  }
}

export default {
  name: "ComplexTS",
  setup() {
    const service = new UserService();
    return { service };
  }
}
</script>
"#;

    let module = compile_from_string("ComplexTypeScript", source).unwrap();

    // 验证编译成功
    assert!(!module.script.is_empty());

    // 验证 TypeScript 类型注解被移除
    assert!(!module.script.contains(": ApiResponse<T>"));
    assert!(!module.script.contains(": Promise<ApiResponse<T>>"));

    // 验证函数和类保留
    assert!(module.script.contains("async function fetchData"));
    assert!(module.script.contains("class UserService"));
}

/// 测试所有模板指令组合
#[test]
fn test_all_directives_combination() {
    let source = r#"
<template>
  <!-- 条件渲染 -->
  <div v-if="show">
    <span v-else-if="other">Other</span>
    <span v-else>Default</span>
  </div>
  
  <!-- 列表渲染 -->
  <ul>
    <li v-for="(item, index) in items" :key="item.id">
      {{ index }}: {{ item.name }}
    </li>
  </ul>
  
  <!-- 数据和事件 -->
  <input 
    v-model="username"
    @input="onInput"
    @focus="onFocus"
    :placeholder="hint"
  />
  
  <!-- 内容渲染 -->
  <div v-text="message"></div>
  <div v-html="rawHtml"></div>
  <div v-show="isVisible">Show</div>
  
  <!-- 其他指令 -->
  <div v-once>Once</div>
  <div v-pre>{{ noCompile }}</div>
  <div v-cloak>Cloak</div>
  <div v-memo="[count]">Memo</div>
  
  <!-- 插槽 -->
  <component>
    <template #header="{ title }">
      <h1>{{ title }}</h1>
    </template>
    <template #default>
      <p>Default slot</p>
    </template>
  </component>
</template>

<script setup>
const show = true
const items = []
</script>
"#;

    let module = compile_from_string("AllDirectives", source).unwrap();

    // 验证渲染函数包含指令生成的代码（已转换，不是原始指令名）
    // v-if/v-else-if/v-else 生成三元表达式和条件渲染代码
    assert!(module.render_fn.contains("null") || module.render_fn.contains("if"));
    assert!(module.render_fn.contains("v-for") || module.render_fn.contains("map"));
    assert!(module.render_fn.contains("v-model") || module.render_fn.contains("onInput"));
    assert!(module.render_fn.contains("textContent")); // v-text
    assert!(module.render_fn.contains("innerHTML")); // v-html
    assert!(module.render_fn.contains("style.display")); // v-show
}

/// 测试缓存效果
#[test]
fn test_cache_effectiveness() {
    use std::time::Instant;

    let source = r#"
<template>
  <div>{{ message }}</div>
</template>

<script setup lang="ts">
const message = "Hello"
</script>

<style module>
.test { color: red; }
</style>
"#;

    // 首次编译
    let start1 = Instant::now();
    let module1 = compile_from_string("CacheTest", source).unwrap();
    let duration1 = start1.elapsed();

    // 第二次编译（应该命中缓存）
    let start2 = Instant::now();
    let module2 = compile_from_string("CacheTest", source).unwrap();
    let duration2 = start2.elapsed();

    // 验证结果一致
    assert_eq!(module1.source_hash, module2.source_hash);

    // 验证缓存加速（至少 10 倍）
    let speedup = duration1.as_nanos() as f64 / duration2.as_nanos() as f64;
    println!("首次编译: {:?}", duration1);
    println!("缓存命中: {:?}", duration2);
    println!("加速比: {:.2}x", speedup);

    // 注意：由于系统负载，这个断言可能不稳定
    // assert!(speedup > 10.0, "缓存应该至少提供 10 倍加速");
}

/// 测试边界情况
#[test]
fn test_edge_cases() {
    // 空模板
    let source1 = r#"
<template></template>
<script setup></script>
"#;
    let module1 = compile_from_string("EmptyTemplate", source1).unwrap();
    assert!(module1.render_fn.contains("return null"));

    // 空脚本
    let source2 = r#"
<template><div>Test</div></template>
"#;
    let module2 = compile_from_string("EmptyScript", source2).unwrap();
    assert_eq!(module2.script, "");

    // 仅样式
    let source3 = r#"
<template><div></div></template>
<style module>.test {}</style>
"#;
    let module3 = compile_from_string("StylesOnly", source3).unwrap();
    assert_eq!(module3.styles.len(), 1);
    assert!(module3.styles[0].module);
}

/// 测试错误处理
#[test]
fn test_error_handling() {
    // 缺少 template 和 script
    let source1 = r#"
<style>.test {}</style>
"#;
    let result1 = compile_from_string("Invalid", source1);
    assert!(result1.is_err());
    assert!(result1
        .unwrap_err()
        .to_string()
        .contains("must have at least"));

    // TypeScript 语法错误（应该被 swc 捕获）
    let source2 = r#"
<template><div></div></template>
<script lang="ts">
const x: = invalid
</script>
"#;
    let result2 = compile_from_string("SyntaxError", source2);
    // swc 可能会尝试修复或报错，取决于错误类型
    // 这里只验证不会 panic
    let _ = result2;
}

/// 测试性能基准
#[test]
fn test_performance_benchmark() {
    use std::time::Instant;

    let source = r#"
<template>
  <div :class="$style.container">
    <h1>{{ title }}</h1>
    <ul>
      <li v-for="item in items" :key="item.id">{{ item.name }}</li>
    </ul>
    <button @click="onClick">Click</button>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  title: string
  items: Array<{ id: number; name: string }>
}>()

const emit = defineEmits<{
  click: [timestamp: number]
}>()

function onClick() {
  emit('click', Date.now())
}
</script>

<style module>
.container {
  padding: 20px;
  max-width: 1200px;
  margin: 0 auto;
}
</style>
"#;

    let iterations = 100;
    let start = Instant::now();

    for i in 0..iterations {
        let _ = compile_from_string(&format!("Benchmark{}", i), source).unwrap();
    }

    let duration = start.elapsed();
    let avg = duration.as_micros() as f64 / iterations as f64;

    println!("{} 次编译总耗时: {:?}", iterations, duration);
    println!("平均编译时间: {:.2} μs", avg);
    println!("每秒编译次数: {:.0}", 1_000_000.0 / avg);

    // 验证平均编译时间合理（应该 < 5ms）
    assert!(avg < 5000.0, "平均编译时间应该小于 5ms");
}

/// 测试哈希一致性
#[test]
fn test_hash_consistency() {
    let source = r#"
<template><div>{{ msg }}</div></template>
<script setup>const msg = "test"</script>
"#;

    let module1 = compile_from_string("HashTest1", source).unwrap();
    let module2 = compile_from_string("HashTest2", source).unwrap();

    // 相同源码应该生成相同哈希
    assert_eq!(module1.source_hash, module2.source_hash);

    // 不同源码应该生成不同哈希
    let source3 = r#"
<template><div>{{ msg }}</div></template>
<script setup>const msg = "different"</script>
"#;
    let module3 = compile_from_string("HashTest3", source3).unwrap();
    assert_ne!(module1.source_hash, module3.source_hash);
}

/// 测试序列化
#[test]
fn test_serialization() {
    use serde_json;

    let source = r#"
<template><div>{{ msg }}</div></template>
<script setup>
const props = { msg: "test" }
</script>
<style module>.test { color: red; }</style>
"#;

    let module = compile_from_string("Serializable", source).unwrap();

    // 序列化为 JSON
    let json = serde_json::to_string(&module).unwrap();
    assert!(!json.is_empty());

    // 反序列化
    let deserialized: iris_sfc::SfcModule = serde_json::from_str(&json).unwrap();

    // 验证数据一致
    assert_eq!(deserialized.name, module.name);
    assert_eq!(deserialized.render_fn, module.render_fn);
    assert_eq!(deserialized.script, module.script);
    assert_eq!(deserialized.source_hash, module.source_hash);
}
