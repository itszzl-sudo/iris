# PowerShell UTF-8 配置
# 将此文件保存到: $PROFILE (通常在 C:\Users\你的用户名\Documents\PowerShell\Microsoft.PowerShell_profile.ps1)

# 设置 UTF-8 编码
$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# 切换代码页到 UTF-8
chcp 65001 | Out-Null

# 可选：设置别名方便使用
function Enable-UTF8 {
    $OutputEncoding = [System.Text.Encoding]::UTF8
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    chcp 65001 | Out-Null
    Write-Host "✅ UTF-8 encoding enabled" -ForegroundColor Green
}

# 可选：每次启动自动启用
Enable-UTF8
