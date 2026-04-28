#!/usr/bin/env node

/**
 * Iris Runtime CLI
 * 
 * Vue 3 development server powered by WebAssembly
 * 
 * Usage:
 *   npx iris-runtime dev
 */

import { IrisRuntime } from '../pkg/iris_runtime.js';
import { startDevServer } from '../lib/dev-server.js';
import { program } from 'commander';
import chalk from 'chalk';

// 创建 WASM 运行时实例
const runtime = new IrisRuntime();

program
  .name('iris-runtime')
  .description('Iris Runtime - Vue 3 development server')
  .version(IrisRuntime.version());

// 唯一的命令：dev
program
  .command('dev')
  .description('Start development server with HMR')
  .option('-p, --port <port>', 'Server port', '3000')
  .option('--host <host>', 'Server host', 'localhost')
  .option('--open', 'Open browser automatically', true)
  .action(async (options) => {
    console.log(chalk.cyan('🚀 Starting Iris Runtime dev server...\n'));
    
    const config = {
      port: parseInt(options.port),
      host: options.host,
      open: options.open,
      root: process.cwd(),
    };

    try {
      await startDevServer(runtime, config);
    } catch (error) {
      console.error(chalk.red('\n❌ Failed to start dev server:'));
      console.error(error.message);
      process.exit(1);
    }
  });

program.parse();
