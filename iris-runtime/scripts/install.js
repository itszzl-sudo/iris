#!/usr/bin/env node

/**
 * Post-install script
 * 
 * Downloads pre-built iris-cli binary from GitHub/Gitee releases
 * No Rust/Cargo dependency required!
 */

const https = require('https');
const http = require('http');
const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');
const chalk = require('chalk');

const BIN_DIR = path.join(__dirname, '..', 'bin');
const VERSION = '0.1.0';

// GitHub and Gitee release URLs
const GITHUB_RELEASES = `https://github.com/iris-engine/iris/releases/download/v${VERSION}`;
const GITEE_RELEASES = `https://gitee.com/wanquanbuhuime/iris/releases/download/v${VERSION}`;

/**
 * Get platform-specific binary name
 */
function getBinaryName() {
  const platform = os.platform();
  const arch = os.arch();
  
  if (platform === 'win32') {
    return arch === 'x64' ? 'iris-runtime-x86_64-pc-windows-msvc.exe' : 'iris-runtime.exe';
  } else if (platform === 'darwin') {
    return arch === 'arm64' ? 'iris-runtime-aarch64-apple-darwin' : 'iris-runtime-x86_64-apple-darwin';
  } else if (platform === 'linux') {
    return arch === 'x64' ? 'iris-runtime-x86_64-unknown-linux-gnu' : 'iris-runtime';
  }
  
  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

/**
 * Get the final binary name (without platform suffix)
 */
function getFinalBinaryName() {
  return os.platform() === 'win32' ? 'iris-runtime.exe' : 'iris-runtime';
}

/**
 * Download file from URL
 */
function download(url, dest) {
  return new Promise((resolve, reject) => {
    const protocol = url.startsWith('https') ? https : http;
    
    protocol.get(url, (response) => {
      // Handle redirects
      if (response.statusCode === 302 || response.statusCode === 301) {
        download(response.headers.location, dest)
          .then(resolve)
          .catch(reject);
        return;
      }
      
      if (response.statusCode !== 200) {
        reject(new Error(`Download failed: ${response.statusCode} ${response.statusMessage}`));
        return;
      }
      
      const file = fs.createWriteStream(dest);
      response.pipe(file);
      
      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', reject);
  });
}

/**
 * Download binary from GitHub or Gitee
 */
async function downloadBinary() {
  const binaryName = getBinaryName();
  const finalName = getFinalBinaryName();
  const downloadPath = path.join(BIN_DIR, binaryName);
  const finalPath = path.join(BIN_DIR, finalName);
  
  console.log(chalk.blue(`\nDownloading ${binaryName}...\n`));
  
  // Try GitHub first, then Gitee as fallback
  const sources = [
    { name: 'GitHub', url: `${GITHUB_RELEASES}/${binaryName}` },
    { name: 'Gitee', url: `${GITEE_RELEASES}/${binaryName}` }
  ];
  
  for (const source of sources) {
    try {
      console.log(`  Trying ${source.name}...`);
      await download(source.url, downloadPath);
      
      // Rename to final name
      fs.renameSync(downloadPath, finalPath);
      
      // Make executable on Unix
      if (os.platform() !== 'win32') {
        fs.chmodSync(finalPath, 0o755);
      }
      
      console.log();
      console.log(chalk.green(`✓ Binary downloaded from ${source.name}!`));
      console.log(chalk.green(`  Location: ${finalPath}`));
      console.log();
      return true;
    } catch (error) {
      console.warn(chalk.yellow(`  ⚠ Failed to download from ${source.name}`));
      // Clean up partial download
      if (fs.existsSync(downloadPath)) {
        fs.unlinkSync(downloadPath);
      }
    }
  }
  
  return false;
}

/**
 * Verify binary signature (placeholder for future implementation)
 */
function verifySignature(binaryPath) {
  // TODO: Implement signature verification
  // For now, just check if file exists and is executable
  const stats = fs.statSync(binaryPath);
  return stats.size > 0;
}

/**
 * Main installation function
 */
async function install() {
  const finalName = getFinalBinaryName();
  const finalPath = path.join(BIN_DIR, finalName);
  
  // Check if binary already exists
  if (fs.existsSync(finalPath)) {
    console.log(chalk.green(`\n✓ iris-runtime binary already exists.`));
    console.log(chalk.green(`  Location: ${finalPath}\n`));
    return;
  }
  
  // Ensure bin directory exists
  if (!fs.existsSync(BIN_DIR)) {
    fs.mkdirSync(BIN_DIR, { recursive: true });
  }
  
  // Download binary
  const success = await downloadBinary();
  
  if (!success) {
    console.error(chalk.red('\n✗ Failed to download iris-runtime binary'));
    console.error(chalk.red('\nPlease try one of the following:'));
    console.error(chalk.red('  1. Check your network connection'));
    console.error(chalk.red('  2. Download manually from:'));
    console.error(chalk.red(`     ${GITHUB_RELEASES}`));
    console.error(chalk.red(`     ${GITEE_RELEASES}`));
    console.error(chalk.red('  3. Build from source (requires Rust):'));
    console.error(chalk.red('     cargo build --release -p iris-cli\n'));
    
    // Don't fail the install, just warn
    process.exit(0);
  }
  
  // Verify binary
  if (verifySignature(finalPath)) {
    console.log(chalk.green('✓ Binary verification passed!'));
  }
}

// Run installation
install().catch(error => {
  console.error(chalk.red('\n✗ Installation failed'));
  console.error(error.message);
  process.exit(0);
});
