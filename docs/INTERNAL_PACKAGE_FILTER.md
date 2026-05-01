# 内部包过滤机制

## 概述

Iris JetCrab CLI 的内置 npm 包管理器现在能够**自动识别并过滤 Iris 框架的内部包**，避免从 npm registry 下载这些由框架自身提供的组件。

## 内部包列表

以下包被识别为 Iris 框架的内部组件，**不会**从 npm 下载：

| 包名 | 说明 |
|------|------|
| `iris` | Iris 框架核心 |
| `@irisverse/iris` | Iris CLI npm 包 |
| `iris-runtime` | Iris 运行时（兼容旧名） |
| `iris-core` | 核心库 |
| `iris-gpu` | WebGPU 渲染 |
| `iris-layout` | 布局引擎 |
| `iris-dom` | DOM API |
| `iris-sfc` | SFC 编译器 |
| `iris-cssom` | CSSOM 解析 |
| `iris-jetcrab` | JetCrab JS 引擎 |
| `iris-jetcrab-engine` | Vue 编排引擎 |
| `iris-jetcrab-cli` | CLI 工具 |

## 过滤位置

### 1. package.json 解析时过滤

**文件**: `vue_compiler.rs` → `load_package_dependencies()`

```rust
fn load_package_dependencies(project_root: &Path) -> Result<HashMap<String, String>> {
    // 解析 package.json
    if let Some(deps) = package_json.get("dependencies").and_then(|v| v.as_object()) {
        for (name, version) in deps {
            // 跳过内部包
            if crate::npm_downloader::NpmDownloader::is_internal_package(name) {
                debug!("Skipping internal package in package.json: {}", name);
                continue;  // ← 过滤
            }
            dependencies.insert(name.clone(), version.as_str().unwrap_or("").to_string());
        }
    }
    // ...
}
```

**效果**: 即使 package.json 中声明了 iris 相关包，也不会被加入依赖列表。

### 2. npm 包解析时过滤

**文件**: `vue_compiler.rs` → `resolve_npm_package()`

```rust
fn resolve_npm_package(&mut self, package_name: &str) -> Result<()> {
    // 检查是否为内部包
    if crate::npm_downloader::NpmDownloader::is_internal_package(package_name) {
        debug!("Skipping internal package: {} (provided by framework)", package_name);
        return Ok(()); // ← 直接返回，不解析
    }
    
    // 继续正常的 npm 包解析流程...
}
```

**效果**: 编译过程中遇到 iris 包的 import 语句时，不会尝试下载。

### 3. 下载器层面过滤

**文件**: `npm_downloader.rs` → `download_and_install()`

```rust
pub fn download_and_install(&self, package_name: &str, version: Option<&str>) -> Result<PathBuf> {
    // 检查是否为内部包
    if Self::is_internal_package(package_name) {
        debug!("Skipping internal package: {}", package_name);
        return Err(anyhow::anyhow!(
            "Package '{}' is an internal Iris package, skipping download",
            package_name
        )); // ← 拒绝下载
    }
    
    // 继续正常的下载流程...
}
```

**效果**: 即使代码尝试下载内部包，也会被拦截。

## 使用示例

### 场景 1: package.json 包含 iris 包

```json
{
  "name": "my-vue-app",
  "version": "1.0.0",
  "dependencies": {
    "vue": "^3.5.0",
    "iris": "^0.1.0",
    "iris-runtime": "^0.1.0",
    "pinia": "^2.1.0"
  }
}
```

**处理结果**:
```
Loaded 2 dependencies from package.json (excluding internal packages)
  ✅ vue@^3.5.0    (外部包，会下载)
  ❌ iris          (内部包，已过滤)
  ❌ iris-runtime  (内部包，已过滤)
  ✅ pinia@^2.1.0  (外部包，会下载)
```

### 场景 2: 代码中 import iris 包

```typescript
// main.ts
import { createApp } from 'vue'
import { init } from '@irisverse/iris'  // ← 内部包

createApp(App).mount('#app')
```

**处理结果**:
```
Resolving npm package: vue
  ✅ Downloading vue@3.5.33...

Resolving npm package: iris-runtime
  ⏭️  Skipping internal package: iris-runtime (provided by framework)
```

## API 文档

### `NpmDownloader::is_internal_package()`

```rust
/// 检查是否为内部包（不需要从 npm 下载）
pub fn is_internal_package(package_name: &str) -> bool
```

**参数**:
- `package_name`: 包名

**返回**:
- `true`: 是内部包，不应下载
- `false`: 是外部包，可以下载

**示例**:
```rust
use iris_jetcrab_engine::NpmDownloader;

assert!(NpmDownloader::is_internal_package("iris"));
assert!(NpmDownloader::is_internal_package("iris-runtime"));
assert!(!NpmDownloader::is_internal_package("vue"));
assert!(!NpmDownloader::is_internal_package("pinia"));
```

## 测试用例

### 测试 1: 内部包识别

