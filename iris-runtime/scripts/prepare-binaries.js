#!/usr/bin/env node

/**
 * Prepare binaries script (For Maintainers Only)
 * 
 * Builds and copies pre-built binaries for all platforms
 * Run this before publishing to npm
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const chalk = require('chalk');

const ROOT_DIR = path.join(__dirname, '..');
const BINARIES_DIR = path.join(ROOT_DIR, 'binaries');
const CARGO_WORKSPACE = path.join(ROOT_DIR, '..');

// Platform targets
const TARGETS = [
  { name: 'Windows x64', target: 'x86_64-pc-windows-msvc', ext: '.exe' },
  { name: 'macOS Intel', target: 'x86_64-apple-darwin', ext: '' },
  { name: 'macOS Apple Silicon', target: 'aarch64-apple-darwin', ext: '' },
  { name: 'Linux x64', target: 'x86_64-unknown-linux-gnu', ext: '' }
];

/**
 * Check if Rust toolchain is available
 */
function checkRust() {
  try {
    execSync('cargo --version', { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

/**
 * Build binary for a specific target
 */
function buildTarget(target) {
  console.log(chalk.blue(`\nBuilding for ${target.name}...`));
  
  try {
    // Add target if not already installed
    try {
      execSync(`rustup target add ${target.target}`, { 
        cwd: CARGO_WORKSPACE, 
        stdio: 'ignore' 
      });
    } catch {
      // Target might already be installed
    }
    
    // Build in release mode
    const cmd = `cargo build --release -p iris-cli --target ${target.target}`;
    console.log(`  ${cmd}`);
    
    execSync(cmd, {
      cwd: CARGO_WORKSPACE,
      stdio: 'inherit'
    });
    
    // Copy binary
    const binaryName = `iris-${target.target}${target.ext}`;
    const sourcePath = path.join(CARGO_WORKSPACE, 'target', target.target, 'release', `iris${target.ext}`);
    const destPath = path.join(BINARIES_DIR, binaryName);
    
    if (fs.existsSync(sourcePath)) {
      fs.copyFileSync(sourcePath, destPath);
      
      const stats = fs.statSync(destPath);
      const sizeMB = (stats.size / (1024 * 1024)).toFixed(2);
      
      console.log(chalk.green(`  ✓ Built: ${binaryName} (${sizeMB} MB)`));
      return true;
    } else {
      console.error(chalk.red(`  ✗ Binary not found at: ${sourcePath}`));
      return false;
    }
  } catch (error) {
    console.error(chalk.red(`  ✗ Build failed for ${target.name}`));
    console.error(chalk.red(`    ${error.message}`));
    return false;
  }
}

/**
 * Main function
 */
function prepareBinaries() {
  console.log(chalk.blue('╔═══════════════════════════════════════════╗'));
  console.log(chalk.blue('║  Iris Runtime Binary Preparation Tool    ║'));
  console.log(chalk.blue('╚═══════════════════════════════════════════╝\n'));
  
  // Check Rust
  if (!checkRust()) {
    console.error(chalk.red('✗ Rust toolchain not found!'));
    console.error(chalk.red('  Please install Rust from: https://rustup.rs/\n'));
    process.exit(1);
  }
  
  // Ensure binaries directory exists
  if (!fs.existsSync(BINARIES_DIR)) {
    fs.mkdirSync(BINARIES_DIR, { recursive: true });
  }
  
  // Build for each target
  const results = [];
  for (const target of TARGETS) {
    results.push({
      name: target.name,
      success: buildTarget(target)
    });
  }
  
  // Summary
  console.log(chalk.blue('\n╔═══════════════════════════════════════════╗'));
  console.log(chalk.blue('║  Build Summary                            ║'));
  console.log(chalk.blue('╚═══════════════════════════════════════════╝\n'));
  
  const successCount = results.filter(r => r.success).length;
  
  for (const result of results) {
    const icon = result.success ? '✓' : '✗';
    const color = result.success ? 'green' : 'red';
    console.log(chalk[color](`  ${icon} ${result.name}`));
  }
  
  console.log();
  
  if (successCount === TARGETS.length) {
    console.log(chalk.green(`✓ All ${successCount} binaries built successfully!`));
    console.log(chalk.green(`  Location: ${BINARIES_DIR}`));
    console.log(chalk.green('\nYou can now publish to npm:'));
    console.log(chalk.green('  npm publish\n'));
  } else {
    console.warn(chalk.yellow(`⚠ ${successCount}/${TARGETS.length} binaries built successfully`));
    console.warn(chalk.yellow('  Check errors above and retry\n'));
  }
}

// Run
prepareBinaries();
