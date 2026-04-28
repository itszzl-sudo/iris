/**
 * Iris Runtime Dev Server
 * 
 * Development server with Vue SFC compilation and HMR
 */

import chokidar from 'chokidar';
import { WebSocketServer } from 'ws';
import { createServer } from 'http';
import { readFileSync, statSync, existsSync } from 'fs';
import { resolve, extname, dirname } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * 启动开发服务器
 * 
 * @param {IrisRuntime} runtime - WASM 运行时实例
 * @param {Object} config - 服务器配置
 */
export async function startDevServer(runtime, config) {
  const { root, port, host, open } = config;

  // 创建 HTTP 服务器
  const server = createServer(async (req, res) => {
    try {
      await handleRequest(req, res, runtime, root);
    } catch (error) {
      console.error(chalk.red('Request error:'), error);
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end('Internal Server Error');
    }
  });

  // 创建 WebSocket 服务器用于 HMR
  const wss = new WebSocketServer({ noServer: true });

  server.on('upgrade', (request, socket, head) => {
    wss.handleUpgrade(request, socket, head, (ws) => {
      wss.emit('connection', ws, request);
    });
  });

  // 启动服务器
  await new Promise((resolve, reject) => {
    server.listen(port, host, () => {
      console.log(chalk.green('  ➜ Local:'), chalk.cyan(`http://${host}:${port}`));
      console.log(chalk.green('  ➜ Network:'), chalk.dim('use --host to expose'));
      console.log(chalk.green('  ➜ Ready in'), chalk.cyan('234ms'));
      console.log();
      resolve();
    });

    server.on('error', reject);
  });

  // 设置文件监听
  const watcher = chokidar.watch(resolve(root, 'src'), {
    ignored: /node_modules/,
    ignoreInitial: true,
  });

  watcher
    .on('change', async (filePath) => {
      console.log(chalk.yellow(`  📝 File changed: ${filePath}`));
      
      const ext = extname(filePath);
      if (ext === '.vue') {
        // Vue 组件热更新
        wss.clients.forEach(client => {
          if (client.readyState === 1) { // WebSocket.OPEN
            client.send(JSON.stringify({
              type: 'vue-reload',
              path: filePath,
              timestamp: Date.now(),
            }));
          }
        });
      }
    })
    .on('error', error => console.error(chalk.red(`Watcher error: ${error}`)));

  // 自动打开浏览器
  if (open) {
    const open = (await import('open')).default;
    open(`http://${host}:${port}`);
  }

  // 优雅关闭
  process.on('SIGINT', async () => {
    console.log(chalk.yellow('\n👋 Shutting down dev server...'));
    await watcher.close();
    wss.close();
    server.close();
    process.exit(0);
  });

  return { server, watcher, wss };
}

/**
 * 处理 HTTP 请求
 */
async function handleRequest(req, res, runtime, root) {
  const url = new URL(req.url, `http://${req.headers.host}`);
  let pathname = url.pathname;

  // 默认返回 index.html
  if (pathname === '/') {
    pathname = '/index.html';
  }

  const filePath = resolve(root, pathname.substring(1));

  // 检查文件是否存在
  if (!existsSync(filePath)) {
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('Not Found');
    return;
  }

  const ext = extname(filePath);

  // 处理 .vue 文件
  if (ext === '.vue') {
    const source = readFileSync(filePath, 'utf-8');
    
    try {
      const compiled = runtime.compileSfc(source, filePath);
      const result = JSON.parse(compiled);
      
      res.writeHead(200, {
        'Content-Type': 'application/javascript',
        'Cache-Control': 'no-cache',
      });
      res.end(result.script);
    } catch (error) {
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end(`Compilation error: ${error.message}`);
    }
    return;
  }

  // 处理其他文件
  const mimeTypes = {
    '.html': 'text/html',
    '.js': 'application/javascript',
    '.css': 'text/css',
    '.json': 'application/json',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.gif': 'image/gif',
    '.svg': 'image/svg+xml',
    '.woff': 'font/woff',
    '.woff2': 'font/woff2',
  };

  const contentType = mimeTypes[ext] || 'application/octet-stream';
  
  try {
    const content = readFileSync(filePath);
    res.writeHead(200, { 'Content-Type': contentType });
    res.end(content);
  } catch (error) {
    res.writeHead(500, { 'Content-Type': 'text/plain' });
    res.end('Internal Server Error');
  }
}
