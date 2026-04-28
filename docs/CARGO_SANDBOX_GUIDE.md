# Cargo 沙盒隔离使用指南

> **问题**: 多个项目同时使用 Cargo 时出现锁文件冲突、依赖版本冲突  
> **解决**: 为每个项目创建独立的 Cargo 缓存环境

---

## 🎯 为什么需要沙盒隔离？

### 常见问题

1. **锁文件冲突**
   ```
   Blocking waiting for file lock on package cache
   ```

2. **依赖版本冲突**
   - 项目 A 需要 `tokio 1.30`
   - 项目 B 需要 `tokio 1.35`
   - 全局缓存导致冲突

3. **构建产物混乱**
   - 多个项目共享 `target/` 目录
   - 增量编译缓存互相干扰

4. **网络请求竞争**
   - 多个项目同时下载依赖
   - 注册表索引锁冲突

---

## 📦 方案 1：项目级本地缓存（推荐）

### 原理

为每个项目创建独立的 Cargo 缓存目录：

```
project-a/
├── .cargo-local/        ← 项目 A 的独立缓存
│   ├── registry/
│   └── git/
└── target/              ← 项目 A 的构建产物

project-b/
├── .cargo-local/        ← 项目 B 的独立缓存
│   ├── registry/
│   └── git/
└── target/              ← 项目 B 的构建产物
```

### 快速使用

#### **Windows (PowerShell)**

```powershell
# 1. 进入项目目录
cd C:\Users\a\Documents\lingma\leivueruntime

# 2. 运行沙盒设置脚本
.\Set-CargoSandbox.ps1

# 3. 正常运行 Cargo 命令
cargo build
cargo test
```

#### **Linux/macOS (Bash)**

```bash
# 1. 进入项目目录
cd /path/to/your/project

# 2. 加载环境变量
source .cargo-sandbox.env

# 3. 正常运行 Cargo 命令
cargo build
cargo test
```

### 手动设置

如果不想使用脚本，可以手动设置环境变量：

**PowerShell:**
```powershell
$env:CARGO_HOME = "$PWD\.cargo-local"
$env:CARGO_TARGET_DIR = "$PWD\target"
$env:CARGO_REGISTRY_CACHE = "$PWD\.cargo-local\registry"
```

**Bash:**
```bash
export CARGO_HOME="$PWD/.cargo-local"
export CARGO_TARGET_DIR="$PWD/target"
export CARGO_REGISTRY_CACHE="$PWD/.cargo-local/registry"
```

**CMD:**
```cmd
set CARGO_HOME=%CD%\.cargo-local
set CARGO_TARGET_DIR=%CD%\target
set CARGO_REGISTRY_CACHE=%CD%\.cargo-local\registry
```

---

## 🔧 方案 2：Cargo 配置级别隔离

在项目根目录创建 `.cargo/config.toml`：

```toml
# .cargo/config.toml

# 使用本地缓存
[build]
target-dir = "target"

# 配置镜像源（加速下载）
[source.crates-io]
replace-with = 'ustc'

[source.ustc]
registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"

# 网络超时设置
[net]
retry = 3
timeout = 30
```

---

## 🐳 方案 3：Docker 容器隔离（完全隔离）

使用 Docker 完全隔离构建环境：

```dockerfile
# Dockerfile
FROM rust:1.78-slim

WORKDIR /app

# 复制项目文件
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# 构建项目
RUN cargo build --release

# 运行
CMD ["cargo", "run", "--release"]
```

**构建和运行：**

```bash
# 构建镜像
docker build -t iris-project .

# 运行容器
docker run --rm -v $PWD:/app iris-project
```

---

## 🎛️ 方案 4：多版本 Rust 工具链隔离

使用 `rustup` 管理多个 Rust 版本：

```bash
# 为项目指定特定工具链
rustup override set 1.78.0

# 查看当前项目的工具链
rustup show

# 移除覆盖
rustup override unset
```

---

## 📊 方案对比

| 方案 | 隔离级别 | 磁盘占用 | 配置复杂度 | 适用场景 |
|------|---------|---------|-----------|---------|
| **本地缓存** | 中等 | 中等（2-5GB/项目） | 简单 | ✅ 日常开发（推荐） |
| **Cargo 配置** | 低 | 低 | 简单 | 轻量级隔离 |
| **Docker** | 完全 | 高（>10GB） | 中等 | CI/CD、生产环境 |
| **工具链隔离** | 低 | 中等 | 简单 | 多版本测试 |

