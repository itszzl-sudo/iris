<template>
  <div class="app">
    <header class="header">
      <h1>🎨 Iris Runtime</h1>
      <p class="subtitle">TypeScript Module Support Test</p>
    </header>

    <main class="content">
      <div class="card">
        <h2>✅ Features</h2>
        <ul>
          <li>🚀 Rust + WebGPU 渲染</li>
          <li>🎯 Vue 3 SFC 支持</li>
          <li>📝 TypeScript 模块语法</li>
          <li>⚡ GPU 加速渲染</li>
        </ul>
      </div>

      <div class="card">
        <h2>📊 Counter Demo</h2>
        <p class="counter">Count: {{ count }}</p>
        <button @click="increment" class="btn">Increment</button>
      </div>
    </main>
  </div>
</template>

<script lang="ts">
// TypeScript component (no import)
const count: number = 0

function increment(): void {
  console.log('Button clicked!')
}
</script>

<style>
.app {
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  padding: 40px 20px;
}

.header {
  text-align: center;
  margin-bottom: 40px;
}

.header h1 {
  font-size: 48px;
  margin: 0 0 10px 0;
}

.subtitle {
  font-size: 20px;
  opacity: 0.9;
}

.content {
  max-width: 800px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.card {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  border-radius: 16px;
  padding: 24px;
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.card h2 {
  margin: 0 0 16px 0;
}

.card ul {
  margin: 0;
  padding-left: 20px;
}

.card li {
  margin: 8px 0;
}

.counter {
  font-size: 24px;
  margin: 16px 0;
}

.btn {
  background: white;
  color: #667eea;
  border: none;
  padding: 12px 24px;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
}
</style>
