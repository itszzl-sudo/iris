<template>
  <div class="container">
    <h1>Hello, Iris! (Updated)</h1>
    <p>This is a test Vue component</p>
    <button @click="count++">
      Count: {{ count }}
    </button>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const count = ref(0)
const message: string = "Hot reload works!"
</script>

<style scoped>
.container {
  padding: 20px;
  font-family: Arial, sans-serif;
}

h1 {
  color: #6B4EE6;
}

button {
  background: #6B4EE6;
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: 4px;
  cursor: pointer;
}
</style>
