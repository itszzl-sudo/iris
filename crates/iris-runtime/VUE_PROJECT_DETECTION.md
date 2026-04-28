# Vue 项目目录检测功能

> 自动检测 Vue 项目根目录，并在需要时显示友好的目录选择界面

---

## 🎯 功能概述

当用户执行 `npx iris-runtime dev` 时：

1. **自动检测**当前目录是否为 Vue 项目
2. **如果是**：正常启动开发服务器
3. **如果不是**：在浏览器中显示目录选择页面

---

## 📋 检测逻辑

### Vue 项目特征

iris-runtime 通过以下方式判断是否为 Vue 项目：

```javascript
function isVueProjectRoot(dirPath) {
  // 1. 检查 package.json 是否存在
  // 2. 解析 package.json
  // 3. 检查 Vue 依赖：
  //    - dependencies.vue
  //    - dependencies.vue3
  //    - devDependencies.vue
  // 4. 识别构建工具：
  //    - vite / @vitejs/plugin-vue
  //    - webpack / vue-loader
}
```

### 检测结果

```javascript
// 成功
{
  isVueProject: true,
  reason: 'Vue dependency found',
  buildTool: 'vite'  // 或 'webpack' 或 'unknown'
}

// 失败
{
  isVueProject: false,
  reason: 'No package.json found'
  // 或 'No Vue dependency in package.json'
  // 或 'Failed to parse package.json'
}
```

---

## 🖥️ 用户体验

### 场景 1: 在 Vue 项目中启动

```bash
$ cd my-vue-app
$ npx iris-runtime dev

✓ Vue project detected (vite)

🚀 Starting Iris Runtime dev server...

  ➜ Local: http://localhost:3000
  ➜ Ready in 234ms
```

**浏览器访问**: `http://localhost:3000` → 正常显示 Vue 应用

---

### 场景 2: 在非 Vue 项目中启动

```bash
$ cd ~/Documents
$ npx iris-runtime dev

⚠️  Warning: Current directory is not a Vue project
   Reason: No package.json found

A directory selection page will be shown in the browser.

🚀 Starting Iris Runtime dev server...

  ➜ Local: http://localhost:3000
  ➜ Ready in 234ms
```

**浏览器访问**: `http://localhost:3000` → 显示目录选择页面

---

## 🎨 目录选择页面

### 页面设计

```
┌──────────────────────────────────────────┐
│                                          │
│  🎯 Select Vue Project                  │
│  Please select the root directory of     │
│  your Vue project                        │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │ Current Path:                      │ │
│  │ /home/user/Documents               │ │
│  └────────────────────────────────────┘ │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │ ⚠️ Not a Vue Project               │ │
│  │ The current directory does not     │ │
│  │ appear to be a Vue project root.   │ │
│  └────────────────────────────────────┘ │
│                                          │
│  Choose Vue Project Directory:           │
│  ┌──────────────────────┐               │
│  │ 📁 Browse Directory  │               │
│  └──────────────────────┘               │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │ 💡 Vue Project Root Should Contain:│ │
│  │ ✓ package.json with vue dependency │ │
│  │ ✓ src/ directory with .vue files   │ │
│  │ ✓ index.html or public/index.html  │ │
│  │ ✓ Configuration files              │ │
│  └────────────────────────────────────┘ │
│                                          │
└──────────────────────────────────────────┘
```

---

## 🔄 交互流程

### 1. 用户点击 "Browse Directory"

```
用户点击
    ↓
触发文件选择器
    ↓
用户选择目录（支持 webkitdirectory）
    ↓
JavaScript 获取目录路径
```

### 2. 验证目录

```javascript
// 前端发送验证请求
const response = await fetch('/api/validate-project', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ path: directoryPath })
});

const result = await response.json();
```

### 3. 显示结果

**成功**：
```
✅ Vue Project Detected!
Build tool: vite
Redirecting to your application...
```

