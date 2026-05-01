# 会话记忆

> 自动导出于 2026-04-30。下次启动会话时加载此文件以恢复工作上下文。

---

## 一、当前会话任务：端口共用范围与管理面板重构

### 1.1 任务概述
对 iris-jetcrab-daemon 管理面板进行大规模重构，涉及：
1. **端口范围共用**: HTTP服务器、Mock API Server和控制面板三服务共用端口范围，遇端口被占自动换端口
2. **状态面板重构**: 显示三个服务的实际监听端口，移除操作按钮
3. **Vue渲染状态迁移**: 从状态面板移到项目列表
4. **AI本地模型文件UI重构**: 改名、合并按钮、隐藏模型仓库、显示下载完成度
5. **跨平台浏览器检测排序**: Chrome第一，第二优先级按平台区分

### 1.2 进度状态
| 任务 | 状态 | 说明 |
|------|------|------|
| Task 1: DaemonState 添加端口字段 | ✅ 完成 | actual_http_port + actual_mock_port |
| Task 2: 端口检测 + Mock Server | ✅ 完成 | find_available_port + mock路由 |
| Task 3: handle_status 返回实际端口 | ✅ 完成 | 返回 actual_* 字段 |
| Task 4a: 配置面板只保留端口范围 | ✅ 完成 | HTML+JS均已修改 |
| Task 4b: 状态面板移除按钮+显示端口 | ✅ 完成 | HTML+Rust替换变量 |
| Task 4c: 项目列表显示渲染状态 | ✅ 完成 | 每个项目显示渲染状态badge |
| Task 4d: AI本地模型文件重构 | ✅ 完成 | HTML+Rust替换变量+JS |
| Task 4e: JS函数调整 | ✅ 完成 | refreshStatus/Projects/toggleModelDownload |
| Task 5: 浏览器检测跨平台排序 | ✅ 完成 | Windows/macOS/Linux三平台 |
| **Task 6: 编译验证** | **✅ 已完成** | **编译通过，0 错误** |

### 1.3 编译错误（待修复）
**错误**: `handle_start_server` 的 `Handler` trait 不满足
- 位置: `crates/iris-jetcrab-daemon/src/main.rs` 中的 `.route("/api/server/start", post(handle_start_server))`
- 原因: 在 `handle_start_server` 中调用了 `find_available_port(...).await`，可能改变了函数的返回类型或生命周期，导致 axum 的 `Handler` trait 无法自动实现
- 建议修复方向: 检查函数签名是否完整，确保返回值类型正确

**未使用变量警告**（可同时修复）:
1. `mock_delay_ms` 在 `handle_mock_start` 中声明但未使用
2. `render_ok` 在 `handle_management_page` 中定义但未在 replace 中使用
3. `ai_model_dl_btn_display` 在 `handle_management_page` 中定义但未使用

---

## 二、修改的文件

**唯一修改的文件**: `crates/iris-jetcrab-daemon/src/main.rs`
- 包含所有 Rust 后端逻辑、路由、HTML 常量、JS 脚本
- 为单一文件架构（嵌入式模板），所有前端代码以内联方式存在

### 关键修改内容

#### Rust 部分
- `DaemonState` 新增: `actual_http_port`, `actual_mock_port`, `mock_server_running`
- 新增 `find_available_port()` 异步函数
- 新增 `handle_mock_start/stop/status` 三个处理函数
- `handle_start_server`: 使用 `find_available_port` 检测端口
- `handle_status`: 返回实际端口值
- `handle_management_page`: 更新替换变量（移除旧变量，添加新变量）
- `detect_installed_browsers`: 跨平台排序+新增Safari检测
- 注册新路由: `/api/mock/start`, `/api/mock/stop`, `/api/mock/status`

#### HTML 部分
- 配置面板: 只保留端口范围输入框
- 状态面板: 显示三个实际端口，移除按钮和Vue渲染状态
- 项目列表: 每个项目项显示渲染成功/未知状态badge
- AI本地模型文件: 移除模型仓库，合并为暂停/继续按钮，显示下载完成度

#### JS 部分
- `saveConfig`: 只保存端口范围和show_icon
- `refreshStatus`: 更新实际端口显示
- `refreshProjects`: 调用/api/status获取渲染状态
- `toggleModelDownload`: 合并开始/停止为单按钮
- `pollModelStatus`: 更新暂停/继续按钮状态
- `openWorkspaceBrowser`: 通过/api/status获取实际http_port

---

## 三、依赖与构建

### 项目信息
- 工作区: `c:\Users\a\Documents\lingma\leivueruntime`
- 包名: `iris-jetcrab-daemon`
- 构建命令: `cargo check -p iris-jetcrab-daemon`

### 端口配置
- 默认端口范围起始: 19999
- 默认端口范围大小: 500
- 配置文件: DaemonConfig (TOML序列化)
- 三个服务: HTTP Dev Server / Mock API Server / 控制面板

### 浏览器检测
- 平台: Windows（当前）
- 跨平台候选路径: Windows/macOS/Linux 各有独立候选列表
- 排序: Chrome第一 → 平台第二候选 → 其他

---

## 四、AI模型下载功能

### 核心机制
- 下载引擎: `iris_ai::downloader::download_model_with_progress`
- 断点续传: 通过 `iris_ai::downloader::model_partial_path` 检测
- 进度追踪: `DaemonState.model_download_progress` 存储进度百分比
- 停止控制: `DaemonState.model_download_stop` AtomicBool

### 状态字段
- `ai_model_downloaded`: 是否已下载完成
- `ai_model_repo`: HuggingFace模型仓库名（已隐藏）
- `ai_model_file`: GGUF模型文件名
- `model_download_progress`: 当前下载进度(0-100)

---

## 五、测试流程

### 守护进程功能测试
```powershell
powershell -ExecutionPolicy Bypass -File C:\Users\a\Documents\lingma\leivueruntime\test-daemon.ps1
```

### 构建验证（待执行）
```powershell
cargo check -p iris-jetcrab-daemon
```

---

## 六、下次启动待办

- [x] **修复 `handle_start_server` 的 Handler trait 编译错误** → 已完成（非Send的MutexGuard跨.await持有）
- [x] 修复3个未使用变量警告 → 已完成
- [x] 执行 `cargo check -p iris-jetcrab-daemon` 验证编译通过 → 已完成
4. 构建并启动 daemon 验证全功能（可选）
