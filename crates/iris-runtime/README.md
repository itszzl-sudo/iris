# Iris CLI (crate: iris-runtime)

Vue 3 SFC compiler and dev server powered by Rust + WebAssembly.

This crate provides the WASM-based compilation engine for the Iris CLI (`@irisverse/iris`).

## Quick Start

```bash
# Install Iris CLI globally
npm install -g @irisverse/iris

# Start dev server
iris dev
```

### For Developers (Build WASM)

```bash
# Build WASM module (requires wasm-pack)
wasm-pack build --target nodejs --release

# Or use the build script
./build-wasm.sh          # Linux/macOS
.\build-wasm.ps1         # Windows

# Create npm package
npm pack

# Publish to npm
npm publish --access public
```

## Features

- ⚡ **Fast Compilation** - Vue SFC compilation via WebAssembly
- 🔥 **Hot Module Replacement** - Instant updates without page reload
- 📦 **Zero Configuration** - Works out of the box
- 🌐 **Cross-Platform** - Single WASM binary, runs everywhere
- 🚀 **Lightweight** - Only ~5MB vs 50MB+ native binaries

## Usage

### Basic

```bash
# Start development server
iris dev

# Custom port
iris dev --port 8080

# Custom host
iris dev --host 0.0.0.0
```

### With npm scripts

```json
{
  "scripts": {
    "dev": "iris dev",
    "build": "vite build"
  }
}
```

Then run:

```bash
npm run dev
```

## How It Works

```
npm install -g @irisverse/iris
    ↓
Download pre-built binary
    ↓
iris dev
    ↓
Start HTTP server + WebSocket (HMR)
    ↓
Watch .vue files
    ↓
Compile on-the-fly via WASM
    ↓
Serve to browser with live reload
```

## Architecture

```
@irisverse/iris (npm package)
├── bin/
│   └── iris.js                 # CLI entry point
├── scripts/
│   ├── install.js              # Post-install binary copy
│   └── prepare-binaries.js     # Build binaries for publish
├── binaries/                   # Pre-built native binaries
└── package.json
```

## API

### JavaScript API

```javascript
import { IrisRuntime } from '@irisverse/iris';

const runtime = new IrisRuntime();

// Compile Vue SFC
const compiled = runtime.compileSfc(`
  <template>
    <h1>{{ message }}</h1>
  </template>
  <script>
    export default {
      data() { return { message: 'Hello!' } }
    }
  </script>
`, 'App.vue');

console.log(JSON.parse(compiled));
// {
//   script: "export default { ... }",
//   styles: [{ code: "...", scoped: false }],
//   deps: []
// }
```

## Requirements

- Node.js >= 16.0.0
- npm >= 7.0.0

## License

MIT

## Repository

https://github.com/itszzl-sudo/iris
