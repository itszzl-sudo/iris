#!/usr/bin/env node

/**
 * Iris Runtime CLI
 * 
 * Node.js wrapper for iris-cli (Rust binary)
 * Usage: npx iris-runtime dev/build/info
 */

const { Command } = require('commander');
const chalk = require('chalk');
const path = require('path');
const fs = require('fs');
const { execFileSync } = require('child_process');
const which = require('which');

const program = new Command();

// Get the binary path
function getBinaryPath() {
  // Try to find iris-runtime binary in PATH
  try {
    return which.sync('iris-runtime');
  } catch (e) {
    // If not in PATH, try to find in node_modules/.bin
    const localBin = path.join(__dirname, '..', 'node_modules', '.bin', 'iris-runtime');
    if (fs.existsSync(localBin)) {
      return localBin;
    }
    
    // Fallback: build from source
    console.warn(chalk.yellow('⚠ iris-runtime binary not found.'));
    console.warn(chalk.yellow('  Run: npm install iris-runtime\n'));
    process.exit(1);
  }
}

// Print banner
function printBanner() {
  console.log();
  console.log(chalk.cyan('╔══════════════════════════════════════════════════════════╗'));
  console.log(chalk.cyan('║          Iris Runtime - Vue 3 Dev Server                 ║'));
  console.log(chalk.cyan('║          Powered by Rust + WebGPU                        ║'));
  console.log(chalk.cyan('╚══════════════════════════════════════════════════════════╝'));
  console.log();
}

program
  .name('iris-runtime')
  .description('Iris Runtime CLI - Vue 3 development server with Rust+WebGPU')
  .version('0.1.0');

// dev command
program
  .command('dev')
  .description('Start development server with hot reload')
  .option('-r, --root <path>', 'Project root directory', '.')
  .option('-p, --port <number>', 'Port to listen on', '3000')
  .option('--no-hot', 'Disable hot reload')
  .action((options) => {
    printBanner();
    console.log(chalk.blue('Starting development server...'));
    console.log(`  Root: ${options.root}`);
    console.log(`  Port: ${options.port}`);
    console.log(`  Hot Reload: ${options.hot ? 'enabled' : 'disabled'}`);
    console.log();

    try {
      const binary = getBinaryPath();
      execFileSync(binary, ['dev', ...process.argv.slice(3)], {
        stdio: 'inherit',
        cwd: process.cwd()
      });
    } catch (error) {
      console.error(chalk.red('✗ Failed to start dev server'));
      console.error(error.message);
      process.exit(1);
    }
  });

// build command
program
  .command('build')
  .description('Build for production')
  .option('-r, --root <path>', 'Project root directory', '.')
  .option('-o, --out <path>', 'Output directory', 'dist')
  .option('--no-minify', 'Disable minification')
  .action((options) => {
    printBanner();
    console.log(chalk.blue('Building for production...'));
    console.log(`  Root: ${options.root}`);
    console.log(`  Output: ${options.out}`);
    console.log(`  Minify: ${options.minify !== false ? 'enabled' : 'disabled'}`);
    console.log();

    try {
      const binary = getBinaryPath();
      execFileSync(binary, ['build', ...process.argv.slice(3)], {
        stdio: 'inherit',
        cwd: process.cwd()
      });
    } catch (error) {
      console.error(chalk.red('✗ Build failed'));
      console.error(error.message);
      process.exit(1);
    }
  });

// info command
program
  .command('info')
  .description('Show runtime information')
  .action(() => {
    printBanner();
    console.log(chalk.yellow('Iris Runtime Information'));
    console.log();
    console.log(`  Version: 0.1.0`);
    console.log(`  Binary: ${getBinaryPath()}`);
    console.log(`  Node.js: ${process.version}`);
    console.log();
    console.log(chalk.green('Features:'));
    console.log('  ✓ Vue 3 SFC Support');
    console.log('  ✓ TypeScript Compilation');
    console.log('  ✓ Hot Module Replacement');
    console.log('  ✓ GPU-Accelerated Rendering');
    console.log('  ✓ CSS Modules & Scoped CSS');
    console.log();
  });

program.parse(process.argv);
