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

## Requirements

- Node.js >= 14.0.0
- Rust toolchain (for building from source)
  - Install: https://rustup.rs/

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

### Build from source

```bash
cd iris-runtime
npm install
npm run build-cli
```

### Publish to npm

```bash
npm publish
```

## License

MIT

## Repository

https://github.com/iris-engine/iris
