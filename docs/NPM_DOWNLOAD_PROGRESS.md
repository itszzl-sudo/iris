# NPM 包下载进度显示

## 概述

Iris JetCrab CLI 现在支持**实时显示 npm 包下载进度**，通过 WebSocket 将进度信息推送到浏览器，用户可以在网页上直观地看到包的下载状态。

## 功能特性

### ✅ 核心功能

- **实时进度推送**: WebSocket 实时通信
- **可视化进度条**: 每个包独立显示
- **状态标识**: resolving → downloading → extracting → installed
- **自动重连**: WebSocket 断开后自动重连
- **多包并行显示**: 同时显示多个包的下载进度

### 📊 进度状态

| 状态 | 说明 | 进度范围 |
|------|------|---------|
| `resolving` | 解析包信息 | 0-10% |
| `downloading` | 下载 tarball | 10-80% |
| `extracting` | 解压文件 | 80-100% |
| `installed` | 安装完成 | 100% |
| `error` | 下载失败 | - |

## 技术实现

### 1. 后端：进度回调机制

**文件**: `npm_downloader.rs`

```rust
pub struct NpmDownloader {
    progress_callback: Option<Box<dyn Fn(&str, &str, u8, &str) + Send + Sync>>,
}

impl NpmDownloader {
    pub fn with_progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, &str, u8, &str) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    fn report_progress(&self, package: &str, version: &str, progress: u8, status: &str) {
        if let Some(callback) = &self.progress_callback {
            callback(package, version, progress, status);
        }
    }
}
```

### 2. 进度报告点

```rust
pub fn download_and_install(&self, package_name: &str, version: Option<&str>) -> Result<PathBuf> {
    // 解析包信息 (5%)
    self.report_progress(package_name, version, 5, "resolving");
    let version = self.resolve_version(package_name, version)?;
    
    // 开始下载 (20%)
    self.report_progress(package_name, &version.version, 20, "downloading");
    let tarball_data = self.download_tarball(&version.tarball_url)?;
    
    // 下载完成 (70%)
    self.report_progress(package_name, &version.version, 70, "downloading");
    
    // 开始解压 (80%)
    self.report_progress(package_name, &version.version, 80, "extracting");
    let package_path = self.extract_and_install(package_name, &tarball_data)?;
    
    // 安装完成 (100%)
    self.report_progress(package_name, &version.version, 100, "installed");
    
    Ok(package_path)
}
```

### 3. HMR 事件扩展

**文件**: `hmr.rs`

```rust
pub enum HmrEvent {
    // ... 其他事件
    
    /// npm 包下载进度
    #[serde(rename = "npm-download")]
    NpmDownload {
        package: String,      // 包名
        version: String,      // 版本号
        progress: u8,         // 进度 0-100
        status: String,       // 状态
        error: Option<String>, // 错误信息
    },
}
```

### 4. 编译器集成

**文件**: `compiler_cache.rs`

```rust
// 设置进度回调
if let Some(ws_manager) = &self.ws_manager {
    let ws_manager_clone = ws_manager.clone();
    compiler = compiler.with_progress_callback(move |package, version, progress, status| {
        let event = HmrEvent::NpmDownload {
            package: package.to_string(),
            version: version.to_string(),
            progress,
            status: status.to_string(),
            error: None,
        };
        ws_manager_clone.broadcast(event);
    });
}
```

### 5. 前端进度页面

**文件**: `progress.html`

完整的单页应用，包含：
- WebSocket 连接管理
- 进度数据管理
- 实时 UI 更新
- 自动重连机制

## 使用方式

### 1. 启动开发服务器

```bash
iris-jetcrab dev --root ./my-vue-project
```

### 2. 访问进度页面

在浏览器中打开：

```
http://localhost:3000/progress.html
```

### 3. 观察进度

页面会实时显示：
- 连接状态（右上角）
- 总体下载状态
- 每个包的详细进度

## 页面截图描述

### 空状态
```
┌──────────────────────────────────────┐
│  📦 NPM Package Download Progress    │
│  Real-time monitoring of downloads   │
│                                      │
│          📦                          │
│   No packages downloading            │
│   Waiting for compilation...         │
└──────────────────────────────────────┘
```

### 下载中
```
┌──────────────────────────────────────┐
│  🟢 Connected                        │
│                                      │
│  📥 Downloading 2 packages...        │
│                                      │
│  ┌─ vue v3.5.33 ──────────────────┐ │
│  │                    downloading  │ │
│  │ [████████░░░░░░░░░░] 65%       │ │
│  │ ⏳ In progress...               │ │
│  └────────────────────────────────┘ │
│                                      │
│  ┌─ pinia v2.1.7 ─────────────────┐ │
│  │                    extracting   │ │
│  │ [███████████████░░░] 85%       │ │
│  │ ⏳ In progress...               │ │
│  └────────────────────────────────┘ │
└──────────────────────────────────────┘
```

