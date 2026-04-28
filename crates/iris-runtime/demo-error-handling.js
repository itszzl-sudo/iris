#!/usr/bin/env node

/**
 * 演示端口占用错误提示
 * 
 * 使用方法:
 *   node demo-error-handling.js
 */

import chalk from 'chalk';

console.log();
console.log(chalk.cyan('═══════════════════════════════════════════════════════════'));
console.log(chalk.cyan('  Iris Runtime - Error Handling Demo'));
console.log(chalk.cyan('═══════════════════════════════════════════════════════════'));
console.log();

// 演示 1: 端口占用错误（Windows）
console.log(chalk.yellow('─'.repeat(60)));
console.log(chalk.yellow('Scenario 1: Port 3000 is in use (Windows)'));
console.log(chalk.yellow('─'.repeat(60)));
console.log();
console.log(chalk.red('❌ Error: Port 3000 is already in use'));
console.log();
console.log(chalk.yellow('Possible causes:'));
console.log('  • Another instance of iris-runtime is running');
console.log('  • Another application is using port 3000');
console.log();
console.log(chalk.cyan('Solutions:'));
console.log();
console.log('  ' + chalk.green('Option 1:') + ' Kill the process using port 3000');
console.log();
console.log('    ' + chalk.dim('# Find process:'));
console.log('    ' + chalk.yellow('netstat -ano | findstr :3000'));
console.log();
console.log('    ' + chalk.dim('# Kill process (replace PID):'));
console.log('    ' + chalk.yellow('taskkill /F /PID <PID>'));
console.log();
console.log('  ' + chalk.green('Option 2:') + ' Use a different port');
console.log();
console.log('    ' + chalk.yellow('npx iris-runtime dev --port 3001'));
console.log();
console.log('  ' + chalk.green('Option 3:') + ' Auto-select available port');
console.log();
console.log('    ' + chalk.yellow('npx iris-runtime dev --port 0'));
console.log();

// 演示 2: 端口占用错误（macOS/Linux）
console.log(chalk.yellow('─'.repeat(60)));
console.log(chalk.yellow('Scenario 2: Port 3000 is in use (macOS/Linux)'));
console.log(chalk.yellow('─'.repeat(60)));
console.log();
console.log(chalk.red('❌ Error: Port 3000 is already in use'));
console.log();
console.log(chalk.yellow('Possible causes:'));
console.log('  • Another instance of iris-runtime is running');
console.log('  • Another application is using port 3000');
console.log();
console.log(chalk.cyan('Solutions:'));
console.log();
console.log('  ' + chalk.green('Option 1:') + ' Kill the process using port 3000');
console.log();
console.log('    ' + chalk.dim('# Find process:'));
console.log('    ' + chalk.yellow('lsof -i :3000'));
console.log();
console.log('    ' + chalk.dim('# Kill process (replace PID):'));
console.log('    ' + chalk.yellow('kill -9 <PID>'));
console.log();
console.log('  ' + chalk.green('Option 2:') + ' Use a different port');
console.log();
console.log('    ' + chalk.yellow('npx iris-runtime dev --port 3001'));
console.log();
console.log('  ' + chalk.green('Option 3:') + ' Auto-select available port');
console.log();
console.log('    ' + chalk.yellow('npx iris-runtime dev --port 0'));
console.log();

// 演示 3: 没有图形界面
console.log(chalk.yellow('─'.repeat(60)));
console.log(chalk.yellow('Scenario 3: No display available'));
console.log(chalk.yellow('─'.repeat(60)));
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

// 演示 4: 自动端口选择
console.log(chalk.yellow('─'.repeat(60)));
console.log(chalk.yellow('Scenario 4: Auto-select port (--port 0)'));
console.log(chalk.yellow('─'.repeat(60)));
console.log();
console.log(chalk.yellow('⚠️  Port 3000 is in use, using port 3001 instead'));
console.log();
console.log(chalk.green('  ➜ Local:'), chalk.cyan('http://localhost:3001'));
console.log(chalk.green('  ➜ Network:'), chalk.dim('use --host to expose'));
console.log(chalk.green('  ➜ Ready in'), chalk.cyan('234ms'));
console.log();

console.log(chalk.cyan('═══════════════════════════════════════════════════════════'));
console.log(chalk.cyan('  Demo Complete'));
console.log(chalk.cyan('═══════════════════════════════════════════════════════════'));
console.log();
