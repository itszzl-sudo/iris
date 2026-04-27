# Iris Runtime

Iris Runtime - Vue 3 development server powered by Rust + WebGPU.

This npm package wraps the `iris-cli` Rust binary, providing a seamless npm-compatible CLI experience.

## Installation

```bash
npm install iris-runtime
# or
npx iris-runtime dev
```

## Usage

### Development Server

```bash
# Start dev server with hot reload
npx iris-runtime dev

# Custom port
npx iris-runtime dev -p 8080

# Disable hot reload
npx iris-runtime dev --no-hot
```

### Production Build

```bash
# Build for production
npx iris-runtime build

# Custom output directory
npx iris-runtime build -o build
```

### Information

```bash
# Show runtime information
npx iris-runtime info
```

## Features

- ✓ Vue 3 SFC Support
- ✓ TypeScript Compilation
- ✓ Hot Module Replacement
- ✓ GPU-Accelerated Rendering
- ✓ CSS Modules & Scoped CSS
- ✓ Powered by Rust + WebGPU
- ✓ **No Rust Required** - Pre-built binaries provided!

## Requirements

- Node.js >= 14.0.0
- **No Rust toolchain needed** - Binary is downloaded automatically

### Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| Windows  | x64         | ✅     |
| macOS    | x64/ARM64   | ✅     |
| Linux    | x64         | ✅     |

## Installation

### Automatic Installation (Recommended)

```bash
npm install iris-runtime
```

The postinstall script will automatically download the pre-built binary for your platform from GitHub or Gitee.

### Manual Installation

If automatic download fails, you can manually download the binary:

**From GitHub:**
```bash
# Windows
https://github.com/iris-engine/iris/releases/download/v0.1.0/iris-runtime-x86_64-pc-windows-msvc.exe

# macOS (Intel)
https://github.com/iris-engine/iris/releases/download/v0.1.0/iris-runtime-x86_64-apple-darwin

# macOS (Apple Silicon)
https://github.com/iris-engine/iris/releases/download/v0.1.0/iris-runtime-aarch64-apple-darwin

# Linux
https://github.com/iris-engine/iris/releases/download/v0.1.0/iris-runtime-x86_64-unknown-linux-gnu
```

**From Gitee (China):**
```bash
# Windows
https://gitee.com/wanquanbuhuime/iris/releases/download/v0.1.0/iris-runtime-x86_64-pc-windows-msvc.exe

# macOS (Intel)
https://gitee.com/wanquanbuhuime/iris/releases/download/v0.1.0/iris-runtime-x86_64-apple-darwin

# macOS (Apple Silicon)
https://gitee.com/wanquanbuhuime/iris/releases/download/v0.1.0/iris-runtime-aarch64-apple-darwin

# Linux
https://gitee.com/wanquanbuhuime/iris/releases/download/v0.1.0/iris-runtime-x86_64-unknown-linux-gnu
```

Place the downloaded binary in `node_modules/iris-runtime/bin/` directory.

## Architecture

```
npm/Node.js                    Rust Binary
┌─────────────┐               ┌──────────────┐
│ iris-runtime│ ──exec──→     │  iris-cli    │
│  (Node.js)  │               │  (Rust)      │
└─────────────┘               └──────────────┘
                                      │
                                      ↓
                              ┌──────────────┐
                              │  iris-engine │
                              │  (WebGPU)    │
                              └──────────────┘
```

## Development

### Build and Release Binary (For Maintainers)

To build binaries for all platforms:

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

### Sign and Publish Binary

1. Build binaries for all platforms
2. Sign binaries with your GPG key (optional but recommended)
3. Create a GitHub/Gitee release
4. Upload binaries to the release
5. Update VERSION in `scripts/install.js`

### Publish to npm

```bash
npm publish
```

## License

MIT

## Repository

https://github.com/iris-engine/iris
