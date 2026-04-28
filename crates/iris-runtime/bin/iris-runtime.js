#!/usr/bin/env node

/**
 * Iris Runtime - Vue 3 Development Server
 * 
 * Usage:
 *   npx iris-runtime dev
 * 
 * Options (via environment variables):
 *   IRIS_PORT - Server port (default: 3000)
 *   IRIS_HOST - Server host (default: localhost)
 */

import { IrisRuntime } from '../pkg/iris_runtime.js';
import { startDevServer } from '../lib/dev-server.js';
import chalk from 'chalk';

// 解析命令行参数
const args = process.argv.slice(2);

// 检查是否是 dev 命令
if (args[0] !== 'dev') {
  console.log(chalk.yellow('Usage: npx iris-runtime dev'));
  console.log();
  console.log(chalk.cyan('Options:'));
  console.log('  --port <number>  Server port (default: 3000, use 0 for auto)');
  console.log('  --host <string>  Server host (default: localhost)');
  console.log('  --no-open        Do not open browser automatically');
  console.log();
  process.exit(0);
}

// 解析选项
const options = {
  port: 3000,
  host: 'localhost',
  open: true,
};

for (let i = 1; i < args.length; i++) {
  if (args[i] === '--port' && args[i + 1]) {
    const portValue = args[i + 1];
    // 支持 '0' 表示自动选择端口
    options.port = portValue === '0' ? 0 : parseInt(portValue);
    i++;
  } else if (args[i] === '--host' && args[i + 1]) {
    options.host = args[i + 1];
    i++;
  } else if (args[i] === '--no-open') {
    options.open = false;
  }
}

// 创建 WASM 运行时实例
const runtime = new IrisRuntime();

// 启动开发服务器
const config = {
  port: options.port,
  host: options.host,
  open: options.open,
  root: process.cwd(),
};

console.log(chalk.cyan('🚀 Starting Iris Runtime dev server...\n'));

startDevServer(runtime, config).catch(error => {
  console.error(chalk.red('\n❌ Failed to start dev server:'));
  console.error(error.message);
  process.exit(1);
});
