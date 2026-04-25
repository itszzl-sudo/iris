# Iris SFC 环境变量配置

本文档说明 Iris SFC 支持的所有环境变量配置。

## 📋 环境变量列表

### 1. IRIS_SOURCE_MAP

**用途**: 控制是否生成 Source Map

**类型**: Boolean

**可选值**:
- `true`, `1`, `yes` - 启用 Source Map
- `false`, `0`, `no`, 或未设置 - 禁用 Source Map（默认）

**默认值**: `false`（禁用）

**示例**:
```bash
# 启用 Source Map（用于浏览器调试）
IRIS_SOURCE_MAP=true cargo run

# 禁用 Source Map（默认，节省内存）
IRIS_SOURCE_MAP=false cargo run
```

**影响**:
- ✅ **启用**: 
  - 生成 Source Map 文件
  - 支持浏览器 DevTools 调试
  - 支持 Sentry 错误监控
  - 内存占用 +30-50%
  - 编译时间 +10-15%

- ❌ **禁用**（推荐）:
  - 不生成 Source Map
  - 节省内存和编译时间
  - 适合开发阶段和内部工具

---

### 2. IRIS_CACHE_CAPACITY

**用途**: 设置 SFC 编译缓存的最大容量

**类型**: Integer

**范围**: 1 - 10000

**默认值**: `100`

**示例**:
```bash
# 缓存 200 个组件
IRIS_CACHE_CAPACITY=200 cargo run

# 缓存 50 个组件（节省内存）
IRIS_CACHE_CAPACITY=50 cargo run

# 缓存 1000 个组件（大型项目）
IRIS_CACHE_CAPACITY=1000 cargo run
```

**影响**:
- 缓存容量越大，命中率越高
- 但内存占用也越大
- 建议根据项目组件数量调整

**内存估算**:
```
每个缓存项: ~5-10 KB
100 项: ~500 KB - 1 MB
200 项: ~1-2 MB
1000 项: ~5-10 MB
```

---

### 3. IRIS_CACHE_ENABLED

**用途**: 启用或禁用 SFC 编译缓存

**类型**: Boolean

**可选值**:
- `true`, `1`, `yes`, 或未设置 - 启用缓存（默认）
- `false`, `0`, `no` - 禁用缓存

**默认值**: `true`（启用）

**示例**:
```bash
# 启用缓存（默认，推荐）
IRIS_CACHE_ENABLED=true cargo run

# 禁用缓存（调试用途）
IRIS_CACHE_ENABLED=false cargo run
```

**影响**:
- ✅ **启用**（推荐）:
  - 首次编译: 5-10 ms
  - 缓存命中: <3 μs
  - 性能提升: 1000-3000 倍
  - 适合热重载场景

- ❌ **禁用**:
  - 每次编译: 5-10 ms
  - 无缓存开销
  - 适合调试编译问题

---

## 🎯 使用场景

### 场景 1: 日常开发（推荐配置）

```bash
# 使用默认配置
cargo run

# 或显式配置
IRIS_SOURCE_MAP=false \
IRIS_CACHE_CAPACITY=100 \
IRIS_CACHE_ENABLED=true \
cargo run
```

**特点**:
- 快速编译
- 低内存占用
- 适合大部分开发场景

---

### 场景 2: 浏览器调试

```bash
# 启用 Source Map，增大缓存
IRIS_SOURCE_MAP=true \
IRIS_CACHE_CAPACITY=200 \
IRIS_CACHE_ENABLED=true \
cargo run
```

**特点**:
- 支持浏览器 DevTools 调试
- 显示原始 .vue 源码
- 准确的错误行号

---

### 场景 3: 大型项目

```bash
# 增大缓存容量
IRIS_SOURCE_MAP=false \
IRIS_CACHE_CAPACITY=500 \
IRIS_CACHE_ENABLED=true \
cargo run
```

**特点**:
- 缓存更多组件
- 提高热重载命中率
- 内存占用 ~2.5-5 MB

---

### 场景 4: 调试编译问题

```bash
# 禁用缓存，每次重新编译
IRIS_SOURCE_MAP=false \
IRIS_CACHE_ENABLED=false \
cargo run
```

**特点**:
- 确保每次都是全新编译
- 排除缓存相关问题
- 适合排查编译错误

