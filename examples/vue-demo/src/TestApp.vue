<template>
  <div class="app">
    <header class="header">
      <h1>🎨 Iris Runtime</h1>
      <p class="subtitle">Native Window Rendering with WebGPU</p>
    </header>

    <main class="content">
      <div class="card">
        <h2>✨ Features</h2>
        <ul>
          <li>🚀 Rust + WebGPU 渲染</li>
          <li>🎯 Vue 3 SFC 支持</li>
          <li>⚡ 高性能 GPU 渲染管线</li>
          <li>🔥 热重载支持</li>
          <li>💻 原生窗口渲染</li>
        </ul>
      </div>

      <div class="card">
        <h2>📦 Tech Stack</h2>
        <div class="tech-grid">
          <span class="tech-badge">Rust</span>
          <span class="tech-badge">WebGPU</span>
          <span class="tech-badge">Vue 3</span>
          <span class="tech-badge">wgpu</span>
          <span class="tech-badge">winit</span>
          <span class="tech-badge">Boa JS</span>
        </div>
      </div>

      <div class="card">
        <h2>🎮 Interactive Demo</h2>
        <p>Counter: <strong>{{ counter }}</strong></p>
        <button @click="increment" class="btn">Increment</button>
      </div>
    </main>

    <footer class="footer">
      <p>Built with ❤️ using Rust + WebGPU</p>
    </footer>
  </div>
</template>

<script>
export default {
  name: 'TestApp',
  data() {
    return {
      counter: 0
    }
  },
  methods: {
    increment() {
      this.counter++
    }
  },
  mounted() {
    console.log('TestApp mounted successfully!')
  }
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
  font-weight: 700;
}

.subtitle {
  font-size: 20px;
  opacity: 0.9;
  margin: 0;
}

.content {
  max-width: 800px;
  margin: 0 auto;
  display: grid;
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
  font-size: 24px;
}

.card ul {
  margin: 0;
  padding-left: 20px;
  list-style: none;
}

.card li {
  margin: 8px 0;
  font-size: 16px;
}

.tech-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.tech-badge {
  background: rgba(255, 255, 255, 0.2);
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 600;
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
  transition: transform 0.2s;
}

.btn:hover {
  transform: scale(1.05);
}

.footer {
  text-align: center;
  margin-top: 40px;
  opacity: 0.8;
  font-size: 14px;
}
</style>
