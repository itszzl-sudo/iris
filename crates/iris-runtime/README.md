# Iris Runtime

Vue 3 development server powered by WebAssembly.

## Quick Start

### For Users (Vue Project)

```bash
# Install in your Vue project
npm install -D iris-runtime

# Start dev server
npx iris-runtime dev
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
npx iris-runtime dev

# Custom port
npx iris-runtime dev --port 8080

# Custom host
npx iris-runtime dev --host 0.0.0.0
```

### With npm scripts

```json
{
  "scripts": {
    "dev": "iris-runtime dev",
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
npm install -D iris-runtime
    ↓
Download WASM module (~5MB)
    ↓
npx iris-runtime dev
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
iris-runtime (npm package)
├── pkg/
│   ├── iris_runtime_bg.wasm    # WASM binary (Rust)
│   ├── iris_runtime.js         # JS bindings
│   └── iris_runtime.d.ts       # TypeScript types
├── bin/
│   └── iris-runtime.js         # CLI entry point
├── lib/
│   └── dev-server.js           # Dev server implementation
└── package.json
```

## API

### JavaScript API

```javascript
import { IrisRuntime } from 'iris-runtime';

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

https://github.com/iris-engine/iris
