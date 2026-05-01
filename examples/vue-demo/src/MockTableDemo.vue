<template>
  <div class="demo-container">
    <header class="demo-header">
      <h1>Mock API Server Demo</h1>
      <p class="subtitle">此表格数据来源于内置 Mock API Server，无需真实后端即可展示完整功能</p>
    </header>

    <div class="table-controls">
      <div class="control-group">
        <label>每页显示：</label>
        <select v-model="pageSize" @change="loadUsers(1)">
          <option :value="10">10 条</option>
          <option :value="20">20 条</option>
          <option :value="50">50 条</option>
        </select>
      </div>
      <div class="control-group">
        <span class="total-info">共 {{ total }} 条记录</span>
      </div>
    </div>

    <div class="table-wrapper">
      <table class="demo-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>头像</th>
            <th>姓名</th>
            <th>邮箱</th>
            <th>电话</th>
            <th>角色</th>
            <th>部门</th>
            <th>状态</th>
            <th>创建时间</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="loading">
            <td colspan="9" class="loading-cell">
              <div class="loading-spinner"></div>
              <span>正在加载...</span>
            </td>
          </tr>
          <tr v-else-if="error" class="error-row">
            <td colspan="9" class="error-cell">
              <span class="error-icon">⚠</span>
              <span>加载失败：{{ error }}</span>
            </td>
          </tr>
          <tr v-else-if="users.length === 0">
            <td colspan="9" class="empty-cell">暂无数据</td>
          </tr>
          <tr v-for="(user, index) in users" :key="user.id" :class="{ 'row-even': index % 2 === 0 }">
            <td>{{ user.id }}</td>
            <td>
              <img :src="user.avatar" :alt="user.name" class="avatar" @error="handleImgError" />
            </td>
            <td>{{ user.name }}</td>
            <td>{{ user.email }}</td>
            <td>{{ user.phone }}</td>
            <td><span class="role-badge" :class="'role-' + (user.role || '').toLowerCase()">{{ user.role }}</span></td>
            <td>{{ user.department }}</td>
            <td>
              <span class="status-dot" :class="user.status === '启用' ? 'status-active' : 'status-inactive'"></span>
              {{ user.status }}
            </td>
            <td>{{ user.createdAt }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="pagination">
      <button :disabled="page <= 1" @click="loadUsers(page - 1)" class="page-btn">
        上一页
      </button>
      <div class="page-numbers">
        <template v-for="p in pageNumbers" :key="p">
          <button
            v-if="p !== '...'"
            :class="['page-btn', { active: p === page }]"
            @click="loadUsers(p)"
          >
            {{ p }}
          </button>
          <span v-else class="page-ellipsis">...</span>
        </template>
      </div>
      <button :disabled="page >= totalPages" @click="loadUsers(page + 1)" class="page-btn">
        下一页
      </button>
    </div>

    <div class="mock-info">
      <h3>Mock API 信息</h3>
      <p><strong>请求地址：</strong>GET /api/users?page={{ page }}&amp;pageSize={{ pageSize }}</p>
      <p><strong>响应状态：</strong><span class="mock-status">Mock Data (200)</span></p>
      <p><strong>数据总览：</strong>当前第 {{ page }}/{{ totalPages }} 页，每页 {{ pageSize }} 条，共 {{ total }} 条</p>
      <p class="mock-tip">💡 提示：Mock 数据由 Iris Runtime 内置 Mock Engine 自动生成，可在项目根目录的 iris.mock.json 中自定义数据 Schema</p>
    </div>
  </div>
</template>

<script>
export default {
  name: 'MockTableDemo',
  data() {
    return {
      users: [],
      page: 1,
      pageSize: 20,
      total: 0,
      totalPages: 0,
      loading: false,
      error: null,
    };
  },
  computed: {
    pageNumbers() {
      const pages = [];
      const total = this.totalPages;
      const current = this.page;
      if (total <= 7) {
        for (let i = 1; i <= total; i++) pages.push(i);
      } else {
        pages.push(1);
        if (current > 3) pages.push('...');
        const start = Math.max(2, current - 1);
        const end = Math.min(total - 1, current + 1);
        for (let i = start; i <= end; i++) pages.push(i);
        if (current < total - 2) pages.push('...');
        pages.push(total);
      }
      return pages;
    },
  },
  mounted() {
    this.loadUsers(1);
  },
  methods: {
    async loadUsers(page) {
      this.page = page;
      this.loading = true;
      this.error = null;
      try {
        const response = await fetch(`/api/users?page=${page}&pageSize=${this.pageSize}`);
        const result = await response.json();
        if (result.code === 0) {
          this.users = result.data.list;
          this.total = result.data.total;
          this.totalPages = result.data.totalPages;
        } else {
          this.error = result.message || '请求失败';
        }
      } catch (err) {
        this.error = err.message;
      } finally {
        this.loading = false;
      }
    },
    handleImgError(e) {
      e.target.src = 'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40"><rect width="40" height="40" fill="%23e0e0e0"/><text x="20" y="24" text-anchor="middle" fill="%23999" font-size="14">?</text></svg>';
    },
  },
};
</script>

<style>
.demo-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 30px 20px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  color: #333;
}

