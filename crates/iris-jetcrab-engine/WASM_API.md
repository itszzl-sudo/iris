# Iris JetCrab Engine WASM 导出接口

## 概述

iris-jetcrab-engine 提供了完整的 WASM 导出接口，允许浏览器端直接调用 Vue SFC 编译、模块解析和热更新功能。

## 快速开始

### 1. 编译 WASM

**Windows:**
```bash
cd crates/iris-jetcrab-engine
.\build-wasm-engine.ps1
```

**Linux/macOS:**
```bash
cd crates/iris-jetcrab-engine
chmod +x build-wasm-engine.sh
./build-wasm-engine.sh
```

### 2. 在 JavaScript 中使用

```javascript
// 导入 WASM 模块
import initEngine, { IrisEngine } from './pkg-engine/iris_jetcrab_engine.js';

// 初始化 WASM
await initEngine();

// 创建引擎实例
const engine = new IrisEngine();

// 编译 Vue SFC
const source = `
<template>
  <h1>{{ message }}</h1>
</template>

<script>
export default {
  data() {
    return { message: 'Hello Iris!' }
  }
}
</script>

<style scoped>
h1 { color: #42b883; }
</style>
`;

const result = engine.compileSfc(source, 'App.vue');
const compiled = JSON.parse(result);

console.log(compiled.script); // 编译后的 JavaScript
console.log(compiled.styles); // 样式数组
console.log(compiled.deps);   // 依赖列表
```

## API 参考

### IrisEngine 类

#### 构造函数

```javascript
const engine = new IrisEngine();
```

#### 方法

##### `compileSfc(source: string, filename: string): string`

编译 Vue SFC 文件。

**参数:**
- `source` - Vue SFC 源码
- `filename` - 文件名（用于错误提示和 sourcemap）

**返回:** JSON 字符串

```json
{
  "script": "export default { ... }",
  "styles": [
    { "code": "h1 { color: #42b883; }", "scoped": true }
  ],
  "deps": ["./components/Foo.vue"]
}
```

**示例:**
```javascript
const result = engine.compileSfc(source, 'App.vue');
const compiled = JSON.parse(result);
```

---

##### `resolveImport(importPath: string, importer: string): string`

解析模块导入路径。

**参数:**
- `importPath` - 导入路径
- `importer` - 导入者路径

**返回:** 解析后的绝对路径

**示例:**
```javascript
const resolved = engine.resolveImport('./components/Foo', 'App.vue');
console.log(resolved); // "components/Foo.vue"
```

---

##### `generateHmrPatch(oldSource: string, newSource: string, filename: string): string`

生成热更新补丁。

**参数:**
- `oldSource` - 旧源码
- `newSource` - 新源码  
- `filename` - 文件名

**返回:** JSON 字符串

```json
{
  "type": "vue-reload",
  "path": "App.vue",
  "timestamp": 1234567890,
  "changes": ["script", "template"]
}
```

**示例:**
```javascript
const patch = engine.generateHmrPatch(oldSource, newSource, 'App.vue');
const patchData = JSON.parse(patch);

if (patchData.type === 'vue-reload') {
  // 重新加载组件
}
```

---

##### `getCompiledModule(filename: string): string`

获取已编译模块的信息。

**参数:**
- `filename` - 文件名

**返回:** JSON 字符串（格式同 compileSfc）

**示例:**
```javascript
const module = engine.getCompiledModule('App.vue');
```

---

##### `clearCache(): void`

清除编译缓存。

**示例:**
```javascript
engine.clearCache();
```

---

##### `getCacheSize(): number`

获取编译缓存大小（已编译的模块数量）。

**示例:**
```javascript
console.log(`缓存中有 ${engine.getCacheSize()} 个模块`);
```

---

##### `setDebug(enabled: boolean): void`

设置调试模式。

**参数:**
- `enabled` - 是否启用调试

**示例:**
```javascript
engine.setDebug(true);
```

---

##### `static version(): string`

获取引擎版本。

**示例:**
```javascript
console.log(`Iris Engine 版本: ${IrisEngine.version()}`);
```

## 完整示例

### Vue 开发服务器集成

```javascript
import initEngine, { IrisEngine } from './pkg-engine/iris_jetcrab_engine.js';

class VueDevServer {
  constructor() {
    this.engine = null;
    this.modules = new Map();
  }

  async init() {
    await initEngine();
    this.engine = new IrisEngine();
    this.engine.setDebug(true);
  }

  async compileFile(filename, source) {
    const result = this.engine.compileSfc(source, filename);
    const compiled = JSON.parse(result);
    
    this.modules.set(filename, {
      compiled,
      source,
      timestamp: Date.now()
    });

    return compiled;
  }

  async updateFile(filename, newSource) {
    const old = this.modules.get(filename);
    if (!old) {
      return await this.compileFile(filename, newSource);
    }

    // 生成 HMR 补丁
    const patch = this.engine.generateHmrPatch(
      old.source,
      newSource,
      filename
    );
    const patchData = JSON.parse(patch);

    // 更新缓存
    old.source = newSource;
    old.timestamp = Date.now();

    return patchData;
  }

  resolveImport(importPath, importer) {
    return this.engine.resolveImport(importPath, importer);
  }

  getCacheStats() {
    return {
      size: this.engine.getCacheSize(),
      modules: Array.from(this.modules.keys())
    };
  }
}

// 使用
const server = new VueDevServer();
await server.init();

const compiled = await server.compileFile('App.vue', vueSource);
console.log(compiled);
```

## 构建选项

### 开发模式

```bash
.\build-wasm-engine.ps1 debug
```

- 编译速度快
- 文件较大
- 包含调试信息

### 发布模式

```bash
.\build-wasm-engine.ps1 release
```

- 编译速度慢
- 文件最小化
- 启用 LTO 优化
- 去除调试信息

## 输出文件

编译后会在 `pkg-engine/` 目录生成以下文件：

```
pkg-engine/
├── iris_jetcrab_engine.js      # JavaScript 绑定
├── iris_jetcrab_engine.d.ts    # TypeScript 类型定义
├── iris_jetcrab_engine_bg.wasm # WASM 二进制
└── package.json                # NPM 包配置
```

## 性能优化

### 1. 使用缓存

```javascript
// 编译缓存自动管理
const result1 = engine.compileSfc(source, 'App.vue'); // 首次编译
const result2 = engine.getCompiledModule('App.vue');   // 从缓存获取
```

### 2. 批量处理

```javascript
// 批量编译多个文件
const files = ['App.vue', 'Header.vue', 'Footer.vue'];
for (const file of files) {
  engine.compileSfc(source, file);
}
```

### 3. Release 模式

生产环境使用 release 模式编译，可获得 5-10 倍性能提升。

## 错误处理

```javascript
try {
  const result = engine.compileSfc(source, 'App.vue');
} catch (error) {
  console.error('编译失败:', error.message);
  // 错误信息包含详细的编译错误
}
```

## 与 iris-runtime 集成

iris-runtime 的 dev-server.js 可以直接使用 WASM 模块：

```javascript
// crates/iris-runtime/lib/dev-server.js
import initEngine, { IrisEngine } from '/@iris/engine/iris_jetcrab_engine.js';

let engine = null;

async function initWasm() {
  await initEngine();
  engine = new IrisEngine();
}

// 编译 Vue 文件
async function compileVueFile(filename, source) {
  if (!engine) await initWasm();
  
  const result = engine.compileSfc(source, filename);
  return JSON.parse(result);
}
```

## 浏览器兼容性

- ✅ Chrome 97+
- ✅ Firefox 96+
- ✅ Safari 15.4+
- ✅ Edge 97+

## 相关文档

- [双运行时架构](../../docs/DUAL_RUNTIME_ARCHITECTURE.md)
- [iris-jetcrab-engine 实现报告](../../docs/IRIS_JETCRAB_ENGINE_IMPLEMENTATION.md)
- [架构调整说明](../../docs/IRIS_RUNTIME_ARCHITECTURE_CHANGE.md)
