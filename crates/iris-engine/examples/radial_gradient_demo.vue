<template>
  <div class="radial-gradient-showcase">
    <header class="header">
      <h1>🎨 径向渐变展示</h1>
      <p class="subtitle">Radial Gradient GPU Rendering Demo</p>
    </header>

    <main class="content">
      <!-- 基础径向渐变 -->
      <section class="gradient-section">
        <h2>基础径向渐变</h2>
        <div class="gradient-grid">
          <div class="gradient-card white-to-purple">
            <span>白 → 紫</span>
          </div>
          <div class="gradient-card cyan-to-magenta">
            <span>青 → 品红</span>
          </div>
          <div class="gradient-card gold-to-red">
            <span>金 → 红</span>
          </div>
        </div>
      </section>

      <!-- 多色标径向渐变 -->
      <section class="gradient-section">
        <h2>多色标径向渐变</h2>
        <div class="gradient-grid">
          <div class="gradient-card multi-color-1">
            <span>红 → 黄 → 蓝</span>
          </div>
          <div class="gradient-card multi-color-2">
            <span>绿 → 青 → 蓝</span>
          </div>
          <div class="gradient-card multi-color-3">
            <span>橙 → 粉 → 紫</span>
          </div>
        </div>
      </section>

      <!-- 应用场景 -->
      <section class="gradient-section">
        <h2>应用场景</h2>
        <div class="application-grid">
          <!-- 按钮效果 -->
          <div class="button-demo">
            <h3>按钮效果</h3>
            <div class="button-group">
              <button class="gradient-button primary">主要按钮</button>
              <button class="gradient-button secondary">次要按钮</button>
              <button class="gradient-button success">成功按钮</button>
            </div>
          </div>

          <!-- 卡片背景 -->
          <div class="card-demo">
            <h3>卡片背景</h3>
            <div class="info-card">
              <h4>信息卡片</h4>
              <p>使用径向渐变创建视觉吸引力的卡片背景</p>
            </div>
          </div>

          <!-- 头像占位符 -->
          <div class="avatar-demo">
            <h3>头像占位符</h3>
            <div class="avatar-group">
              <div class="avatar avatar-1">A</div>
              <div class="avatar avatar-2">B</div>
              <div class="avatar avatar-3">C</div>
            </div>
          </div>
        </div>
      </section>

      <!-- 大型背景效果 -->
      <section class="gradient-section">
        <h2>大型背景效果</h2>
        <div class="background-demo">
          <div class="hero-background">
            <div class="hero-content">
              <h3>Hero 背景</h3>
              <p>大型径向渐变创建沉浸式背景效果</p>
            </div>
          </div>
        </div>
      </section>
    </main>

    <footer class="footer">
      <p>✨ Iris Engine - 径向渐变 GPU 渲染</p>
    </footer>
  </div>
</template>

<script>
export default {
  name: 'RadialGradientShowcase',
  data() {
    return {
      title: 'Radial Gradient Demo'
    }
  },
  mounted() {
    console.log('Radial Gradient Showcase mounted!')
  }
}
</script>

<style>
.radial-gradient-showcase {
  min-height: 100vh;
  background: #1a1a2e;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  color: white;
  padding: 40px 20px;
  box-sizing: border-box;
}

.header {
  text-align: center;
  margin-bottom: 50px;
}

.header h1 {
  font-size: 48px;
  margin: 0 0 16px 0;
  font-weight: 800;
}

.subtitle {
  font-size: 18px;
  margin: 0;
  opacity: 0.8;
  color: #a0a0a0;
}

.content {
  max-width: 1200px;
  margin: 0 auto;
}

.gradient-section {
  margin-bottom: 50px;
}

.gradient-section h2 {
  font-size: 28px;
  margin: 0 0 24px 0;
  font-weight: 700;
  border-left: 4px solid #667eea;
  padding-left: 16px;
}

/* 基础渐变网格 */
.gradient-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 20px;
}

.gradient-card {
  height: 200px;
  border-radius: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  font-weight: 600;
  text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

/* 基础渐变样式 */
.white-to-purple {
  background: radial-gradient(circle, #ffffff, #6b5b95);
}

.cyan-to-magenta {
  background: radial-gradient(circle, #00bcd4, #e91e63);
}

.gold-to-red {
  background: radial-gradient(circle, #ffd700, #d32f2f);
}

/* 多色标渐变 */
.multi-color-1 {
  background: radial-gradient(circle, #ff0000, #ffff00, #0000ff);
}

.multi-color-2 {
  background: radial-gradient(circle, #4caf50, #00bcd4, #2196f3);
}

.multi-color-3 {
  background: radial-gradient(circle, #ff9800, #e91e63, #9c27b0);
}

/* 应用场景区 */
.application-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 24px;
}

.button-demo h3,
.card-demo h3,
.avatar-demo h3 {
  font-size: 20px;
  margin: 0 0 16px 0;
  font-weight: 600;
}

.button-group {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.gradient-button {
  padding: 12px 24px;
  border: none;
  border-radius: 24px;
  color: white;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
}

.gradient-button.primary {
  background: radial-gradient(circle, #667eea, #764ba2);
}

.gradient-button.secondary {
  background: radial-gradient(circle, #f093fb, #f5576c);
}

.gradient-button.success {
  background: radial-gradient(circle, #4facfe, #00f2fe);
}

/* 信息卡片 */
.info-card {
  background: radial-gradient(circle at 30% 30%, #667eea, #764ba2);
  padding: 24px;
  border-radius: 12px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.info-card h4 {
  font-size: 20px;
  margin: 0 0 12px 0;
}

.info-card p {
  margin: 0;
  opacity: 0.9;
  line-height: 1.5;
}

/* 头像 */
.avatar-group {
  display: flex;
  gap: 16px;
}

.avatar {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 32px;
  font-weight: 700;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  border: 3px solid rgba(255, 255, 255, 0.2);
}

.avatar-1 {
  background: radial-gradient(circle, #ff6b6b, #c44569);
}

.avatar-2 {
  background: radial-gradient(circle, #4ecdc4, #44a08d);
}

.avatar-3 {
  background: radial-gradient(circle, #ffe66d, #f7b731);
}

/* 大型背景效果 */
.background-demo {
  margin-top: 20px;
}

.hero-background {
  background: radial-gradient(circle at 50% 50%, #667eea 0%, #764ba2 50%, #1a1a2e 100%);
  padding: 80px 40px;
  border-radius: 20px;
  text-align: center;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.hero-content h3 {
  font-size: 36px;
  margin: 0 0 16px 0;
  font-weight: 800;
}

.hero-content p {
  font-size: 18px;
  opacity: 0.9;
  margin: 0;
}

/* 页脚 */
.footer {
  text-align: center;
  margin-top: 60px;
  padding-top: 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  color: #a0a0a0;
}

.footer p {
  margin: 0;
  font-size: 16px;
}
</style>
