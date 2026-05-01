#!/usr/bin/env node

/**
 * Post-install script
 * 
 * Copies the pre-built binary from the package to bin/ directory
 * No network download required!
 */

const path = require('path');
const fs = require('fs');
const os = require('os');
const chalk = require('chalk');

const BIN_DIR = path.join(__dirname, '..', 'bin');
const BINARIES_DIR = path.join(__dirname, '..', 'binaries');

/**
 * Get platform-specific binary name
 */
function getBinaryName() {
  const platform = os.platform();
  const arch = os.arch();
  
  if (platform === 'win32') {
    return arch === 'x64' ? 'iris-x86_64-pc-windows-msvc.exe' : 'iris.exe';
  } else if (platform === 'darwin') {
    return arch === 'arm64' ? 'iris-aarch64-apple-darwin' : 'iris-x86_64-apple-darwin';
  } else if (platform === 'linux') {
    return arch === 'x64' ? 'iris-x86_64-unknown-linux-gnu' : 'iris';
  }
  
  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

/**
 * Get the final binary name (without platform suffix)
 */
function getFinalBinaryName() {
  return os.platform() === 'win32' ? 'iris.exe' : 'iris';
}

/**
 * Main installation function
 */
function install() {
  const binaryName = getBinaryName();
  const finalName = getFinalBinaryName();
  const sourcePath = path.join(BINARIES_DIR, binaryName);
  const destPath = path.join(BIN_DIR, finalName);
  
  // Check if source binary exists
  if (!fs.existsSync(sourcePath)) {
    console.error(chalk.red(`\n✗ Pre-built binary not found: ${binaryName}`));
    console.error(chalk.red(`\nExpected location: ${sourcePath}`));
    console.error(chalk.red('\nPlease ensure the binary is included in the package.'));
    console.error(chalk.red('For maintainers: Run npm run prepare-binaries\n'));
    process.exit(1);
  }
  
  // Ensure bin directory exists
  if (!fs.existsSync(BIN_DIR)) {
    fs.mkdirSync(BIN_DIR, { recursive: true });
  }
  
  try {
    // Copy binary
    fs.copyFileSync(sourcePath, destPath);
    
    // Make executable on Unix
    if (os.platform() !== 'win32') {
      fs.chmodSync(destPath, 0o755);
    }
    
    console.log(chalk.green(`\n✓ iris binary installed successfully!`));
    console.log(chalk.green(`  Platform: ${binaryName}`));
    console.log(chalk.green(`  Location: ${destPath}`));
    
    // Show binary info
    const stats = fs.statSync(destPath);
    const sizeMB = (stats.size / (1024 * 1024)).toFixed(2);
    console.log(chalk.green(`  Size: ${sizeMB} MB`));
    console.log();
    
  } catch (error) {
    console.error(chalk.red('\n✗ Failed to install binary'));
    console.error(error.message);
    process.exit(1);
  }
}

// Run installation
install();
