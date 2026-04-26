<template>
  <div class="app">
    <h1>{{ title }}</h1>
    <p>Welcome to Iris Engine - Vue 3 Runtime Demo</p>
    
    <div class="counter">
      <h2>Counter Example</h2>
      <p class="count">Count: {{ count }}</p>
      <button @click="increment" class="btn btn-primary">+</button>
      <button @click="decrement" class="btn btn-secondary">-</button>
      <button @click="reset" class="btn">Reset</button>
    </div>

    <div class="info">
      <h3>Runtime Information</h3>
      <ul>
        <li>Engine: Iris 0.1.0</li>
        <li>JS Runtime: Boa Engine</li>
        <li>Renderer: WebGPU</li>
        <li>Framework: Vue 3</li>
      </ul>
    </div>
  </div>
</template>

<script setup>
const title = "Iris Demo App"
const count = 0

function increment() {
  count++
}

function decrement() {
  count--
}

function reset() {
  count = 0
}
</script>

<style scoped>
.app {
  font-family: Arial, sans-serif;
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
}

h1 {
  color: #42b883;
  text-align: center;
}

.counter {
  background: #f5f5f5;
  padding: 20px;
  border-radius: 8px;
  margin: 20px 0;
  text-align: center;
}

.count {
  font-size: 24px;
  font-weight: bold;
  margin: 10px 0;
}

.btn {
  padding: 10px 20px;
  margin: 5px;
  border: 1px solid #ddd;
  border-radius: 4px;
  cursor: pointer;
  font-size: 16px;
}

.btn-primary {
  background: #42b883;
  color: white;
  border: none;
}

.btn-secondary {
  background: #35495e;
  color: white;
  border: none;
}

.info {
  background: #fff;
  border: 1px solid #ddd;
  padding: 15px;
  border-radius: 4px;
  margin-top: 20px;
}

.info h3 {
  margin-top: 0;
  color: #35495e;
}

.info ul {
  list-style: none;
  padding: 0;
}

.info li {
  padding: 5px 0;
  border-bottom: 1px solid #eee;
}
</style>
