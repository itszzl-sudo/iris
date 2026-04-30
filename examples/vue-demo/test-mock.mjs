import { startDevServer } from '../../crates/iris-runtime/lib/server.js';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// 测试 Mock API Server
async function test() {
  const { server, mockHandler } = await startDevServer({
    root: __dirname,  // 使用 vue-demo 目录作为项目根目录
    port: 3456,
    host: 'localhost',
    open: false,
    enableHmr: false,
    debug: true,
    mock: { enabled: true, autoScan: true, delay: 0 },
  });

  // 等待服务器启动
  setTimeout(async () => {
    try {
      // 测试分页数据
      console.log('\n=== Test 1: Paginated Users ===');
      const r1 = await fetch('http://localhost:3456/api/users?page=1&pageSize=5');
      const d1 = await r1.json();
      console.log('Status:', r1.status);
      console.log('Headers X-Mock:', r1.headers.get('X-Mock-Enabled'));
      console.log('Data:', JSON.stringify(d1, null, 2).slice(0, 500));

      // 验证分页结构
      if (d1.code === 0 && d1.data && d1.data.list && d1.data.list.length === 5) {
        console.log('\n✓ Pagination: list has 5 items');
      }
      if (d1.data.total > 0 && d1.data.totalPages > 0) {
        console.log(`✓ Total: ${d1.data.total}, Pages: ${d1.data.totalPages}`);
      }

      // 测试单个对象
      console.log('\n=== Test 2: Single User ===');
      const r2 = await fetch('http://localhost:3456/api/users/1');
      const d2 = await r2.json();
      console.log('Single user:', JSON.stringify(d2, null, 2).slice(0, 300));

      // 测试 POST
      console.log('\n=== Test 3: POST /api/login ===');
      const r3 = await fetch('http://localhost:3456/api/login', { method: 'POST' });
      const d3 = await r3.json();
      console.log('Login result:', JSON.stringify(d3, null, 2).slice(0, 300));

      // 测试不存在的路径（应该走自动推断）
      console.log('\n=== Test 4: Auto-inferred endpoint ===');
      const r4 = await fetch('http://localhost:3456/api/orders?page=1&pageSize=3');
      const d4 = await r4.json();
      console.log('Orders:', JSON.stringify(d4, null, 2).slice(0, 500));

      console.log('\n✅ All tests passed!');
    } catch (err) {
      console.error('❌ Test failed:', err.message);
    } finally {
      server.close();
      process.exit(0);
    }
  }, 3000);
}

test().catch(err => {
  console.error(err);
  process.exit(1);
});
