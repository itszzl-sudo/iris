# PowerShell UTF-8 编码配置指南

## 问题描述
Windows PowerShell 默认使用 GBK 编码，导致 Rust 程序输出的中文显示为乱码。

## 解决方案

### 方案 1：每次手动启用（临时）

在 PowerShell 中运行：
```powershell
.\Enable-UTF8.ps1
```

然后运行测试：
```powershell
cargo test -p iris-sfc -- --nocapture
```

### 方案 2：使用便捷测试脚本（推荐）

直接运行测试脚本（自动处理编码）：
```powershell
# 运行所有测试
.\run-tests.ps1

# 运行特定测试
.\run-tests.ps1 template_compiler
.\run-tests.ps1 test_parse_vonce
```

### 方案 3：配置 PowerShell 永久启用（最佳）

#### 步骤 1：检查配置文件路径
```powershell
$PROFILE
```

#### 步骤 2：创建配置文件（如果不存在）
```powershell
if (!(Test-Path $PROFILE)) {
    New-Item -ItemType File -Path $PROFILE -Force
}
```

#### 步骤 3：编辑配置文件
```powershell
notepad $PROFILE
```

#### 步骤 4：添加以下内容
```powershell
# Auto-enable UTF-8 encoding
$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001 | Out-Null
```

#### 步骤 5：重新加载配置
```powershell
. $PROFILE
```

#### 步骤 6：验证设置
```powershell
cargo test -p iris-sfc template_compiler::tests
```

## 编码设置说明

| 设置项 | 作用 | 必需性 |
|--------|------|--------|
| `$OutputEncoding = UTF8` | 设置管道输出编码 | ✅ 必需 |
| `[Console]::OutputEncoding = UTF8` | 设置控制台直接输出编码 | ✅ 必需 |
| `chcp 65001` | 设置代码页为 UTF-8 | 建议 |

**注意**：仅执行 `chcp 65001` 不足以解决乱码，必须同时设置前两个编码！

## 快速命令参考

```powershell
# 启用 UTF-8
.\Enable-UTF8.ps1

# 运行测试
.\run-tests.ps1
.\run-tests.ps1 template_compiler

# 运行演示
cargo run -p iris-sfc --example sfc_demo

# 检查当前编码
[Console]::OutputEncoding
```

## 故障排除

### 问题：仍然显示乱码
**解决**：
1. 确认三个编码设置都已执行
2. 重新打开 PowerShell 窗口
3. 检查 `$OutputEncoding` 和 `[Console]::OutputEncoding` 是否都是 UTF-8

### 问题：配置不生效
**解决**：
1. 检查执行策略：`Get-ExecutionPolicy`
2. 如果是 `Restricted`，改为 `RemoteSigned`：
   ```powershell
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   ```

## 提供的脚本文件

| 文件 | 用途 |
|------|------|
| `Enable-UTF8.ps1` | 一次性启用 UTF-8 编码 |
| `run-tests.ps1` | 自动处理编码并运行测试 |
| `PowerShell-Profile.ps1` | 完整的 PowerShell 配置模板 |

## 永久配置模板

复制 `PowerShell-Profile.ps1` 的内容到您的 `$PROFILE` 文件中，即可获得：
- ✅ 自动 UTF-8 编码
- ✅ 自定义提示符
- ✅ 便捷别名
- ✅ 启动欢迎信息