.demo-header {
  margin-bottom: 30px;
  text-align: center;
}

.demo-header h1 {
  font-size: 32px;
  margin: 0 0 10px 0;
  color: #1a1a2e;
}

.subtitle {
  color: #666;
  font-size: 15px;
  margin: 0;
}

.table-controls {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
  padding: 12px 16px;
  background: #f8f9fa;
  border-radius: 8px;
}

.control-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.control-group label {
  font-size: 14px;
  color: #555;
}

.control-group select {
  padding: 6px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
  background: white;
}

.total-info {
  font-size: 14px;
  color: #666;
}

.table-wrapper {
  overflow-x: auto;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
}

.demo-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 14px;
}

.demo-table th {
  background: #f5f5f5;
  padding: 12px 16px;
  text-align: left;
  font-weight: 600;
  color: #555;
  border-bottom: 2px solid #e0e0e0;
  white-space: nowrap;
}

.demo-table td {
  padding: 12px 16px;
  border-bottom: 1px solid #f0f0f0;
}

.demo-table tbody tr:hover {
  background: #f0f7ff;
}

.row-even {
  background: #fafafa;
}

.avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  object-fit: cover;
}

.role-badge {
  display: inline-block;
  padding: 2px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.role-admin { background: #fff3e0; color: #e65100; }
.role-编辑 { background: #e3f2fd; color: #1565c0; }
.role-管理员 { background: #fff3e0; color: #e65100; }
.role-用户 { background: #e8f5e9; color: #2e7d32; }
.role-访客 { background: #f3e5f5; color: #7b1fa2; }
.role-运营 { background: #fce4ec; color: #c62828; }
.role-测试 { background: #f1f8e9; color: #558b2f; }

.status-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 6px;
}

.status-active { background: #4caf50; }
.status-inactive { background: #f44336; }

.loading-cell {
  text-align: center;
  padding: 60px 20px;
  color: #999;
}

.loading-spinner {
  display: inline-block;
  width: 24px;
  height: 24px;
  border: 3px solid #e0e0e0;
  border-top-color: #667eea;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-right: 10px;
  vertical-align: middle;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error-row td {
  background: #fff5f5;
}

.error-cell {
  text-align: center;
  padding: 30px 20px;
  color: #c62828;
}

.error-icon {
  margin-right: 8px;
  font-size: 18px;
}

.empty-cell {
  text-align: center;
  padding: 60px 20px;
  color: #999;
}

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 8px;
  margin-top: 20px;
  padding: 15px 0;
}

.page-numbers {
  display: flex;
  align-items: center;
  gap: 4px;
}

.page-btn {
  padding: 8px 14px;
  border: 1px solid #ddd;
  border-radius: 4px;
  background: white;
  cursor: pointer;
  font-size: 14px;
  color: #555;
  transition: all 0.2s;
}

.page-btn:hover:not(:disabled) {
  background: #667eea;
  color: white;
  border-color: #667eea;
}

.page-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.page-btn.active {
  background: #667eea;
  color: white;
  border-color: #667eea;
}

.page-ellipsis {
  padding: 0 4px;
  color: #999;
}

.mock-info {
  margin-top: 30px;
  padding: 20px;
  background: #f0f8ff;
  border: 1px solid #b3d9f7;
  border-radius: 8px;
  font-size: 14px;
  line-height: 1.8;
}

.mock-info h3 {
  margin: 0 0 10px 0;
  color: #1a1a2e;
}

.mock-info p {
  margin: 4px 0;
  color: #555;
}

.mock-status {
  display: inline-block;
  padding: 1px 8px;
  background: #e8f5e9;
  color: #2e7d32;
  border-radius: 4px;
  font-weight: 500;
  font-size: 13px;
}

.mock-tip {
  margin-top: 10px !important;
  padding-top: 10px;
  border-top: 1px dashed #b3d9f7;
  color: #1565c0 !important;
}
</style>
