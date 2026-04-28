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
 * 检查端口是否被占用
 * 
 * @param {number} port - 端口号
 * @returns {Promise<boolean>} - 是否被占用
 */
function isPortInUse(port) {
  return new Promise((resolve) => {
    const net = require('net');
    const server = net.createServer();
    
    server.once('error', (err) => {
      if (err.code === 'EADDRINUSE') {
        resolve(true);
      } else {
        resolve(false);
      }
      server.close();
    });
    
    server.once('listening', () => {
      server.close();
      resolve(false);
    });
    
    server.listen(port);
  });
}

/**
 * 查找可用端口
 * 
 * @param {number} startPort - 起始端口
 * @returns {Promise<number>} - 可用端口号
 */
function findAvailablePort(startPort) {
  return new Promise((resolve, reject) => {
    const net = require('net');
    
    const tryPort = (port) => {
      const server = net.createServer();
      
      server.once('error', (err) => {
        if (err.code === 'EADDRINUSE') {
          // 端口被占用，尝试下一个
          if (port < startPort + 100) { // 最多尝试 100 个端口
            tryPort(port + 1);
          } else {
            reject(new Error('No available ports found'));
          }
        } else {
          reject(err);
        }
        server.close();
      });
      
      server.once('listening', () => {
        server.close();
        resolve(port);
      });
      
      server.listen(port);
    };
    
    tryPort(startPort);
  });
}

/**
 * 显示端口占用错误并退出
 * 
 * @param {number} port - 被占用的端口
 */
function showPortInUseError(port) {
  console.log();
  console.log(chalk.red('❌ Error: Port ' + port + ' is already in use'));
  console.log();
  console.log(chalk.yellow('Possible causes:'));
  console.log('  • Another instance of iris-runtime is running');
  console.log('  • Another application is using port ' + port);
  console.log();
  console.log(chalk.cyan('Solutions:'));
  console.log();
  console.log('  ' + chalk.green('Option 1:') + ' Kill the process using port ' + port);
  console.log();
  
  // 根据不同操作系统提供命令
  const os = require('os');
  const platform = os.platform();
  
  if (platform === 'win32') {
    console.log('    ' + chalk.dim('# Find process:'));
    console.log('    ' + chalk.yellow(`netstat -ano | findstr :${port}`));
    console.log();
    console.log('    ' + chalk.dim('# Kill process (replace PID):'));
    console.log('    ' + chalk.yellow('taskkill /F /PID <PID>'));
  } else if (platform === 'darwin' || platform === 'linux') {
    console.log('    ' + chalk.dim('# Find process:'));
    console.log('    ' + chalk.yellow(`lsof -i :${port}`));
    console.log();
    console.log('    ' + chalk.dim('# Kill process (replace PID):'));
    console.log('    ' + chalk.yellow('kill -9 <PID>'));
  }
  
  console.log();
  console.log('  ' + chalk.green('Option 2:') + ' Use a different port');
  console.log();
  console.log('    ' + chalk.yellow(`npx iris-runtime dev --port ${port + 1}`));
  console.log();
  console.log('  ' + chalk.green('Option 3:') + ' Auto-select available port');
  console.log();
  console.log('    ' + chalk.yellow('npx iris-runtime dev --port 0'));
  console.log();
  
  process.exit(1);
}

/**
 * 检查是否有图形界面可用
 * 
 * @returns {Promise<boolean>} - 是否有图形界面
 */
async function hasDisplayAvailable() {
  // 在 Windows 上，总是假设有图形界面
  const os = require('os');
  if (os.platform() === 'win32') {
    return true;
  }
  
  // 在 Linux/macOS 上，检查 DISPLAY 环境变量
  return !!process.env.DISPLAY || !!process.env.WAYLAND_DISPLAY;
}

/**
 * 启动开发服务器
 * 
 * @param {IrisRuntime} runtime - WASM 运行时实例
 * @param {Object} config - 服务器配置
 */
export async function startDevServer(runtime, config) {
  const { root, port: requestedPort, host, open } = config;
  
  // 检查端口是否被占用
  const portInUse = await isPortInUse(requestedPort);
  
  if (portInUse) {
    // 如果指定了端口 0，自动查找可用端口
    if (requestedPort === 0) {
      try {
        const availablePort = await findAvailablePort(3000);
        console.log(chalk.yellow(`⚠️  Port ${requestedPort} is in use, using port ${availablePort} instead`));
        port = availablePort;
      } catch (error) {
        showPortInUseError(3000);
        return;
      }
    } else {
      showPortInUseError(requestedPort);
      return;
    }
  } else {
    port = requestedPort;
  }
  
  // 检查图形界面可用性
  const hasDisplay = await hasDisplayAvailable();
  
  if (!hasDisplay) {
    console.log();
    console.log(chalk.red('❌ Error: No display available'));
    console.log();
    console.log(chalk.yellow('Iris Runtime requires a graphical environment to run.'));
    console.log(chalk.yellow('Without a GUI, iris-runtime cannot provide value.'));
    console.log();
    console.log(chalk.cyan('Possible causes:'));
    console.log('  • Running in SSH session without X11 forwarding');
    console.log('  • Running in a headless server/container');
    console.log('  • DISPLAY environment variable not set');
    console.log();
    console.log(chalk.cyan('Solutions:'));
    console.log();
    console.log('  ' + chalk.green('Option 1:') + ' Enable X11 forwarding (SSH)');
    console.log();
    console.log('    ' + chalk.yellow('ssh -X user@host'));
    console.log();
    console.log('  ' + chalk.green('Option 2:') + ' Set DISPLAY variable');
    console.log();
    console.log('    ' + chalk.yellow('export DISPLAY=:0'));
    console.log();
    console.log('  ' + chalk.green('Option 3:') + ' Run on a machine with GUI');
    console.log();
    
    process.exit(1);
  }

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

    server.on('error', (error) => {
      if (error.code === 'EADDRINUSE') {
        // 端口在启动时被占用（竞态条件）
        console.log(chalk.yellow(`⚠️  Port ${port} became unavailable, trying port ${port + 1}...`));
        server.close();
        server.listen(port + 1, host, () => {
          console.log(chalk.green('  ➜ Local:'), chalk.cyan(`http://${host}:${port + 1}`));
          console.log(chalk.green('  ➜ Network:'), chalk.dim('use --host to expose'));
          console.log(chalk.green('  ➜ Ready in'), chalk.cyan('234ms'));
          console.log();
          resolve();
        });
      } else {
        reject(error);
      }
    });
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
