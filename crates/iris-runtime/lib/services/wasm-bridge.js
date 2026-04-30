import { existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
let engineInstance = null;

// WASM 模块搜索路径（按优先级）
const WASM_SOURCES = [
  // 1. iris-sfc-wasm（独立 SFC 编译器，推荐）
  {
    name: 'iris-sfc-wasm',
    path: resolve(__dirname, '..', '..', '..', 'iris-sfc-wasm', 'pkg'),
    js: 'iris_sfc_wasm.js',
    factory: (mod) => mod
  },
  // 2. iris-jetcrab-engine（完整引擎）
  {
    name: 'iris-jetcrab-engine',
    path: resolve(__dirname, '..', '..', '..', 'iris-jetcrab-engine', 'pkg'),
    js: 'iris_jetcrab_engine.js',
    factory: (mod) => ({ compileSfc: mod.IrisEngine?.compileSfc })
  },
  // 3. iris-runtime（传统方式）
  {
    name: 'iris-runtime',
    path: resolve(__dirname, '..', '..', 'pkg'),
    js: 'iris_runtime.js',
    factory: (mod) => mod
  }
];

export async function ensureEngine() {
  if (engineInstance) return engineInstance;

  for (const source of WASM_SOURCES) {
    const jsPath = resolve(source.path, source.js);
    if (existsSync(jsPath)) {
      try {
        const url = pathToFileURL(jsPath).href;
        const mod = await import(url);
        // wasm-pack --target nodejs 自动在模块加载时初始化 WASM
        // 但通过 ESM import() 加载 CJS 时，mod.default 是 exports 对象而非函数
        // 尝试调用 default() 仅当它是函数（wasm-pack --target web 模式）
        if (typeof mod.default === 'function') {
          await mod.default();
        }
        engineInstance = mod;
        return engineInstance;
      } catch (err) {
        console.warn('[WASM] Failed to load ' + source.name + ':', err.message);
      }
    }
  }

  const error = new Error(
    'WASM engine not found. Build the WASM module first:\n\n' +
    '  cd ' + resolve(__dirname, '..', '..', '..', 'iris-sfc-wasm') + '\n' +
    '  wasm-pack build --target nodejs --release\n'
  );
  error.code = 'WASM_ENGINE_MISSING';
  throw error;
}

export async function compileSfc(source, filename) {
  const engine = await ensureEngine();
  const result = engine.compileSfc(source, filename);
  return JSON.parse(result);
}

export async function resolveImport(importPath, importer) {
  const engine = await ensureEngine();
  if (engine.resolveImport) {
    return engine.resolveImport(importPath, importer);
  }
  throw new Error('resolveImport not supported by the loaded WASM module');
}

export async function clearCache() {
  if (engineInstance && engineInstance.clearCache) {
    engineInstance.clearCache();
  }
}