**失败**：
```
❌ Not a Vue Project
No Vue dependency in package.json
Please select a different directory.
```

---

## 🔧 API 端点

### POST /api/validate-project

验证指定路径是否为 Vue 项目。

**请求**:

```json
{
  "path": "my-vue-app"
}
```

**响应（成功）**:

```json
{
  "isVueProject": true,
  "reason": "Vue dependency found",
  "buildTool": "vite"
}
```

**响应（失败）**:

```json
{
  "isVueProject": false,
  "reason": "No Vue dependency in package.json"
}
```

---

## 💡 实现细节

### 1. 文件选择器

使用 HTML5 的 `webkitdirectory` 属性：

```html
<input type="file" webkitdirectory directory multiple>
```

**优势**:
- ✅ 支持目录选择（而非文件）
- ✅ 跨浏览器兼容（Chrome, Firefox, Edge）
- ✅ 获取完整的目录结构

### 2. 路径解析

```javascript
const firstFile = files[0];
const path = firstFile.webkitRelativePath;  // "my-vue-app/src/App.vue"
const directoryPath = path.split('/')[0];   // "my-vue-app"
```

### 3. 自动重定向

验证成功后 1.5 秒自动重定向：

```javascript
setTimeout(() => {
  window.location.href = '/';
}, 1500);
```

---

## 🎨 CSS 设计

### 渐变背景

```css
background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
```

### 卡片阴影

```css
box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
```

### 按钮悬停效果

```css
.browse-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
}
```

---

## 📊 检测流程图

```
启动服务器
    ↓
检测当前目录
    ↓
isVueProjectRoot(root)?
    ├─ 是 → 显示 "✓ Vue project detected"
    │         ↓
    │      正常服务 Vue 文件
    │
    └─ 否 → 显示 "⚠️ Warning: Not a Vue project"
              ↓
           用户访问 /index.html
              ↓
           返回目录选择页面
              ↓
           用户选择目录
              ↓
           发送到 /api/validate-project
              ↓
           验证成功？
              ├─ 是 → 重定向到 /
              └─ 否 → 显示错误信息
```

---

## 🚀 使用示例

### 正确用法

```bash
# 进入 Vue 项目目录
cd my-vue-app

# 启动
npx iris-runtime dev

# 输出:
# ✓ Vue project detected (vite)
# 🚀 Starting Iris Runtime dev server...
```

### 错误用法（有自动纠正）

```bash
# 在错误的目录
cd ~/Documents

# 启动
npx iris-runtime dev

# 输出:
# ⚠️ Warning: Current directory is not a Vue project
# A directory selection page will be shown in the browser.

# 浏览器打开后，用户可以选择正确的目录
```

---

## 🎯 设计理念

### ✅ Do

1. **友好提示**
   - 清晰的错误原因
   - 具体的解决方案

2. **可视化选择**
   - 图形界面选择目录
   - 实时验证反馈

3. **自动恢复**
   - 选择正确后自动重定向
   - 无需重启服务器

### ❌ Don't

1. 不直接退出程序
2. 不显示技术性错误
3. 不要求用户手动修改路径
4. 不假设用户知道 Vue 项目结构

---

## 📝 代码位置

```
crates/iris-runtime/lib/dev-server.js
├── isVueProjectRoot()          # 检测函数
├── generateDirectorySelectorPage()  # 生成 HTML 页面
└── handleRequest()             # API 端点处理
    └── /api/validate-project   # 验证 API
```

---

## 🔮 未来改进

1. **最近项目历史**
   - 记住用户选择的目录
   - 快速切换项目

2. **项目扫描**
   - 自动扫描父目录
   - 查找最近的 Vue 项目

3. **拖拽支持**
   - 拖拽文件夹到浏览器
   - 自动验证并加载

4. **多项目支持**
   - Monorepo 检测
   - 子项目选择

---

**文档维护者**: Iris Development Team  
**最后更新**: 2026-04-28
