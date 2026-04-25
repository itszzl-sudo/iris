//! Iris SFC 与 iris-app 集成示例
//! 
//! 演示如何在 Iris 应用中使用 SFC 编译器

use iris_sfc::{compile_from_string, SfcModule};

fn main() {
    println!("🚀 Iris SFC Integration Example");
    println!("================================\n");
    
    // 示例 1: 完整功能组件
    println!("📦 Compiling Full Featured Component...");
    let full = create_full_featured_component();
    println!("✅ Name: {}", full.name);
    println!("✅ Render function: {} bytes", full.render_fn.len());
    println!("✅ Script: {} bytes", full.script.len());
    println!("✅ Styles: {} blocks", full.styles.len());
    if !full.styles.is_empty() {
        let style = &full.styles[0];
        println!("   - CSS Modules: {}", style.module);
        println!("   - Scoped: {}", style.scoped);
        println!("   - Class mappings: {}", style.class_mapping.len());
    }
    println!();
    
    // 示例 2: 计数器组件
    println!("📦 Compiling Counter Component...");
    let counter = create_counter_component();
    println!("✅ Name: {}", counter.name);
    println!("✅ Render function: {} bytes", counter.render_fn.len());
    println!("✅ Styles: {} blocks", counter.styles.len());
    println!();
    
    // 示例 3: TypeScript 组件
    println!("📦 Compiling TypeScript Component...");
    let ts = create_typescript_component();
    println!("✅ Name: {}", ts.name);
    println!("✅ Script compiled: {} bytes", ts.script.len());
    println!("✅ Type annotations removed: {}", !ts.script.contains(": User"));
    println!();
    
    println!("✨ All components compiled successfully!");
}

/// 示例：完整的 Vue 3 组件
/// 包含所有 Iris SFC 支持的特性
pub fn create_full_featured_component() -> SfcModule {
    let vue_source = r#"
<template>
  <div :class="$style.app">
    <h1 v-text="title"></h1>
    <p>Count: {{ count }}</p>
    <button 
      :class="$style.button"
      @click="increment"
      :disabled="count >= maxCount"
    >
      Increment
    </button>
    <div v-show="showDetails" v-html="details"></div>
  </div>
</template>

<script setup>
const props = defineProps(['title', 'maxCount'])
const emit = defineEmits(['update', 'max-reached'])

const count = 0
const showDetails = true
const details = '<strong>Details</strong>'

function increment() {
  if (count < props.maxCount) {
    emit('update', count + 1)
  }
}
</script>

<style module>
.app {
  font-family: Arial, sans-serif;
  padding: 20px;
  max-width: 800px;
  margin: 0 auto;
}

.button {
  padding: 10px 20px;
  background: #4CAF50;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.button:disabled {
  background: #ccc;
  cursor: not-allowed;
}

:global(.external-theme) {
  --primary-color: #4CAF50;
}
</style>
"#;

    compile_from_string("FullFeaturedComponent", vue_source)
        .expect("Failed to compile component")
}

/// 示例：简单的计数器组件
pub fn create_counter_component() -> SfcModule {
    let vue_source = r#"
<template>
  <div>
    <h2>{{ title }}</h2>
    <p>Count: {{ count }}</p>
    <button @click="count++">+</button>
    <button @click="count--">-</button>
    <button @click="reset">Reset</button>
  </div>
</template>

<script setup>
const props = defineProps(['title', 'initialCount'])
const count = props.initialCount || 0

function reset() {
  count = props.initialCount || 0
}
</script>

<style scoped>
div {
  padding: 20px;
  border: 1px solid #ddd;
  border-radius: 8px;
}

button {
  margin: 0 5px;
  padding: 5px 15px;
}
</style>
"#;

    compile_from_string("CounterComponent", vue_source)
        .expect("Failed to compile counter")
}

/// 示例：TypeScript 组件
pub fn create_typescript_component() -> SfcModule {
    let vue_source = r#"
<template>
  <div>
    <h1>{{ user.name }}</h1>
    <p>Email: {{ user.email }}</p>
    <p>Age: {{ user.age }}</p>
  </div>
</template>

<script lang="ts">
interface User {
  name: string;
  email: string;
  age: number;
}

function formatUser(user: User): string {
  return `${user.name} (${user.email})`;
}

export default {
  name: "UserCard",
  setup() {
    const user: User = {
      name: "Iris User",
      email: "user@iris.dev",
      age: 1
    };
    
    return { user, formatUser };
  }
}
</script>
"#;

    compile_from_string("TypeScriptComponent", vue_source)
        .expect("Failed to compile TypeScript component")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_featured_component() {
        let module = create_full_featured_component();
        
        assert_eq!(module.name, "FullFeaturedComponent");
        assert!(!module.render_fn.is_empty());
        assert!(!module.script.is_empty());
        assert_eq!(module.styles.len(), 1);
        
        // 验证 CSS Modules
        let style = &module.styles[0];
        assert!(style.module);
        assert!(!style.class_mapping.is_empty());
        assert!(style.css.contains("__")); // 作用域化类名
    }

    #[test]
    fn test_counter_component() {
        let module = create_counter_component();
        
        assert_eq!(module.name, "CounterComponent");
        assert!(!module.render_fn.is_empty());
        assert!(!module.script.is_empty());
        assert_eq!(module.styles.len(), 1);
        
        // 验证 scoped 样式
        let style = &module.styles[0];
        assert!(style.scoped);
        assert!(!style.module);
    }

    #[test]
    fn test_typescript_component() {
        let module = create_typescript_component();
        
        assert_eq!(module.name, "TypeScriptComponent");
        assert!(!module.render_fn.is_empty());
        assert!(!module.script.is_empty());
        
        // 验证 TypeScript 被编译
        // 类型注解应该被移除
        assert!(!module.script.contains(": User"));
        assert!(!module.script.contains(": string"));
    }

    #[test]
    fn test_component_serialization() {
        let module = create_full_featured_component();
        
        // 序列化为 JSON（使用有 CSS Modules 的组件）
        let json = serde_json::to_string(&module).unwrap();
        assert!(!json.is_empty());
        
        // 反序列化
        let deserialized: SfcModule = serde_json::from_str(&json).unwrap();
        
        // 验证数据一致
        assert_eq!(deserialized.name, module.name);
        assert_eq!(deserialized.render_fn, module.render_fn);
        assert_eq!(deserialized.script, module.script);
        assert_eq!(deserialized.styles.len(), module.styles.len());
    }
}
