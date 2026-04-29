# npm 包自动下载功能

## 概述

Iris JetCrab CLI 现在内置了 **npm 包下载器**，无需安装任何外部工具（npm、yarn、pnpm 等），即可自动下载并安装 npm 包。

## 工作原理

```
Vue 项目编译
    ↓
检测到 npm 包依赖（如 vue）
    ↓
检查 node_modules 中是否存在
    ↓
如果不存在 → 自动从 npm registry 下载
    ↓
解压并安装到 node_modules
    ↓
继续编译
```

## 特性

### ✅ 核心功能

- **零外部依赖**：不需要 npm、yarn、pnpm 等工具
- **自动下载**：编译时自动检测并下载缺失的包
- **版本管理**：从 package.json 读取版本号
- **Scoped Packages**：支持 `@vue/runtime-core` 等 scoped 包
- **缓存机制**：已安装的包不会重复下载
- **批量下载**：支持同时下载多个包

### 📦 技术实现

- **HTTP 客户端**：使用 `ureq`（轻量级同步 HTTP 客户端）
- **Tarball 解压**：使用 `flate2` + `tar`（纯 Rust 实现）
- **npm Registry**：直接从 `https://registry.npmjs.org` 下载

## 使用示例

### 1. 创建 Vue 项目（无需 node_modules）

```bash
# 创建项目目录
mkdir my-vue-app
cd my-vue-app

# 创建 package.json（声明依赖但不安装）
cat > package.json << 'EOF'
{
  "name": "my-vue-app",
  "version": "1.0.0",
  "dependencies": {
    "vue": "^3.5.0",
    "pinia": "^2.1.0"
  }
}
EOF

# 创建 main.ts
cat > src/main.ts << 'EOF'
import { createApp } from 'vue'
import App from './App.vue'

createApp(App).mount('#app')
EOF

# 创建 App.vue
cat > src/App.vue << 'EOF'
<template>
  <div>
    <h1>Hello Vue!</h1>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const message = ref('Hello from Vue 3!')
</script>
```

### 2. 启动开发服务器

```bash
iris-jetcrab dev --root ./my-vue-app
```

### 3. 观察自动下载

服务器启动时会看到如下日志：

```
🦀 Iris JetCrab CLI
Vue Development Server (Runtime On-Demand Compilation)

📁 Project: ./my-vue-app
✅ Vue: Project detected

📦 Downloading npm package: vue@3.5.33
📦 Downloading npm package: pinia@2.1.7

✅ Installed npm package: vue@3.5.33 -> ./my-vue-app/node_modules/vue
✅ Installed npm package: pinia@2.1.7 -> ./my-vue-app/node_modules/pinia

🌐 Server: http://localhost:3000
✨ Ready!
```

## API 文档

### NpmDownloader

```rust
use iris_jetcrab_engine::NpmDownloader;
use std::path::PathBuf;

// 创建下载器
let downloader = NpmDownloader::new(
    PathBuf::from("./node_modules")
);

// 下载单个包
let path = downloader.download_and_install("vue", Some("3.5.33"))?;

// 下载 latest 版本
let path = downloader.download_and_install("vue", None)?;

// 检查是否已安装
if downloader.is_package_installed("vue") {
    println!("Vue is already installed");
}

// 批量下载
let packages = vec![
    ("vue".to_string(), Some("3.5.33".to_string())),
    ("pinia".to_string(), None), // latest
];
let paths = downloader.download_multiple(&packages)?;
```

## 配置

### 版本号解析

1. **指定版本**：`"vue": "3.5.33"` → 下载 3.5.33
2. **范围版本**：`"vue": "^3.5.0"` → 下载 latest（兼容 3.5.x）
3. **未指定**：下载 latest

### 缓存位置

下载的包安装到项目的 `node_modules/` 目录：

```
my-vue-app/
├── node_modules/
│   ├── vue/
│   │   ├── package.json
│   │   ├── dist/
│   │   └── ...
│   └── pinia/
│       ├── package.json
│       └── ...
├── src/
└── package.json
```

## 优势

### vs npm install

| 特性 | npm install | Iris 内置下载器 |
|------|-------------|----------------|
| 外部依赖 | 需要 Node.js + npm | ❌ 无需任何工具 |
| 下载速度 | 快（并行） | 中（串行） |
| 依赖解析 | 完整（peer deps 等） | 基础（直接依赖） |
| 缓存 | npm cache | node_modules |
| 离线支持 | ✅ | ❌（需要网络） |

### 适用场景

✅ **适合**：
- 快速原型开发
- 没有 Node.js 环境
- 简单的 Vue 项目
- 学习和测试

⚠️ **不适合**：
- 复杂的生产项目
- 需要精确依赖锁定
- 需要 peer dependencies
- 离线环境

## 技术细节

### HTTP 请求流程

```
1. GET https://registry.npmjs.org/vue
   → 获取包元数据（versions, dist-tags）

2. 解析 latest 版本
   → vue@3.5.33

3. 获取 tarball URL
   → https://registry.npmjs.org/vue/-/vue-3.5.33.tgz

4. GET <tarball_url>
   → 下载 .tgz 文件

5. 解压 tarball
   → package/* → node_modules/vue/*
```

### 错误处理

- **网络错误**：重试机制（未来版本）
- **包不存在**：记录警告，继续编译
- **解压失败**：清理临时文件，返回错误
- **版本冲突**：使用 package.json 指定的版本

## 未来计划

### Phase 10: npm 下载器增强

- [ ] 并行下载多个包
- [ ] HTTP 重试机制
- [ ] 代理服务器支持
- [ ] 离线缓存（离线模式）
- [ ] 依赖树完整解析（peer deps）
- [ ] package-lock.json 支持
- [ ] 下载进度显示
- [ ] 校验和验证（integrity）

## 常见问题

### Q: 是否需要安装 Node.js？

**A**: 不需要！Iris JetCrab CLI 完全独立运行，内置 npm 包下载器。

### Q: 下载速度慢怎么办？

**A**: 当前版本使用串行下载，未来会实现并行下载。可以使用国内 npm 镜像（配置中）。

### Q: 能否使用私有 registry？

**A**: 当前版本只支持官方 npm registry，未来会支持配置自定义 registry。

### Q: 下载的包会缓存吗？

**A**: 会的！包安装到 `node_modules/` 后不会重复下载，除非手动删除。

## 依赖项

```toml
[dependencies]
ureq = { version = "2.9", features = ["json"] }  # HTTP 客户端
flate2 = "1.0"                                    # Gzip 解压
tar = "0.4"                                       # Tarball 解析
```

## 总结

Iris JetCrab CLI 的内置 npm 下载器让您能够：

✅ **零配置**：无需安装 Node.js 或 npm  
✅ **自动化**：编译时自动下载缺失的包  
✅ **纯 Rust**：所有依赖都是纯 Rust 实现  
✅ **跨平台**：Windows、macOS、Linux 全支持  

现在您可以直接运行 `iris-jetcrab dev`，无需关心依赖安装！🚀