### 完成状态
```
┌──────────────────────────────────────┐
│  🟢 Connected                        │
│                                      │
│  ┌─ vue v3.5.33 ──────────────────┐ │
│  │                    installed    │ │
│  │ [████████████████████] 100%    │ │
│  │ ✅ Installed                    │ │
│  └────────────────────────────────┘ │
│                                      │
│  ┌─ pinia v2.1.7 ─────────────────┐ │
│  │                    installed    │ │
│  │ [████████████████████] 100%    │ │
│  │ ✅ Installed                    │ │
│  └────────────────────────────────┘ │
└──────────────────────────────────────┘
```

## WebSocket 消息格式

### 发送的消息

```json
{
  "type": "npm-download",
  "package": "vue",
  "version": "3.5.33",
  "progress": 65,
  "status": "downloading"
}
```

### 消息流示例

```
Client                            Server
  │                                 │
  │──── WebSocket Connect ─────────▶│
  │◀──── connected ─────────────────│
  │                                 │
  │◀──── npm-download (5%) ─────────│ resolving
  │◀──── npm-download (20%) ────────│ downloading
  │◀──── npm-download (70%) ────────│ downloading
  │◀──── npm-download (80%) ────────│ extracting
  │◀──── npm-download (100%) ───────│ installed
```

## 架构设计

```
┌─────────────────────────────────────────────┐
│              Browser (progress.html)         │
│  ┌─────────────────────────────────────┐    │
│  │  WebSocket Client                    │    │
│  │  - Connect to /@hmr                  │    │
│  │  - Receive events                    │    │
│  │  - Update UI                         │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
                     ↕ WebSocket
┌─────────────────────────────────────────────┐
│           Server (iris-jetcrab-cli)          │
│  ┌─────────────────────────────────────┐    │
│  │  WebSocketManager                    │    │
│  │  - Broadcast events                  │    │
│  └─────────────────────────────────────┘    │
│                     ↕                        │
│  ┌─────────────────────────────────────┐    │
│  │  CompilerCache                       │    │
│  │  - Progress callback                 │    │
│  └─────────────────────────────────────┘    │
│                     ↕                        │
│  ┌─────────────────────────────────────┐    │
│  │  VueProjectCompiler                  │    │
│  │  - with_progress_callback()          │    │
│  └─────────────────────────────────────┘    │
│                     ↕                        │
│  ┌─────────────────────────────────────┐    │
│  │  NpmDownloader                       │    │
│  │  - report_progress()                 │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
```

## 修改的文件

### 后端文件

1. **npm_downloader.rs** (+27 行)
   - 添加 `progress_callback` 字段
   - 添加 `with_progress_callback()` 方法
   - 添加 `report_progress()` 方法
   - 在下载各阶段报告进度

2. **vue_compiler.rs** (+12 行)
   - 添加 `ProgressCallback` 类型
   - 添加 `with_progress_callback()` 方法
   - 将回调传递给 NpmDownloader

3. **hmr.rs** (+15 行)
   - 添加 `NpmDownload` 事件变体

4. **compiler_cache.rs** (+25 行)
   - 添加 `ws_manager` 字段
   - 添加 `with_ws_manager()` 方法
   - 在编译时设置进度回调

5. **http_server.rs** (+2 行)
   - 传递 WebSocket 管理器给 CompilerCache

### 前端文件

1. **progress.html** (新增 371 行)
   - 完整的进度显示页面
   - WebSocket 客户端
   - 实时 UI 更新

## 测试

### 手动测试

1. 删除 `node_modules` 目录
2. 启动开发服务器
3. 访问 `http://localhost:3000/progress.html`
4. 观察下载进度

### 预期行为

- ✅ WebSocket 连接成功
- ✅ 显示包的解析状态（resolving）
- ✅ 显示下载进度（downloading 10-70%）
- ✅ 显示解压进度（extracting 80-100%）
- ✅ 显示完成状态（installed 100%）
- ✅ 多个包同时显示

## 未来优化

- [ ] 下载速度显示（KB/s）
- [ ] 预计剩余时间
- [ ] 文件解压进度细化
- [ ] 下载失败重试
- [ ] 进度历史记录
- [ ] 深色模式支持
- [ ] 移动端优化

## 总结

通过** WebSocket 实时通信 + 进度回调机制**，Iris JetCrab CLI 实现了：

✅ 实时显示 npm 包下载进度  
✅ 可视化进度条和状态标识  
✅ 多包并行下载显示  
✅ 自动重连机制  
✅ 美观的用户界面  

**让依赖安装过程透明可控！** 📦✨
