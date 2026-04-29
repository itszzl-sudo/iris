<template>
  <div class="app">
    <header class="header">
      <h1>Iris Runtime</h1>
      <p class="subtitle">GPU Accelerated Vue 3 Rendering</p>
    </header>

    <main class="content">
      <div class="card">
        <h2>Features</h2>
        <ul>
          <li>Rust + WebGPU Rendering aa</li>
          <li>Vue 3 SFC Support</li>
          <li>GPU Hardware Acceleration</li>
          <li>Hot Module Replacement</li>
          <li>Native Window Rendering</li>
        </ul>
      </div>

      <div class="card">
        <h2>Performance</h2>
        <div class="stats">
          <div class="stat">
            <span class="stat-value">5ms</span>
            <span class="stat-label">Frame Time</span>
          </div>
          <div class="stat">
            <span class="stat-value">60fps</span>
            <span class="stat-label">Render FPS</span>
          </div>
          <div class="stat">
            <span class="stat-value">75MB</span>
            <span class="stat-label">Memory</span>
          </div>
        </div>
      </div>

      <div class="card">
        <h2>Tech Stack</h2>
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
      <p>Built with Rust + WebGPU + Vue 3</p>
    </footer>
  </div>
</template>

<script>
// Empty script - just template rendering
console.log('App loaded')
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
  font-size: 24px;
}

.card ul {
  margin: 0;
  padding-left: 20px;
}

.card li {
  margin: 8px 0;
  font-size: 16px;
}

.stats {
  display: flex;
  gap: 20px;
  justify-content: space-around;
}

.stat {
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 32px;
  font-weight: 700;
  color: #4ade80;
}

.stat-label {
  display: block;
  font-size: 14px;
  opacity: 0.8;
  margin-top: 4px;
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

.footer {
  text-align: center;
  margin-top: 40px;
  opacity: 0.8;
  font-size: 14px;
}
</style>
