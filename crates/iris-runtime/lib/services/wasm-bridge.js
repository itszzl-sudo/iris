import { existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
let engineInstance = null;

export async function ensureEngine() {
  if (engineInstance) return engineInstance;

  const enginePkgPath = resolve(__dirname, '..', '..', '..', 'iris-jetcrab-engine', 'pkg');
  const runtimePkgPath = resolve(__dirname, '..', '..', 'pkg');
  const engineJsPath = resolve(enginePkgPath, 'iris_jetcrab_engine.js');
  const runtimeJsPath = resolve(runtimePkgPath, 'iris_runtime.js');

  if (existsSync(engineJsPath)) {
    try {
      const engineUrl = engineJsPath.replace(/\\/g, '/');
      const mod = await import(engineUrl);
      const initEngine = mod.default;
      const { IrisEngine } = mod;
      await initEngine();
      engineInstance = new IrisEngine();
      return engineInstance;
    } catch (err) {
      console.warn('[WASM] Failed to load iris-jetcrab-engine:', err.message);
    }
  }

  if (existsSync(runtimeJsPath)) {
    try {
      const runtimeUrl = runtimeJsPath.replace(/\\/g, '/');
      const mod = await import(runtimeUrl);
      if (mod.IrisRuntime) {
        engineInstance = new mod.IrisRuntime();
        return engineInstance;
      }
      if (mod.default) {
        await mod.default();
        engineInstance = new mod.IrisRuntime();
        return engineInstance;
      }
    } catch (err) {
      console.warn('[WASM] Failed to load iris-runtime WASM:', err.message);
    }
  }

  const error = new Error(
    'WASM engine not found. Build the WASM module first:\n\n' +
    '  cd ' + resolve(__dirname, '..', '..', '..', 'iris-jetcrab-engine') + '\n' +
    '  wasm-pack build --target nodejs --release\n\n' +
    'Or run the build script:\n  npm run build:wasm\n'
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
  return engine.resolveImport(importPath, importer);
}

export async function clearCache() {
  if (engineInstance) engineInstance.clearCache();
}
