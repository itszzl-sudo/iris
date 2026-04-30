import { createServer } from 'http';
import { readFileSync, existsSync, statSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';
import open from 'open';

import { isVueProjectRoot } from './utils/vue-detect.js';
import { isPortInUse, findAvailablePort } from './utils/port.js';
import { getContentType } from './utils/mime.js';
import { generateIrisIndexHtml, generateDirectorySelectorPage, generateResolvePageHtml } from './utils/html.js';
import { CompilerCache } from './services/compiler-cache.js';
import { startFileWatcher } from './services/file-watcher.js';
import { sourceFileHandler } from './routes/source.js';
import { vueModuleHandler } from './routes/vue-module.js';
import { npmPackageHandler } from './routes/npm-package.js';
import { setupWebSocketUpgrade, handleFileChanges } from './routes/hmr.js';
import { apiHandler } from './routes/api.js';
import { MockApiHandler } from './routes/mock-api.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const TEMPLATE_DIR = resolve(__dirname, 'templates');

export async function startDevServer(config) {
  const {
    root = process.cwd(),
    port: requestedPort = 3000,
    host = 'localhost',
    open: shouldOpen = true,
    enableHmr = true,
    debug = false,
    mock = false,
  } = config;
  const startTime = Date.now();

  printBanner();
  console.log(chalk.dim('  📁 Project root:'), root);
  const projectInfo = isVueProjectRoot(root);

  if (projectInfo.isVueProject) {
    console.log(chalk.green('  ✓ Vue project detected'));
    console.log(chalk.dim('     Version: ' + projectInfo.vueVersion));
    console.log(chalk.dim('     Build tool: ' + projectInfo.buildTool));
    console.log(chalk.dim('     Entry file: ' + (projectInfo.entryFile || 'Auto-scan')));
  } else {
    console.log(chalk.yellow('  ⚠  Not a Vue project'));
    console.log(chalk.dim('     ' + projectInfo.reason));
  }

  const cache = new CompilerCache(root);

  // 初始化 Mock API Server
  let mockHandler = null;
  if (mock) {
    const mockOptions = typeof mock === 'object' ? mock : { enabled: true };
    if (mockOptions.enabled !== false) {
      mockHandler = new MockApiHandler(root, mockOptions);
      await mockHandler.initialize();
    }
  }

  const portInUse = await isPortInUse(requestedPort);
  let port = requestedPort;
  if (portInUse) {
    if (requestedPort === 0) {
      port = await findAvailablePort(3000);
      console.log(chalk.yellow('  ⚠  Port in use, using ' + port));
    } else {
      console.log(chalk.red('  ❌ Port ' + port + ' is already in use'));
      process.exit(1);
    }
  }

  const server = createServer(async (req, res) => {
    try {
      await handleRequest(req, res, { root, cache, projectInfo, enableHmr, mockHandler });
    } catch (error) {
      console.error(chalk.red('  ❌ Request error:'), error.message);
      if (debug) console.error(error);
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end('Internal Server Error');
    }
  });

  let wss = null;
  if (enableHmr) {
    wss = setupWebSocketUpgrade(server, cache, root);
  }

  await new Promise((resolve, reject) => {
    server.listen(port, host, () => {
      const elapsed = Date.now() - startTime;
      console.log();
      console.log(chalk.green('  ➜ Local:  '), chalk.cyan('http://' + host + ':' + port));
      console.log(chalk.green('  ➜ Network:'), chalk.dim('use --host 0.0.0.0 to expose'));
      console.log(chalk.green('  ➜ Ready in'), chalk.cyan(elapsed + 'ms'));
      console.log();
      resolve();
    });
    server.on('error', (error) => {
      if (error.code === 'EADDRINUSE') {
        server.close();
        server.listen(port + 1, host, resolve);
      } else {
        reject(error);
      }
    });
  });

  let watcher = null;
  if (enableHmr) {
    const srcDir = resolve(root, 'src');
    if (existsSync(srcDir)) {
      watcher = startFileWatcher(srcDir, cache, (changes) => {
        if (wss) handleFileChanges(changes, wss, cache, root);
      });
    } else {
      console.log(chalk.yellow('  ⚠  No src/ directory, watching disabled'));
    }
  }

  if (shouldOpen) {
    try { await open('http://' + host + ':' + port); } catch (_) {}
  }

  process.on('SIGINT', async () => {
    console.log(chalk.yellow('\n  👋 Shutting down...'));
    if (watcher) await watcher.close();
    if (wss) wss.close();
    server.close();
    process.exit(0);
  });

  return { server, watcher, wss, cache, mockHandler };
}

async function handleRequest(req, res, ctx) {
  const { root, cache, projectInfo, mockHandler } = ctx;
  const url = new URL(req.url, 'http://' + req.headers.host);
  const pathname = url.pathname;

  if (pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(projectInfo.isVueProject ? generateIrisIndexHtml(root) : generateDirectorySelectorPage());
    return;
  }

  if (pathname === '/resolve.html') {
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(generateResolvePageHtml());
    return;
  }

  if (pathname.startsWith('/src/')) return sourceFileHandler(req, res, url, cache);
  if (pathname.startsWith('/@vue/')) return vueModuleHandler(req, res, url, cache);
  if (pathname.startsWith('/@npm/')) return npmPackageHandler(req, res, url, root);

  // OPTIONS 预检请求
  if (req.method === 'OPTIONS') {
    if (pathname.startsWith('/api/') && mockHandler) {
      if (mockHandler.handleOptions(req, res)) return;
    }
    res.writeHead(204);
    res.end();
    return;
  }

  // Mock API 路由（优先于真实 API）
  if (pathname.startsWith('/api/') && mockHandler) {
    if (await mockHandler.handle(req, res, url)) return;
  }

  if (pathname.startsWith('/api/')) {
    if (await apiHandler(req, res, url, cache, root, projectInfo)) return;
  }

  if (pathname.startsWith('/@iris/')) {
    const assetName = pathname.replace('/@iris/', '');
    const assetPath = resolve(TEMPLATE_DIR, 'assets', assetName);
    if (existsSync(assetPath)) {
      res.writeHead(200, { 'Content-Type': getContentType(assetPath) });
      res.end(readFileSync(assetPath));
      return;
    }
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('Asset not found: ' + assetName);
    return;
  }

  // 从项目根目录服务文件
  const filePath = resolve(root, pathname.slice(1));
  if (existsSync(filePath) && statSync(filePath).isFile()) {
    res.writeHead(200, { 'Content-Type': getContentType(filePath) });
    res.end(readFileSync(filePath));
    return;
  }

  // favicon.ico 不存在时，返回彩虹 emoji SVG
  if (pathname === '/favicon.ico' || pathname === '/__iris-favicon.svg') {
    res.writeHead(200, {
      'Content-Type': 'image/svg+xml',
      'Cache-Control': 'public, max-age=3600'
    });
    res.end('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" fill="#f0f0f0" rx="14"/><text x="50" y="82" text-anchor="middle" font-size="72">&#127752;</text></svg>');
    return;
  }

  // 图片文件不存在时，生成占位 SVG
  if (isImagePath(pathname)) {
    const filename = pathname.split('/').pop() || pathname;
    const safeName = filename.length > 30 ? filename.slice(0, 27) + '...' : filename;
    res.writeHead(200, {
      'Content-Type': 'image/svg+xml',
      'Cache-Control': 'no-cache'
    });
    res.end(`<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">
  <rect width="400" height="300" fill="#f5f5f5" rx="12"/>
  <rect x="1.5" y="1.5" width="397" height="297" fill="none" stroke="#e0e0e0" stroke-width="2" rx="12"/>
  <text x="200" y="140" text-anchor="middle" font-size="64">&#128196;</text>
  <text x="200" y="200" text-anchor="middle" font-size="15" font-family="sans-serif" fill="#b0b0b0">${escapeXml(safeName)} (placeholder)</text>
</svg>`);
    return;
  }

  res.writeHead(404, { 'Content-Type': 'text/plain' });
  res.end('Not Found');
}

function isImagePath(path) {
  const lower = path.toLowerCase();
  return lower.endsWith('.png') || lower.endsWith('.jpg') || lower.endsWith('.jpeg')
    || lower.endsWith('.gif') || lower.endsWith('.webp') || lower.endsWith('.bmp')
    || lower.endsWith('.ico') || lower.endsWith('.svg');
}

function escapeXml(str) {
  return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function printBanner() {
  console.log();
  console.log(chalk.cyan('  ╔══════════════════════════════════╗'));
  console.log(chalk.cyan('  ║     Iris Runtime v0.1.0          ║'));
  console.log(chalk.cyan('  ║     Vue 3 Dev Server             ║'));
  console.log(chalk.cyan('  ╚══════════════════════════════════╝'));
  console.log();
}
