# Phase 6 script_setup.rs 代码审查报告

**审查日期**: 2026-02-24  
**审查人员**: Manual Review  
**文件路径**: `crates/iris-sfc/src/script_setup.rs`  
**代码行数**: 877 行  
**发现问题**: 5 个（1 严重 + 2 警告 + 2 建议）  
**修复状态**: 🔄 待修复

---

## 🔴 严重问题

### 1. withDefaults 处理优先级错误导致重复解析

**问题编号**: CRITICAL-001  
**位置**: L274-L285  
**严重程度**: 🔴 严重

#### 问题描述

`withDefaults` 的解析在 `defineProps` 之后（L274），但使用了独立的 `if` 而非 `else if`。这导致：

1. 如果代码中同时有 `defineProps` 和 `withDefaults`，会先执行 L228-272 的 defineProps 解析
2. 然后再执行 L274-285 的 withDefaults 解析，**覆盖**前面的结果
3. 虽然最终结果正确，但会产生不必要的中间处理和日志

**问题代码**:
```rust
// L228-272: 先解析 defineProps（多种格式）
if let Some(caps) = PROPS_TYPE_FULL_RE.captures(script) {
    result.props = Some(runtime_props);
    // ...
}
else if let Some(caps) = PROPS_ARRAY_RE.captures(script) {
    // ...
}

// L274-285: 然后又解析 withDefaults（覆盖前面的结果）
if let Some(caps) = WITH_DEFAULTS_RE.captures(script) {  // ❌ 应该是 else if
    result.props = Some(runtime_props);  // 覆盖
    // ...
}
```

#### 影响范围

- **性能**: 不必要的中间处理
- **逻辑**: 虽然最终结果正确，但代码意图不清晰
- **维护性**: 容易误导后续开发者

#### 修复建议

```rust
// 方案 1: 先检查 withDefaults（优先级更高）
if let Some(caps) = WITH_DEFAULTS_RE.captures(script) {
    // 处理 withDefaults
} else if let Some(caps) = PROPS_TYPE_FULL_RE.captures(script) {
    // 处理普通 defineProps
}

// 方案 2: 使用标志位
let mut props_parsed = false;
if !props_parsed && let Some(caps) = WITH_DEFAULTS_RE.captures(script) {
    // ...
    props_parsed = true;
}
```

---

## 🟡 警告问题

### 2. 正则表达式无法处理复杂的 TypeScript 类型

**问题编号**: WARNING-001  
**位置**: L99-L100  
**严重程度**: 🟡 边缘情况

#### 问题描述

`PROPS_TYPE_FULL_RE` 使用 `\{([^}]+)\}` 匹配 TypeScript 接口，但无法处理：

1. **嵌套类型**: `{ items: Array<{ id: number }> }`
2. **联合类型**: `{ status: 'active' | 'inactive' }`
3. **交叉类型**: `{ a: A } & { b: B }`
4. **多行注释**: `{ /* comment */ name: string }`

**失败示例**:
```typescript
const props = defineProps<{
  items: Array<{ id: number }>  // ❌ 正则会在第一个 } 处停止
}>()
```

#### 影响范围

- **功能**: 复杂类型定义无法正确解析
- **用户体验**: 开发者需要使用简化类型或运行时 props

#### 修复建议

使用更智能的括号匹配：

```rust
fn extract_ts_interface(text: &str, start: usize) -> Option<(String, usize)> {
    let mut depth = 0;
    let mut result = String::new();
    let chars: Vec<char> = text[start..].chars().collect();
    
    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            '{' => {
                depth += 1;
                if depth > 1 {
                    result.push(ch);
                }
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((result, start + i + 1));
                }
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    None
}
```

---

### 3. extract_top_level_declarations 对解构赋值支持不完整

**问题编号**: WARNING-002  
**位置**: L462-L495  
**严重程度**: 🟡 功能缺陷

#### 问题描述

虽然代码试图排除解构赋值（L473: `!name.starts_with('{')`），但以下场景仍会失败：

```javascript
// ❌ 会被错误提取
const { a, b } = obj  // name = "{ a, b }"，但检查在 find 之后
const [first, second] = arr  // 类似的问题

// ✅ 正确排除
function destruct({ a, b }) {}  // 参数解构，不应提取
```

**问题逻辑**:
```rust
if line.starts_with("const ") {
    if let Some(eq_pos) = line.find(|c: char| c == '=' || c == '(') {
        let name = line[6..eq_pos].trim();
        // ❌ 这里 name 可能是 "{ a, b }"
        if !name.starts_with('{') && !name.starts_with('[') {
            vars.push(name.to_string());
        }
    }
}
```

#### 修复建议

