// SFC 编译器测试脚本
const { execSync } = require('child_process');
const fs = require('fs');

console.log('=== Iris SFC 编译器测试 ===\n');

// 创建测试 .vue 文件
const vueContent = `<template>
  <div class="app">
    <h1>Hello, Iris!</h1>
    <p>{{ message }}</p>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const message: string = "SFC compiler works!"
const count: number = 42
</script>

<style scoped>
.app {
  padding: 20px;
}

h1 {
  color: #6B4EE6;
}
</style>`;

fs.writeFileSync('test_component.vue', vueContent);
console.log('✅ 创建 test_component.vue\n');

// 注意：这里应该调用 iris-sfc 编译器
// 由于是 Rust 代码，我们通过 cargo test 来验证
console.log('运行 iris-sfc 测试...\n');

try {
  const output = execSync('cargo test -p iris-sfc -- --nocapture', {
    encoding: 'utf-8',
    stdio: ['pipe', 'pipe', 'pipe']
  });
  
  console.log(output);
} catch (error) {
  console.log('测试结果:', error.stdout);
}

console.log('\n=== 测试完成 ===');

// 清理
fs.unlinkSync('test_component.vue');
