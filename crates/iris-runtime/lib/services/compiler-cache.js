import { readFileSync, existsSync, statSync } from 'fs';
import { resolve, extname, basename, dirname } from 'path';
import { rewriteBareImports, replaceNodeEnv, generateStyleInjectCode } from '../utils/import-rewriter.js';
import { compileSfc } from './wasm-bridge.js';

export class CompilerCache {
  constructor(projectRoot) {
    this.projectRoot = projectRoot;
    this._cache = new Map();
  }

  async getOrCompile(modulePath) {
    const normalized = modulePath.startsWith('/') ? modulePath.slice(1) : modulePath;
    const cached = this._cache.get(normalized);
    if (cached) return cached;

    const filePath = this.resolveModulePath(normalized);
    const source = readFileSync(filePath, 'utf-8');
    const ext = extname(filePath).toLowerCase();

    let compiled;
    if (ext === '.vue') {
      try {
        compiled = await compileSfc(source, normalized);
      } catch (err) {
        console.warn('[Cache] WASM compile failed for', normalized, err.message);
        compiled = { script: source, styles: [], deps: [] };
      }
    } else if (ext === '.ts' || ext === '.tsx') {
      try {
        compiled = await compileSfc('<script lang="ts">' + source + '</script>', normalized);
      } catch (err) {
        compiled = { script: replaceNodeEnv(source), styles: [], deps: [] };
      }
    } else {
      compiled = { script: replaceNodeEnv(source), styles: [], deps: [] };
    }

    const hmrCode = 'if (import.meta.hot){import.meta.hot.accept(function(m){console.log("[HMR] Module updated:","' + normalized + '");});}\n';
    compiled.script = hmrCode + rewriteBareImports(compiled.script, normalized);
    compiled.finalCode = compiled.script + generateStyleInjectCode(compiled.styles || []);
    this._cache.set(normalized, compiled);
    return compiled;
  }

  resolveModulePath(modulePath) {
    const normalized = modulePath.replace(/\\/g, '/').replace(/^\/+/, '');
    const candidates = [
      resolve(this.projectRoot, 'src', normalized),
      resolve(this.projectRoot, normalized),
    ];
    for (const c of candidates) {
      if (existsSync(c)) {
        const s = statSync(c);
        if (s.isFile()) return c;
        if (s.isDirectory()) {
          for (const idx of ['index.ts', 'index.js', 'index.tsx', 'index.jsx', 'index.mjs']) {
            const p = resolve(c, idx);
            if (existsSync(p)) return p;
          }
        }
      }
    }
    if (!normalized.includes('.')) {
      for (const e of ['.ts', '.tsx', '.vue', '.js', '.jsx', '.mjs']) {
        for (const b of [resolve(this.projectRoot, 'src', normalized + e), resolve(this.projectRoot, normalized + e)]) {
          if (existsSync(b)) return b;
        }
      }
    }
    throw new Error('Module not found: ' + modulePath);
  }

  invalidate(modulePath) {
    const normalized = modulePath.replace(/\\/g, '/');
    for (const key of this._cache.keys()) {
      if (key === normalized || key.endsWith(normalized) || normalized.endsWith(key)) this._cache.delete(key);
    }
  }

  invalidateModules(changedPaths) {
    let count = 0;
    for (const cp of changedPaths) {
      const relative = cp.replace(this.projectRoot.replace(/\\/g, '/'), '').replace(/^[/\\]+/, '');
      const key = relative.replace(/\\/g, '/');
      if (this._cache.delete(key)) count++;
      const stem = basename(key, extname(key));
      const d = dirname(key);
      const noExt = d !== '.' ? d + '/' + stem : stem;
      if (noExt !== key && this._cache.delete(noExt)) count++;
    }
    return count;
  }

  clear() {
    this._cache.clear();
  }

  stats() {
    return { size: this._cache.size };
  }
}
