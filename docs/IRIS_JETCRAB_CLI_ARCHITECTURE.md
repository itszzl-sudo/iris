# Iris JetCrab CLI 架构文档

## 概述

`iris-jetcrab-cli` 是 Vue 项目的开发服务器 CLI 工具，采用**运行时按需编译**架构（方案 B）。

## 架构设计

### 核心原则

1. **iris-jetcrab-engine**: 与 iris-engine 对等的纯编译引擎
   - 不依赖 HTTP/CLI
   - 提供 Vue 项目编译能力
   - 可被任何工具调用

2. **iris-jetcrab-cli**: 调用引擎的 CLI 工具
   - 启动 Web 服务器（axum）
   - 处理 HTTP 请求
   - 调用 engine 编译模块
   - 管理编译缓存

### 数据流

```
浏览器请求 → iris-jetcrab-cli (HTTP服务器) → iris-jetcrab-engine (编译) → 返回结果
```

## 模块结构

```
iris-jetcrab-cli/
├── src/
│   ├── main.rs              # CLI 入口（clap）
│   ├── utils.rs             # 工具函数
│   └── server/
│       ├── mod.rs           # 服务器模块
│       ├── http_server.rs   # HTTP 服务器核心
│       ├── routes.rs        # 路由处理器
│       ├── compiler_cache.rs # 编译缓存管理
│       └── hmr.rs           # HMR（待实现）
```

## 核心功能

### 1. HTTP 服务器（http_server.rs）

- 使用 axum 框架
- 启动在 localhost:3000（可配置）
- 支持 CORS
- 自动打开浏览器

### 2. 路由处理（routes.rs）

| 路由 | 功能 | 说明 |
|------|------|------|
| `GET /` | 主页 | 返回 index.html |
| `GET /@vue/*path` | Vue 模块编译 | 按需编译 Vue 模块 |
| `GET /assets/*path` | 静态资源 | 提供 public/ 目录文件 |
| `GET /api/project-info` | 项目信息 | 返回项目元数据 |
| `GET /@hmr` | HMR WebSocket | 热更新（待实现） |

### 3. 编译缓存（compiler_cache.rs）

**策略**：首次请求时编译整个项目，后续请求使用缓存

```rust
pub struct CompilerCache {
    project_root: PathBuf,
    compiled_modules: Mutex<HashMap<String, CompiledModule>>,
    compilation_result: Mutex<Option<CompilationResult>>,
    is_compiled: Mutex<bool>,
}
```

**工作流程**：
1. 浏览器请求模块 `/@vue/App.vue`
2. 检查缓存是否命中
3. 未命中 → 调用 engine 编译整个项目
4. 缓存编译结果
5. 返回模块数据

### 4. 工具函数（utils.rs）

- `find_project_root`: 查找项目根目录
- `is_vue_project`: 检测 Vue 项目
- `find_entry_file`: 查找入口文件
- `count_vue_files`: 统计 Vue 文件数量

## 技术栈

| 依赖 | 用途 |
|------|------|
| axum 0.7 | HTTP 服务器 |
| tokio | 异步运行时 |
| clap 4.4 | CLI 框架 |
| tower-http | 中间件（CORS） |
| tokio-tungstenite | WebSocket（待使用） |
| notify | 文件监听（待使用） |
| serde_json | JSON 序列化 |
| colored | 终端输出着色 |

## 使用方式

```bash
# 启动开发服务器
iris-jetcrab dev

# 指定端口和项目根目录
iris-jetcrab dev -p 8080 -r /path/to/vue-project

# 自动打开浏览器
iris-jetcrab dev --open

# 查看项目信息
iris-jetcrab info
```

## 编译验证

```bash
# 检查编译
cargo check -p iris-jetcrab-cli

# 构建
cargo build -p iris-jetcrab-cli

# 运行
cargo run -p iris-jetcrab-cli -- dev
```

## 待实现功能

### HMR（热模块替换）- ✅ 已实现

**架构**：
1. ✅ 使用 notify 监听 src 目录文件变化
2. ✅ 防抖处理（300ms 延迟，避免频繁触发）
3. ✅ 使编译缓存失效并重新编译
4. ✅ 通过 WebSocket 推送更新到浏览器

**WebSocket 事件**：
- `connected`: 连接成功
- `file-changed`: 文件变更
- `rebuild-complete`: 重新编译完成
- `compile-error`: 编译错误

**工作流程**：
```
文件修改 → notify 检测 → 300ms 防抖 → 重新编译 → WebSocket 推送 → 浏览器更新
```

### 其他优化

1. 增量编译（只编译变化的模块）
2. 编译进度显示
3. 错误提示优化
4. 性能分析工具

## 与 iris-engine 的对比

| 特性 | iris-engine | iris-jetcrab-engine |
|------|-------------|---------------------|
| 运行时 | Node.js | Rust (WASM) |
| 用途 | 通用 Web 运行时 | Vue 项目专用 |
| 编译 | 无 | Vue SFC 编译 |
| 调用者 | iris-cli | iris-jetcrab-cli |
| 地位 | 核心引擎 | 对等引擎 |

## 架构决策记录

### 为什么选择方案 B（运行时按需编译）？

1. **开发体验**：开发者启动服务器即可看到效果
2. **灵活性**：支持动态模块加载
3. **缓存优化**：编译结果缓存，提升性能
4. **HMR 支持**：便于实现热更新

### 为什么保持 engine 纯净？

1. **职责单一**：engine 只负责编译
2. **可复用性**：可被 CLI、IDE 插件、测试工具调用
3. **可测试性**：纯函数易于测试
4. **对等地位**：与 iris-engine 架构一致
