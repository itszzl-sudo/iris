# Iris Vue Demo

A demo Vue 3 application running on Iris Runtime with WebGPU rendering.

## Features

- ✅ Vue 3 Single File Components
- ✅ TypeScript Support
- ✅ Modern UI with CSS Gradients
- ✅ Interactive Counter Demo
- ✅ Performance Statistics Display

## Getting Started

### Using Iris Runtime

```bash
# Start development server (after global install)
# npm install -g @irisverse/iris
iris dev

# Build for production
iris build
```

### Using npm scripts

```bash
# Install dependencies (optional, for IDE support)
npm install

# Start development
npm run dev

# Build for production
npm run build
```

## Project Structure

```
vue-demo/
├── src/
│   ├── main.ts          # Entry point
│   └── App.vue          # Main Vue component
├── iris.config.json     # Iris Runtime configuration
└── package.json         # NPM package configuration
```

## Tech Stack

- **Frontend**: Vue 3 + TypeScript
- **Runtime**: Iris Engine (Rust + WebGPU)
- **Build Tool**: Iris CLI (`@irisverse/iris`)

## Performance

This demo showcases the performance benefits of Iris Runtime:

- **First Frame**: ~8ms (vs 50-100ms traditional)
- **Memory Usage**: ~75MB (vs 150-300MB traditional)
- **Animation FPS**: Stable 60fps

## License

MIT