```rust
fn extract_top_level_declarations(script: &str) -> Vec<String> {
    let mut vars = Vec::new();
    
    for line in script.lines() {
        let line = line.trim();
        
        if line.starts_with("const ") || line.starts_with("let ") {
            // 提取变量名部分
            let prefix = if line.starts_with("const ") { 6 } else { 4 };
            if let Some(eq_pos) = line[prefix..].find('=') {
                let name_part = line[prefix..prefix + eq_pos].trim();
                
                // 跳过解构赋值
                if !name_part.starts_with('{') && !name_part.starts_with('[') {
                    // 处理多个变量：const a = 1, b = 2
                    for name in name_part.split(',') {
                        let name = name.trim();
                        if !name.is_empty() {
                            vars.push(name.to_string());
                        }
                    }
                }
            }
        } else if line.starts_with("function ") {
            // ...
        }
    }
    
    vars
}
```

---

## 🔵 建议

### 4. parse_props_interface_with_defaults 缺少 required 字段

**问题编号**: INFO-001  
**位置**: L380-L420  
**严重程度**: 🔵 优化建议

#### 问题描述

`parse_props_interface_with_defaults` 生成的 props 定义中缺少 `required` 字段：

```rust
// 当前输出
{ title: { type: String, default: 'Title' } }

// 应该输出
{ title: { type: String, required: false, default: 'Title' } }
```

虽然 Vue 可以推断（有 default 就不是 required），但显式声明更清晰。

#### 修复建议

```rust
let mut prop_def = format!("    {}: {{ type: {}", name, js_type);

if has_default {
    prop_def.push_str(&format!(", default: {}", defaults_map[name]));
    prop_def.push_str(", required: false");  // ✅ 添加
} else {
    prop_def.push_str(", required: true");   // ✅ 添加
}

prop_def.push_str(" }");
```

---

### 5. 缺少对 defineExpose 的完整支持

**问题编号**: INFO-002  
**位置**: 整体架构  
**严重程度**: 🔵 功能缺失

#### 问题描述

代码注释中提到支持 `defineExpose()`（L6），但实际实现中没有相关解析逻辑。

**Vue 3 语法**:
```typescript
defineExpose({
  count,
  increment
})
```

当前依赖 `extract_top_level_declarations` 自动推断暴露的变量，但这不够精确。

#### 修复建议

添加 `defineExpose` 解析：

```rust
static DEFINE_EXPOSE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"defineExpose\(\{([^}]+)\}\)"#).unwrap()
});

fn parse_define_expose(script: &str) -> Option<Vec<String>> {
    DEFINE_EXPOSE_RE.captures(script).map(|caps| {
        caps[1].split(',')
            .map(|s| s.trim().to_string())
            .collect()
    })
}
```

---

## 📊 代码亮点

1. ✅ **良好的文档**: 模块级文档清晰，包含完整的转换示例
2. ✅ **多种语法支持**: 支持 TypeScript 泛型、数组、withDefaults 等
3. ✅ **LazyLock 优化**: 所有正则表达式使用 LazyLock 避免重复编译
4. ✅ **错误处理**: 使用 `Result` 类型正确传播错误
5. ✅ **测试覆盖**: 15 个测试覆盖主要功能场景
6. ✅ **类型映射**: `map_ts_to_js_type` 覆盖常用 TypeScript 类型

---

## 📈 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 功能完整性 | ⭐⭐⭐⭐☆ | 支持主要语法，缺少 defineExpose 完整实现 |
| 代码质量 | ⭐⭐⭐⭐☆ | 整体良好，有边缘情况未处理 |
| 测试质量 | ⭐⭐⭐⭐☆ | 15 个测试覆盖主要场景，可增加边界测试 |
| 可维护性 | ⭐⭐⭐⭐⭐ | 结构清晰，命名规范 |
| 性能 | ⭐⭐⭐⭐☆ | LazyLock 优化良好，正则匹配可改进 |

**总体评分**: ⭐⭐⭐⭐☆ (4/5)

---

## 🎯 修复优先级

| 优先级 | 问题 | 预计工作量 |
|--------|------|-----------|
| 🔴 高 | withDefaults 优先级优化 | 15 分钟 |
| 🟡 中 | 复杂 TypeScript 类型支持 | 2-3 小时 |
| 🟡 中 | 解构赋值处理改进 | 30 分钟 |
| 🔵 低 | required 字段补充 | 10 分钟 |
| 🔵 低 | defineExpose 支持 | 1 小时 |

---

## 📝 总结

`script_setup.rs` 模块整体质量良好，架构清晰，测试覆盖充分。发现的主要问题集中在**边缘情况处理**和**复杂类型支持**上。建议：

1. **立即修复**: withDefaults 优先级问题（简单且影响代码清晰度）
2. **短期优化**: 补充 required 字段、改进解构赋值处理
3. **中期改进**: 支持复杂 TypeScript 类型（嵌套、联合、交叉）
4. **长期规划**: 完整 defineExpose 支持、类型推断增强

---

**审查完成日期**: 2026-02-24  
**修复状态**: 🔄 待修复  
**下一步**: 修复关键问题并更新 REVIEW_LOG.md
