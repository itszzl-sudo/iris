#!/usr/bin/env node

/**
 * Post-install script
 * 
 * Automatically builds the iris-cli Rust binary after npm install
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const chalk = require('chalk');

const ROOT_DIR = path.join(__dirname, '..');
const CLI_DIR = path.join(ROOT_DIR, '..', 'crates', 'iris-cli');
const BIN_DIR = path.join(ROOT_DIR, 'bin');

console.log(chalk.blue('\nBuilding iris-cli binary...\n'));

try {
  // Check if cargo is available
  execSync('cargo --version', { stdio: 'ignore' });
  
  // Build the binary in release mode
  console.log('  Running: cargo build --release -p iris-cli');
  execSync('cargo build --release -p iris-cli', {
    cwd: ROOT_DIR,
    stdio: 'inherit'
  });
  
  // Copy binary to bin directory
  const isWindows = process.platform === 'win32';
  const binaryName = isWindows ? 'iris-runtime.exe' : 'iris-runtime';
  const sourcePath = path.join(ROOT_DIR, 'target', 'release', binaryName);
  const destPath = path.join(BIN_DIR, binaryName);
  
  if (fs.existsSync(sourcePath)) {
    fs.copyFileSync(sourcePath, destPath);
    
    // Make executable on Unix
    if (!isWindows) {
      fs.chmodSync(destPath, 0o755);
    }
    
    console.log();
    console.log(chalk.green('✓ iris-cli binary built successfully!'));
    console.log(chalk.green(`  Location: ${destPath}`));
    console.log();
  } else {
    console.warn(chalk.yellow('⚠ Binary not found after build'));
  }
} catch (error) {
  console.error(chalk.red('\n✗ Failed to build iris-cli binary'));
  console.error(chalk.red('  Make sure Rust and Cargo are installed:'));
  console.error(chalk.red('  https://rustup.rs/\n'));
  console.error(error.message);
  
  // Don't fail the install, just warn
  process.exit(0);
}
