# 端口占用和错误处理

> iris-runtime 提供友好的错误提示和解决方案

---

## 🎯 端口占用检测

### 自动检测

当启动开发服务器时，iris-runtime 会：

1. **检查端口是否被占用**
2. **如果占用，显示友好的错误信息**
3. **提供具体的解决方案**

---

## 📋 错误提示示例

### 场景 1: 端口被占用

```bash
$ npx iris-runtime dev

❌ Error: Port 3000 is already in use

Possible causes:
  • Another instance of iris-runtime is running
  • Another application is using port 3000

Solutions:

  Option 1: Kill the process using port 3000

    # Find process:
    netstat -ano | findstr :3000

    # Kill process (replace PID):
    taskkill /F /PID <PID>

  Option 2: Use a different port

    npx iris-runtime dev --port 3001

  Option 3: Auto-select available port

    npx iris-runtime dev --port 0
```

---

### 场景 2: 没有图形界面

```bash
$ npx iris-runtime dev

❌ Error: No display available

Iris Runtime requires a graphical environment to run.
Without a GUI, iris-runtime cannot provide value.

Possible causes:
  • Running in SSH session without X11 forwarding
  • Running in a headless server/container
  • DISPLAY environment variable not set

Solutions:

  Option 1: Enable X11 forwarding (SSH)

    ssh -X user@host

  Option 2: Set DISPLAY variable

    export DISPLAY=:0

  Option 3: Run on a machine with GUI
```

---

## 🔧 解决方案

### 方案 1: 手动指定端口

```bash
# 使用端口 3001
npx iris-runtime dev --port 3001

# 使用端口 8080
npx iris-runtime dev --port 8080
```

### 方案 2: 自动选择端口

```bash
# 使用 --port 0 自动选择可用端口
npx iris-runtime dev --port 0

# 输出:
# ⚠️  Port 3000 is in use, using port 3001 instead
# ➜ Local: http://localhost:3001
```

### 方案 3: 关闭占用进程

**Windows**:

```powershell
# 查找使用端口 3000 的进程
netstat -ano | findstr :3000

# 输出示例:
# TCP    0.0.0.0:3000           0.0.0.0:0              LISTENING       12345

# 关闭进程
taskkill /F /PID 12345
```

**macOS/Linux**:

```bash
# 查找使用端口 3000 的进程
lsof -i :3000

# 输出示例:
# COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
# node    12345 user   12u  IPv4  *.*    3000      LISTEN

# 关闭进程
kill -9 12345
```

---

## ⚙️ 智能特性

### 1. 竞态条件处理

即使在端口检测和服务器启动之间有进程占用了端口，iris-runtime 也会自动处理：

```bash
$ npx iris-runtime dev

⚠️  Port 3000 became unavailable, trying port 3001...
➜ Local: http://localhost:3001
➜ Ready in 234ms
```

### 2. 跨平台命令提示

根据不同操作系统提供相应的命令：

- **Windows**: `netstat` + `taskkill`
- **macOS**: `lsof` + `kill`
- **Linux**: `lsof` + `kill`

### 3. 图形界面检测

在无头环境（headless）中运行时：

- ✅ 检测是否有可用的图形界面
- ✅ 如果没有，立即退出并提示
- ✅ 提供具体的解决方案

---

## 🎨 用户体验设计原则

### ✅ Do

1. **明确错误原因**
   - ❌ "Port in use"
   - ✅ "Port 3000 is already in use"

2. **提供解决方案**
   - ❌ 只显示错误
   - ✅ 提供 3 个具体方案

3. **跨平台兼容**
   - ❌ 只提供 Linux 命令
   - ✅ 根据 OS 提供对应命令

4. **自动恢复**
   - ❌ 直接退出
   - ✅ 尝试自动选择端口

### ❌ Don't

1. 不显示技术性错误堆栈
2. 不提供模糊的错误信息
3. 不假设用户知道如何解决问题
4. 不在没有 GUI 的情况下继续运行

---

## 📊 错误处理流程

```
启动服务器
    ↓
检查端口可用性
    ↓
端口被占用？
    ├─ 是 → 指定了 --port 0？
    │         ├─ 是 → 自动查找可用端口
    │         └─ 否 → 显示错误 + 退出
    └─ 否 → 继续
    ↓
检查图形界面
    ↓
有 GUI？
    ├─ 否 → 显示错误 + 退出
    └─ 是 → 启动服务器
    ↓
服务器启动成功？
    ├─ 否 → 自动尝试下一个端口
    └─ 是 → 显示成功信息
```

---

## 🚀 最佳实践

### 开发环境

```bash
# 使用默认端口
npx iris-runtime dev

# 如果端口冲突，使用自动选择
npx iris-runtime dev --port 0
```

### CI/CD 环境

```bash
# 在无头环境中，先检查环境
if [ -z "$DISPLAY" ]; then
  echo "Error: No display available"
  exit 1
fi

npx iris-runtime dev --port 0
```

### 团队协作

```bash
# 在 package.json 中配置
{
  "scripts": {
    "dev": "iris-runtime dev --port 0",
    "dev:3000": "iris-runtime dev --port 3000",
    "dev:8080": "iris-runtime dev --port 8080"
  }
}
```

---

**文档维护者**: Iris Development Team  
**最后更新**: 2026-04-28
