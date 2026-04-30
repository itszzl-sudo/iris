import chokidar from 'chokidar';
import { extname, basename } from 'path';

export const ChangeType = { STYLE: 'style-update', MODULE: 'module-update', ENTRY: 'full-reload', UNKNOWN: 'unknown' };

export function startFileWatcher(srcDir, cache, onChanges) {
  const watcher = chokidar.watch(srcDir, { ignored: /node_modules|\.git|\.iris-cache/, ignoreInitial: true });
  let timer = null;
  let pending = [];

  watcher.on('all', (event, filePath) => {
    if (event === 'change' || event === 'add' || event === 'unlink') {
      pending.push({ event, filePath });
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        const changes = pending.slice();
        pending = [];
        const classified = changes.map(c => ({ ...c, type: classifyChange(c.filePath) }));
        if (classified.some(c => c.type === 'full-reload')) {
          cache.clear();
          onChanges([{ type: 'full-reload', filePath: changes[0].filePath }]);
          return;
        }
        const paths = [...new Set(classified.map(c => c.filePath))];
        const invalidated = cache.invalidateModules(paths);
        if (invalidated > 0) {
          onChanges(classified.filter(c => c.type !== 'unknown').map(c => ({
            type: c.type,
            filePath: c.filePath,
            timestamp: Date.now(),
          })));
        }
      }, 300);
    }
  });

  watcher.on('error', (err) => console.error('[Watcher] Error:', err.message));
  return watcher;
}

function classifyChange(filePath) {
  const ext = extname(filePath).toLowerCase();
  const name = basename(filePath).toLowerCase();
  if (['.css', '.scss', '.less', '.styl'].includes(ext)) return 'style-update';
  if (ext === '.vue') return 'module-update';
  if (['.ts', '.tsx', '.js', '.jsx', '.mjs'].includes(ext)) {
    return (name === 'main.ts' || name === 'main.js' || name === 'index.ts' || name === 'index.js')
      ? 'full-reload' : 'module-update';
  }
  if (name === 'index.html') return 'full-reload';
  return 'unknown';
}
