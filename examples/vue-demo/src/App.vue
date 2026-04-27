<template>
  <div id="app">
    <header class="header">
      <h1>{{ title }}</h1>
      <p class="subtitle">Powered by Iris Runtime + WebGPU</p>
    </header>

    <main class="content">
      <div class="card">
        <h2>🚀 Features</h2>
        <ul>
          <li v-for="feature in features" :key="feature">{{ feature }}</li>
        </ul>
      </div>

      <div class="card">
        <h2>📊 Performance</h2>
        <div class="stats">
          <div class="stat">
            <span class="stat-value">{{ stats.firstFrame }}ms</span>
            <span class="stat-label">First Frame</span>
          </div>
          <div class="stat">
            <span class="stat-value">{{ stats.fps }}fps</span>
            <span class="stat-label">Animation FPS</span>
          </div>
          <div class="stat">
            <span class="stat-value">{{ stats.memory }}MB</span>
            <span class="stat-label">Memory Usage</span>
          </div>
        </div>
      </div>

      <div class="card">
        <h2>💡 Interactive Demo</h2>
        <button @click="increment" class="btn">
          Count: {{ count }}
        </button>
        <p v-if="count > 0" class="hint">
          Clicked {{ count }} time{{ count > 1 ? 's' : '' }}!
        </p>
      </div>
    </main>

    <footer class="footer">
      <p>Built with Vue 3 + Rust + WebGPU</p>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'

const title = ref('Iris Runtime Demo')

const features = reactive([
  'Vue 3 SFC Support',
  'TypeScript Compilation',
  'Hot Module Replacement',
  'GPU-Accelerated Rendering',
  'CSS Modules & Scoped CSS',
  'Zero Config'
])

const stats = reactive({
  firstFrame: 8,
  fps: 60,
  memory: 75
})

const count = ref(0)

const increment = () => {
  count.value++
}
</script>

<style scoped>
#app {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
  color: #2c3e50;
}

.header {
  text-align: center;
  padding: 40px 0;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border-radius: 12px;
  margin-bottom: 30px;
}

.header h1 {
  font-size: 3em;
  margin: 0;
  font-weight: 700;
}

.subtitle {
  font-size: 1.2em;
  margin: 10px 0 0;
  opacity: 0.9;
}

.content {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 20px;
  margin-bottom: 30px;
}

.card {
  background: white;
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  transition: transform 0.2s, box-shadow 0.2s;
}

.card:hover {
  transform: translateY(-4px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
}

.card h2 {
  margin-top: 0;
  color: #667eea;
  font-size: 1.5em;
}

.card ul {
  list-style: none;
  padding: 0;
}

.card li {
  padding: 8px 0;
  border-bottom: 1px solid #eee;
}

.card li:last-child {
  border-bottom: none;
}

.card li::before {
  content: '✓ ';
  color: #4caf50;
  font-weight: bold;
  margin-right: 8px;
}

.stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
}

.stat {
  text-align: center;
  padding: 16px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 8px;
  color: white;
}

.stat-value {
  display: block;
  font-size: 2em;
  font-weight: 700;
  margin-bottom: 4px;
}

.stat-label {
  display: block;
  font-size: 0.9em;
  opacity: 0.9;
}

.btn {
  display: inline-block;
  padding: 12px 24px;
  font-size: 1.1em;
  font-weight: 600;
  color: white;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.btn:hover {
  transform: scale(1.05);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.btn:active {
  transform: scale(0.98);
}

.hint {
  margin-top: 12px;
  color: #667eea;
  font-weight: 600;
}

.footer {
  text-align: center;
  padding: 20px;
  color: #666;
  border-top: 1px solid #eee;
}
</style>
