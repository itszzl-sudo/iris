# iris-runtime 开发指南

> 本文档说明如何构建、打包和发布 iris-runtime npm 包

---

## 📋 前置要求

### 必需工具

- **Rust** >= 1.78
- **Node.js** >= 16.0.0
- **wasm-pack** >= 0.12.1

### 安装 wasm-pack

```bash
# 使用 cargo 安装
cargo install wasm-pack

# 或使用官方安装脚本
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

---

## 🔧 开发流程

### 1. 修改 Rust 代码

```bash
# 编辑 WASM 模块
cd crates/iris-runtime
vim src/lib.rs
vim src/compiler.rs
vim src/hmr.rs
```

### 2. 编译 WASM

**生产构建（优化大小）**:

```bash
# 方式 1: 直接使用 wasm-pack
wasm-pack build --target nodejs --release

# 方式 2: 使用 npm script
npm run build:wasm

# 方式 3: 使用构建脚本
./build-wasm.sh          # Linux/macOS
.\build-wasm.ps1         # Windows
```

**开发构建（包含调试信息）**:

```bash
wasm-pack build --target nodejs --dev
npm run build:wasm:dev
./build-wasm.sh dev
```

### 3. 检查生成的文件

```bash
ls -lh pkg/
```

应该包含：

```
pkg/
├── iris_runtime_bg.wasm       # WASM 二进制 (~5MB)
├── iris_runtime_bg.wasm.d.ts  # TypeScript 类型
├── iris_runtime.js            # JavaScript 绑定
└── iris_runtime.d.ts          # TypeScript 声明
```

### 4. 本地测试

```bash
# 安装依赖
npm install

# 创建 npm 包
npm pack

# 在其他项目中测试
cd /path/to/vue-project
npm install /path/to/iris-runtime-0.1.0.tgz
npx iris-runtime dev
```

### 5. 发布到 npm

```bash
# 登录 npm
npm login

# 更新版本号
npm version patch  # 或 minor, major

# 发布
npm run publish
# 或
npm publish --access public
```

---

## 📦 包结构

### 源码结构

```
crates/iris-runtime/
├── Cargo.toml               # Rust crate 配置
├── package.json             # npm 包配置
├── README.md                # 使用文档
├── build-wasm.sh            # Linux/macOS 构建脚本
├── build-wasm.ps1           # Windows 构建脚本
├── src/
│   ├── lib.rs               # WASM 导出接口
│   ├── compiler.rs          # Vue SFC 编译器
│   └── hmr.rs               # 热模块替换
├── bin/
│   └── iris-runtime.js      # CLI 入口
└── lib/
    └── dev-server.js        # 开发服务器实现
```

### 编译后结构

```
crates/iris-runtime/
├── pkg/                     # wasm-pack 生成
│   ├── iris_runtime_bg.wasm
│   ├── iris_runtime.js
│   └── iris_runtime.d.ts
├── bin/
└── lib/
```

---

## 🎯 用户使用流程

### 在 Vue 项目中使用

```bash
# 1. 创建 Vue 项目（使用 create-vue 或其他工具）
npm create vue@latest my-app
cd my-app

# 2. 安装 iris-runtime
npm install -D iris-runtime

# 3. 启动开发服务器
npx iris-runtime dev

# 4. 浏览器访问 http://localhost:3000
```

### package.json 配置

```json
{
  "scripts": {
    "dev": "iris-runtime dev",
    "dev:port": "iris-runtime dev --port 8080",
    "build": "vite build"
  }
}
```

---

## ⚙️ 编译选项

### Cargo.toml 优化

```toml
[profile.release]
opt-level = "z"      # 优化大小（而非性能）
lto = true           # 链接时优化
codegen-units = 1    # 减少代码单元
strip = true         # 移除调试信息
```

### wasm-pack 选项

```bash
# 目标平台
--target nodejs       # Node.js 环境
--target web          # 浏览器环境
--target bundler      # Webpack/Rollup

# 构建模式
--release             # 生产构建
--dev                 # 开发构建
--profiling           # 性能分析构建
```

---

## 🐛 调试

### 启用调试日志

```javascript
// 在 bin/iris-runtime.js 中
const runtime = new IrisRuntime();
runtime.setDebug(true);  // 启用调试模式
```

### 查看 WASM 编译输出

```bash
# 详细输出
RUST_LOG=debug wasm-pack build --target nodejs --dev

# 查看生成的 JS 代码
cat pkg/iris_runtime.js
```

### 测试编译结果

```bash
# Node.js REPL 测试
node
> import('./pkg/iris_runtime.js').then(m => {
    const runtime = new m.IrisRuntime();
    console.log(runtime.version());
  });
```

---

## 📊 性能指标

### WASM 大小

| 构建模式 | 大小 | 说明 |
|---------|------|------|
| dev | ~15MB | 包含调试信息 |
| release | ~5MB | 生产优化 |
| profiling | ~10MB | 性能分析 |

### 编译速度

| 操作 | 时间 |
|------|------|
| 首次编译 | ~30s |
| 增量编译 | ~5s |
| WASM 编译 | ~10s |

### 运行时性能

| 操作 | 时间 |
|------|------|
| SFC 编译 | <100ms |
| HMR 更新 | <50ms |
| 服务器启动 | <1s |

---

## 🚀 CI/CD 配置

### GitHub Actions 示例

```yaml
name: Build and Publish

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
        
      - name: Build WASM
        run: |
          cd crates/iris-runtime
          wasm-pack build --target nodejs --release
          
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
          registry-url: 'https://registry.npmjs.org'
          
      - name: Publish to npm
        run: |
          cd crates/iris-runtime
          npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

---

## 📝 版本管理

### 语义化版本

```bash
# Patch (bug fixes)
npm version patch  # 0.1.0 → 0.1.1

# Minor (new features, backwards compatible)
npm version minor  # 0.1.0 → 0.2.0

# Major (breaking changes)
npm version major  # 0.1.0 → 1.0.0
```

### 发布清单

- [ ] 更新版本号
- [ ] 更新 CHANGELOG.md
- [ ] 运行测试 `cargo test`
- [ ] 编译 WASM `npm run build:wasm`
- [ ] 本地测试 `npm pack && npm install`
- [ ] 发布 `npm publish`
- [ ] 创建 Git tag `git tag v0.1.0 && git push --tags`

---

## 🎓 常见问题

### Q: wasm-pack 编译失败？

**A**: 确保安装了 `wasm32-unknown-unknown` 目标：

```bash
rustup target add wasm32-unknown-unknown
```

### Q: WASM 文件太大？

**A**: 使用 `--release` 模式并启用优化：

```toml
[profile.release]
opt-level = "z"
lto = true
strip = true
```

### Q: 如何在浏览器中使用？

**A**: 修改 target 为 `web`：

```bash
wasm-pack build --target web --release
```

---

## 📚 相关资源

- [wasm-pack 文档](https://rustwasm.github.io/wasm-pack/)
- [WebAssembly 规范](https://webassembly.github.io/spec/)
- [npm 发布指南](https://docs.npmjs.com/packages-and-modules/contributing-packages-to-the-registry)

---

**文档维护者**: Iris Development Team  
**最后更新**: 2026-04-28
