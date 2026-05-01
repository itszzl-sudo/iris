# @irisverse/iris

Iris CLI — Vue 3 development server powered by Rust + WebGPU.

Part of the [irisverse](https://www.npmjs.com/org/irisverse) ecosystem on npm.

## Installation

```bash
npm install -g @irisverse/iris
```

## Usage

### Development Server

```bash
# Start dev server with hot reload
iris dev

# Custom port
iris dev -p 8080

# Disable hot reload
iris dev --no-hot
```

### Production Build

```bash
# Build for production
iris build

# Custom output directory
iris build -o build
```

### Information

```bash
# Show runtime information
iris info
```

## Features

- ✓ Vue 3 SFC Support
- ✓ TypeScript Compilation
- ✓ Hot Module Replacement
- ✓ GPU-Accelerated Rendering
- ✓ CSS Modules & Scoped CSS
- ✓ Powered by Rust + WebGPU
- ✓ **No Rust Required** - Pre-built binaries included!
- ✓ **No Network Download** - Binary bundled in package!

## Requirements

- Node.js >= 14.0.0
- **No Rust toolchain needed** - Binary is included in the package
- **No network access needed** - No download during install

### Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| Windows  | x64         | ✅     |
| macOS    | x64/ARM64   | ✅     |
| Linux    | x64         | ✅     |

## Architecture

```
npm/Node.js                    Rust Binary
┌─────────────┐               ┌──────────────┐
│ @irisverse/ │ ──exec──→     │  iris        │
│  iris       │               │  (Rust CLI)  │
│  (Node.js)  │               └──────────────┘
└─────────────┘                      │
                                     ↓
                             ┌──────────────┐
                             │  iris-engine │
                             │  (WebGPU)    │
                             └──────────────┘
```

## Development

### For Maintainers: Building Binaries

To build binaries for all platforms before publishing:

```bash
cd iris-runtime
npm run prepare-binaries
```

This will:
1. Build `iris-cli` for all supported platforms
2. Copy binaries to `binaries/` directory
3. Verify all builds completed successfully

**Requirements:**
- Rust toolchain installed
- Cross-compilation targets installed (automatically handled)

**Binary Naming Convention:**

| Platform | Target Triple | Binary Name |
|----------|--------------|-------------|
| Windows x64 | `x86_64-pc-windows-msvc` | `iris-x86_64-pc-windows-msvc.exe` |
| macOS Intel | `x86_64-apple-darwin` | `iris-x86_64-apple-darwin` |
| macOS ARM | `aarch64-apple-darwin` | `iris-aarch64-apple-darwin` |
| Linux x64 | `x86_64-unknown-linux-gnu` | `iris-x86_64-unknown-linux-gnu` |

### Manual Build (Single Platform)

```bash
# Windows
cargo build --release -p iris-cli --target x86_64-pc-windows-msvc

# macOS (Intel)
cargo build --release -p iris-cli --target x86_64-apple-darwin

# macOS (Apple Silicon)
cargo build --release -p iris-cli --target aarch64-apple-darwin

# Linux
cargo build --release -p iris-cli --target x86_64-unknown-linux-gnu
```

### Publishing to npm

```bash
# 1. Build all binaries
npm run prepare-binaries

# 2. Verify binaries/ directory contains all platforms
ls binaries/

# 3. Publish
npm publish --access public
```

**Important:** The `binaries/` directory is included in the npm package (see `files` in package.json).

## License

MIT

## Repository

https://github.com/itszzl-sudo/iris