---

## 💡 最佳实践

### 1. 开发环境推荐配置

```
项目结构：
├── .cargo-local/          ← 本地缓存（加入 .gitignore）
├── target/                ← 构建产物（加入 .gitignore）
├── .cargo/
│   └── config.toml        ← 镜像源配置
├── Set-CargoSandbox.ps1   ← 沙盒设置脚本
└── .cargo-sandbox.env     ← 环境变量配置

.gitignore：
.cargo-local/
target/
*.pdb
```

### 2. 磁盘空间管理

```powershell
# 查看本地缓存大小
Get-ChildItem .cargo-local -Recurse | Measure-Object -Property Length -Sum

# 清理缓存
Remove-Item .cargo-local -Recurse -Force

# 重新初始化
.\Set-CargoSandbox.ps1
```

### 3. 多项目并行开发

```powershell
# 终端 1 - 项目 A
cd project-a
.\Set-CargoSandbox.ps1
cargo build --watch

# 终端 2 - 项目 B
cd project-b
.\Set-CargoSandbox.ps1
cargo build --watch

# 两个项目互不干扰！
```

---

## 🔍 故障排查

### 问题 1：仍然出现锁文件冲突

**原因**: 环境变量未正确设置

**解决**:
```powershell
# 验证环境变量
Write-Host "CARGO_HOME: $env:CARGO_HOME"
Write-Host "CARGO_TARGET_DIR: $env:CARGO_TARGET_DIR"

# 应该显示项目本地路径，而不是全局路径
```

### 问题 2：依赖下载失败

**原因**: 本地缓存目录权限问题

**解决**:
```powershell
# 删除并重建缓存目录
Remove-Item .cargo-local -Recurse -Force
.\Set-CargoSandbox.ps1
```

### 问题 3：构建产物占用空间过大

**解决**:
```powershell
# 清理构建产物
cargo clean

# 或手动删除
Remove-Item target -Recurse -Force
```

---

## 📝 自动化脚本

### 开发启动脚本

创建 `dev.ps1`：

```powershell
# dev.ps1 - 一键启动开发环境

Write-Host "🚀 启动 Iris 开发环境..." -ForegroundColor Cyan

# 设置沙盒
.\Set-CargoSandbox.ps1

# 构建项目
Write-Host "📦 构建项目..." -ForegroundColor Yellow
cargo build

# 运行测试
Write-Host "🧪 运行测试..." -ForegroundColor Yellow
cargo test

Write-Host "✅ 开发环境就绪！" -ForegroundColor Green
```

### 清理脚本

创建 `clean.ps1`：

```powershell
# clean.ps1 - 清理项目

Write-Host "🧹 清理项目..." -ForegroundColor Yellow

# 清理构建产物
if (Test-Path target) {
    Remove-Item target -Recurse -Force
    Write-Host "  ✓ 清理 target/" -ForegroundColor Green
}

# 清理本地缓存（可选）
if (Test-Path .cargo-local) {
    $answer = Read-Host "是否清理本地缓存？(y/N)"
    if ($answer -eq 'y') {
        Remove-Item .cargo-local -Recurse -Force
        Write-Host "  ✓ 清理 .cargo-local/" -ForegroundColor Green
    }
}

Write-Host "✅ 清理完成！" -ForegroundColor Green
```

---

## 🎓 进阶：CI/CD 中的沙盒配置

### GitHub Actions

```yaml
# .github/workflows/build.yml
name: Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Set CARGO_HOME
        run: |
          echo "CARGO_HOME=$GITHUB_WORKSPACE/.cargo-local" >> $GITHUB_ENV
          echo "CARGO_TARGET_DIR=$GITHUB_WORKSPACE/target" >> $GITHUB_ENV
      
      - name: Build
        run: cargo build --verbose
      
      - name: Test
        run: cargo test --verbose
```

---

## 📚 相关资源

- [Cargo 官方文档](https://doc.rust-lang.org/cargo/)
- [CARGO_HOME 环境变量](https://doc.rust-lang.org/cargo/guide/cargo-home.html)
- [Rustup 工具链管理](https://rustup.rs/)

---

**文档维护者**: Iris Team  
**最后更新**: 2026-04-28