```rust
#[test]
fn test_internal_package_detection() {
    // 所有 iris 相关包都应该被识别
    assert!(NpmDownloader::is_internal_package("iris"));
    assert!(NpmDownloader::is_internal_package("iris-runtime"));
    assert!(NpmDownloader::is_internal_package("iris-core"));
    // ... 其他内部包

    // 外部包不应该被识别
    assert!(!NpmDownloader::is_internal_package("vue"));
    assert!(!NpmDownloader::is_internal_package("pinia"));
}
```

**结果**: ✅ 通过

### 测试 2: 下载内部包应该失败

```rust
#[test]
fn test_download_internal_package_should_fail() {
    let downloader = NpmDownloader::new(temp_dir);
    
    let result = downloader.download_and_install("iris", None);
    assert!(result.is_err(), "Downloading internal package should fail");
    
    let result = downloader.download_and_install("iris-runtime", None);
    assert!(result.is_err(), "Downloading iris-runtime should fail");
}
```

**结果**: ✅ 通过

### 测试 3: package.json 过滤

```rust
#[test]
fn test_package_json_filtering() {
    let package_json = r#"{
        "dependencies": {
            "vue": "^3.5.0",
            "iris": "^0.1.0",
            "iris-runtime": "^0.1.0",
            "pinia": "^2.1.0"
        }
    }"#;

    // 解析并统计外部包数量
    let external_count = count_external_packages(package_json);
    
    // 应该只有 vue 和 pinia 两个外部包
    assert_eq!(external_count, 2);
}
```

**结果**: ✅ 通过

## 设计原理

### 为什么需要过滤？

1. **避免重复下载**: Iris 框架已经内置这些组件，不需要从 npm 下载
2. **版本一致性**: 确保使用的 iris 包版本与框架版本一致
3. **减少编译时间**: 跳过不必要的下载和解压操作
4. **避免冲突**: 防止 npm 上的 iris 包与框架内置包冲突

### 过滤层次

```
Layer 1: package.json 解析
  └─> 过滤内部包，不加入依赖列表
  
Layer 2: npm 包解析
  └─> 遇到内部包直接跳过
  
Layer 3: 下载器
  └─> 拒绝下载内部包（安全网）
```

三层过滤确保内部包**绝对不会**被下载。

## 日志输出

### 正常情况（过滤内部包）

```
2026-04-29T10:09:25.815132Z DEBUG iris_jetcrab_engine::vue_compiler: 
  Skipping internal package in package.json: iris

2026-04-29T10:09:25.815232Z DEBUG iris_jetcrab_engine::vue_compiler: 
  Skipping internal package in package.json: iris-runtime

2026-04-29T10:09:25.815332Z INFO iris_jetcrab_engine::vue_compiler: 
  Loaded 2 dependencies from package.json (excluding internal packages)
```

### 编译时遇到内部包

```
2026-04-29T10:09:26.123456Z DEBUG iris_jetcrab_engine::vue_compiler: 
  Resolving npm package: vue

2026-04-29T10:09:26.234567Z INFO iris_jetcrab_engine::npm_downloader: 
  Downloading npm package: vue@3.5.33

2026-04-29T10:09:26.345678Z DEBUG iris_jetcrab_engine::vue_compiler: 
  Resolving npm package: iris-runtime

2026-04-29T10:09:26.345778Z DEBUG iris_jetcrab_engine::vue_compiler: 
  Skipping internal package: iris-runtime (provided by framework)
```

## 维护内部包列表

### 添加新的内部包

如果未来添加了新的 iris 组件，需要在 `npm_downloader.rs` 中更新列表：

```rust
const INTERNAL_PACKAGES: &[&str] = &[
    "iris",
    "iris-runtime",
    // ... 现有包
    "iris-new-component",  // ← 添加新包
];
```

### 命名规范

所有 Iris 框架的内部包都以 `iris` 或 `iris-` 开头：
- `iris` - 核心框架
- `iris-*` - 框架组件

这种命名约定使得过滤逻辑简单且可靠。

## 常见问题

### Q: 如果我真的想从 npm 下载 iris 包怎么办？

**A**: 目前不支持这个场景。Iris 框架的内部包由框架自身管理，不应该从 npm 获取。

### Q: 过滤会影响性能吗？

**A**: 不会。过滤只是一个简单的字符串比较（`contains`），性能开销可以忽略不计。

### Q: 如何知道哪些包被过滤了？

**A**: 查看调试日志（`RUST_LOG=debug`），会显示所有被过滤的内部包。

### Q: 内部包列表会变化吗？

**A**: 可能会随着框架发展而增加新组件，但已有的包不会移除。

## 相关文件

- `crates/iris-jetcrab-engine/src/npm_downloader.rs` - 内部包列表和检测逻辑
- `crates/iris-jetcrab-engine/src/vue_compiler.rs` - package.json 解析和包解析过滤
- `crates/iris-jetcrab-engine/tests/internal_package_filter_test.rs` - 单元测试

## 总结

通过**三层过滤机制**，Iris JetCrab CLI 确保：

✅ 内部包不会从 npm 下载  
✅ package.json 中的内部包声明被忽略  
✅ 编译时遇到内部包自动跳过  
✅ 外部包（vue、pinia 等）正常下载  

**框架内置组件与外部依赖完美分离！** 🎯
