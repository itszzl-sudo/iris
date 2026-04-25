# 🚀 Iris 快速开始指南

## 解决中文乱码问题

### 方法 1：一键自动配置（推荐 ⭐）

运行一次即可永久配置 PowerShell：
```powershell
.\Auto-Setup-UTF8.ps1
```

### 方法 2：临时启用

每次打开 PowerShell 后运行：
```powershell
.\Enable-UTF8.ps1
```

### 方法 3：使用测试脚本

测试脚本会自动处理编码：
```powershell
# 运行所有测试
.\run-tests.ps1

# 运行特定测试
.\run-tests.ps1 template_compiler
```

## 常用命令

```powershell
# 运行 SFC 编译器测试
cargo test -p iris-sfc template_compiler::tests

# 运行演示程序
cargo run -p iris-sfc --example sfc_demo

# 构建项目
cargo build -p iris-sfc

# 检查编码设置
[Console]::OutputEncoding
```

## 提供的脚本文件

| 文件 | 用途 | 使用频率 |
|------|------|----------|
| `Auto-Setup-UTF8.ps1` | 一键永久配置 PowerShell | 仅需运行一次 ⭐ |
| `Enable-UTF8.ps1` | 临时启用 UTF-8 编码 | 每次打开 PowerShell |
| `run-tests.ps1` | 运行测试（自动处理编码） | 日常开发 |
| `PowerShell-Profile.ps1` | 完整的配置模板 | 参考 |
| `POWERSHELL-UTF8-SETUP.md` | 详细配置文档 | 查阅 |

## 快速修复乱码

如果看到乱码（如 `鎷掔粷璁块棶`），立即运行：

```powershell
$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001 | Out-Null
```

## 验证配置

运行以下命令检查编码：
```powershell
Write-Host "OutputEncoding: $([Console]::OutputEncoding.EncodingName)"
Write-Host "CodePage: $([Console]::OutputEncoding.CodePage)"
```

应该显示：
- OutputEncoding: Unicode (UTF-8)
- CodePage: 65001