---

### 场景 5: 生产构建

```bash
# 禁用缓存和 Source Map
IRIS_SOURCE_MAP=false \
IRIS_CACHE_ENABLED=false \
cargo build --release
```

**特点**:
- 最小化内存占用
- 无运行时缓存开销
- 适合生产部署

---

## 📊 配置组合推荐

| 场景 | SOURCE_MAP | CACHE_CAPACITY | CACHE_ENABLED | 内存占用 | 性能 |
|------|-----------|---------------|---------------|---------|------|
| **日常开发** | false | 100 | true | ~1 MB | ⭐⭐⭐⭐⭐ |
| **浏览器调试** | true | 200 | true | ~2 MB | ⭐⭐⭐⭐ |
| **大型项目** | false | 500 | true | ~3 MB | ⭐⭐⭐⭐⭐ |
| **问题调试** | false | 100 | false | ~0 MB | ⭐⭐⭐ |
| **生产构建** | false | 0 | false | ~0 MB | ⭐⭐⭐⭐⭐ |

---

## 🔧 PowerShell 配置

### Windows PowerShell

```powershell
# 设置环境变量
$env:IRIS_SOURCE_MAP = "true"
$env:IRIS_CACHE_CAPACITY = "200"
$env:IRIS_CACHE_ENABLED = "true"

# 运行项目
cargo run

# 清除环境变量
Remove-Item Env:\IRIS_SOURCE_MAP
Remove-Item Env:\IRIS_CACHE_CAPACITY
Remove-Item Env:\IRIS_CACHE_ENABLED
```

### 一次性设置（推荐）

```powershell
# 单行命令
$env:IRIS_SOURCE_MAP="true"; $env:IRIS_CACHE_CAPACITY="200"; cargo run
```

---

## 🐛 调试技巧

### 查看缓存统计

在代码中添加：

```rust
use iris_sfc::SFC_CACHE;

// 打印缓存统计
SFC_CACHE.log_stats();
```

**输出示例**:
```
INFO Cache statistics:
  hits: 45
  misses: 10
  compilations: 10
  evictions: 2
  hit_rate: 81.82%
  cache_size: 10
  cache_capacity: 100
```

### 验证 Source Map

启用后检查编译输出：

```rust
let result = TS_COMPILER.compile(source, "test.ts")?;
println!("Source map: {:?}", result.source_map);
```

---

## ⚠️ 注意事项

1. **环境变量只在启动时读取**
   - 修改环境变量需要重启程序
   - 运行时修改不会生效

2. **缓存容量限制**
   - 设置过小会导致频繁淘汰
   - 设置过大会占用过多内存
   - 建议根据实际组件数量调整

3. **Source Map 性能影响**
   - 启用后会增加内存和编译时间
   - 仅在需要调试时启用
   - 生产环境建议禁用

4. **缓存键基于内容哈希**
   - 相同内容的组件只编译一次
   - 内容变化自动失效
   - 不依赖文件路径

---

## 📝 完整示例

### Rust 代码中使用

```rust
use iris_sfc::{compile, SFC_CACHE};

fn main() {
    // 编译组件（自动使用缓存）
    let module = compile("App.vue").unwrap();
    
    // 查看缓存统计
    let stats = SFC_CACHE.stats();
    println!("缓存命中率: {:.2}%", stats.hit_rate() * 100.0);
    
    // 打印详细统计
    SFC_CACHE.log_stats();
}
```

### 命令行使用

```bash
# 开发环境
IRIS_CACHE_ENABLED=true IRIS_CACHE_CAPACITY=100 cargo run

# 调试环境
IRIS_SOURCE_MAP=true IRIS_CACHE_ENABLED=false cargo run

# 生产环境
IRIS_CACHE_ENABLED=false IRIS_SOURCE_MAP=false cargo build --release
```

---

## 🔗 相关文档

- [SFC 热重载缓存机制设计](./SFC-CACHE-DESIGN.md)
- [Source Map 用途评估](./SOURCE-MAP-EVALUATION.md)
- [性能优化指南](./CARGO-PERFORMANCE-OPTIMIZATION.md)

---

**更新日期**: 2026-04-24  
**版本**: Iris SFC v0.0.1
