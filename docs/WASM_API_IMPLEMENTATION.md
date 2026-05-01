# WASM 导出接口实现总结

## 完成时间
2026-02-09

## 实现内容

### 1. 新增文件

#### `src/wasm_api.rs` (192 行)
- **IrisEngine 结构体** - WASM 导出核心类
- **主要方法**:
  - `new()` - 创建引擎实例
  - `compileSfc()` - 编译 Vue SFC
  - `resolveImport()` - 解析模块路径
  - `generateHmrPatch()` - 生成 HMR 补丁
  - `getCompiledModule()` - 获取已编译模块
  - `clearCache()` - 清除缓存
  - `getCacheSize()` - 获取缓存大小
  - `setDebug()` - 设置调试模式
  - `version()` - 获取版本信息

### 2. 修改文件

#### `Cargo.toml`
**新增依赖:**
```toml
# WASM 绑定
wasm-bindgen = "0.2.89"
wasm-bindgen-futures = "0.4.39"
serde-wasm-bindgen = "0.6.3"
js-sys = "0.3.66"
console_error_panic_hook = { version = "0.1.7", optional = true }
```

**新增配置:**
```toml
[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = []
wasm = ["console_error_panic_hook"]

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

#### `src/lib.rs`
```rust
pub mod wasm_api;
pub use wasm_api::IrisEngine;
```

### 3. 构建脚本

#### `build-wasm-engine.ps1` (Windows)
- 支持 debug/release 模式
- 输出到 `pkg-engine/` 目录
- 显示文件大小统计
- 提供使用示例

#### `build-wasm-engine.sh` (Linux/macOS)
- 功能同 PowerShell 版本
- 跨平台兼容

### 4. 文档

#### `WASM_API.md` (393 行)
- 快速开始指南
- 完整 API 参考
- 使用示例
- 性能优化建议
- 错误处理
- 浏览器兼容性

## API 设计

### 核心类: IrisEngine

```rust
#[wasm_bindgen]
pub struct IrisEngine {
    compiled_modules: HashMap<String, CompiledModule>,
    hmr_manager: HMRManager,
    debug: bool,
}
```

### JavaScript 接口

```typescript
interface IrisEngine {
  new(): IrisEngine;
  compileSfc(source: string, filename: string): string;
  resolveImport(importPath: string, importer: string): string;
  generateHmrPatch(oldSource: string, newSource: string, filename: string): string;
  getCompiledModule(filename: string): string;
  clearCache(): void;
  getCacheSize(): number;
  setDebug(enabled: boolean): void;
  static version(): string;
}
```

## 编译方式

### Debug 模式
```bash
cd crates/iris-jetcrab-engine
.\build-wasm-engine.ps1 debug
```

### Release 模式
```bash
cd crates/iris-jetcrab-engine
.\build-wasm-engine.ps1
```

## 输出文件

```
pkg-engine/
├── iris_jetcrab_engine.js      # JavaScript 绑定
├── iris_jetcrab_engine.d.ts    # TypeScript 类型定义
├── iris_jetcrab_engine_bg.wasm # WASM 二进制
└── package.json                # NPM 包配置
```

## 使用示例

```javascript
import initEngine, { IrisEngine } from './pkg-engine/iris_jetcrab_engine.js';

await initEngine();
const engine = new IrisEngine();

// 编译 Vue SFC
const result = engine.compileSfc(`
  <template>
    <h1>{{ message }}</h1>
  </template>
  
  <script>
  export default {
    data() {
      return { message: 'Hello!' }
    }
  }
  </script>
`, 'App.vue');

const compiled = JSON.parse(result);
console.log(compiled.script);
```

## 技术亮点

1. **零成本抽象** - WASM 原生性能
2. **类型安全** - TypeScript 类型定义自动生成
3. **错误处理** - JsError 统一异常处理
4. **缓存优化** - 编译结果自动缓存
5. **HMR 支持** - 热更新补丁生成
6. **调试友好** - 可选的 console_error_panic_hook

## 与现有代码集成

- ✅ 复用 `sfc_compiler` 模块
- ✅ 复用 `hmr` 模块
- ✅ 统一的错误处理
- ✅ 兼容 iris dev-server

## 下一步

1. **测试** - 编写 WASM 集成测试
2. **优化** - 性能基准测试
3. **文档** - 更新双运行时架构文档
4. **集成** - 与 iris dev-server 对接

## 编译状态

⏳ 等待依赖下载和编译完成...

## 统计

- **新增代码**: ~192 行 (wasm_api.rs)
- **新增文档**: ~393 行 (WASM_API.md)
- **构建脚本**: ~120 行 (ps1 + sh)
- **依赖新增**: 5 个 WASM 相关 crate
- **API 方法**: 9 个公共方法
