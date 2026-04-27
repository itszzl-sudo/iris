<template>
  <div class="app">
    <header class="header">
      <h1>🎨 Iris Engine</h1>
      <p class="subtitle">Next-Gen Frontend Runtime with Vue 3 Support</p>
    </header>

    <main class="content">
      <div class="card">
        <h2>✨ Features</h2>
        <ul>
          <li>🚀 Rust + WebGPU 渲染</li>
          <li>🎯 Vue 3 SFC 支持</li>
          <li>⚡ 高性能 GPU 渲染管线</li>
          <li>🔥 热重载支持</li>
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
    </main>

    <footer class="footer">
      <p>Built with ❤️ using Rust + WebGPU</p>
    </footer>
  </div>
</template>

<script>
export default {
  name: 'App',
  data() {
    return {
      version: '1.0.0'
    }
  },
  mounted() {
    console.log('Iris Engine App mounted!')
  }
}
</script>

<style>
.app {
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  color: white;
  padding: 40px 20px;
  box-sizing: border-box;
}

.header {
  text-align: center;
  margin-bottom: 40px;
}

.header h1 {
  font-size: 56px;
  margin: 0 0 16px 0;
  font-weight: 800;
  text-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.subtitle {
  font-size: 20px;
  margin: 0;
  opacity: 0.9;
  font-weight: 400;
}

.content {
  max-width: 800px;
  margin: 0 auto;
  display: flex;
  gap: 24px;
  flex-wrap: wrap;
  justify-content: center;
}

.card {
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(10px);
  border-radius: 16px;
  padding: 32px;
  flex: 1;
  min-width: 280px;
  max-width: 380px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
}

.card h2 {
  font-size: 28px;
  margin: 0 0 20px 0;
  font-weight: 700;
}

.card ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

.card li {
  font-size: 18px;
  padding: 12px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.card li:last-child {
  border-bottom: none;
}

.tech-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.tech-badge {
  background: rgba(255, 255, 255, 0.25);
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  border: 1px solid rgba(255, 255, 255, 0.3);
}

.footer {
  text-align: center;
  margin-top: 60px;
  padding-top: 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.2);
}

.footer p {
  font-size: 16px;
  opacity: 0.8;
  margin: 0;
}
</style>
