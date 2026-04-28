# WASM 编译和打包脚本 (PowerShell)
# 
# 使用方法:
#   .\build-wasm.ps1          # 生产构建
#   .\build-wasm.ps1 dev      # 开发构建

param(
    [string]$Mode = "release"
)

Write-Host "🔧 Building iris-runtime WASM module..." -ForegroundColor Cyan
Write-Host "   Mode: $Mode"
Write-Host ""

# 编译 WASM
if ($Mode -eq "dev") {
    wasm-pack build --target nodejs --dev
} else {
    wasm-pack build --target nodejs --release
}

Write-Host ""
Write-Host "✅ WASM build complete!" -ForegroundColor Green
Write-Host ""

# 显示文件大小
$wasmFile = "pkg/iris_runtime_bg.wasm"
if (Test-Path $wasmFile) {
    $wasmSize = (Get-Item $wasmFile).Length
    $sizeStr = switch ($wasmSize) {
        { $_ -gt 1GB } { "{0:F2} GB" -f ($_ / 1GB) }
        { $_ -gt 1MB } { "{0:F2} MB" -f ($_ / 1MB) }
        { $_ -gt 1KB } { "{0:F2} KB" -f ($_ / 1KB) }
        default { "{0} B" -f $_ }
    }
    Write-Host "📦 WASM binary size: $sizeStr" -ForegroundColor Yellow
}

# 显示生成的文件
Write-Host ""
Write-Host "📁 Generated files:" -ForegroundColor Cyan
Get-ChildItem pkg/ | Where-Object { $_.Name -match "\.(wasm|js|d.ts)$" } | Format-Table Name, Length -AutoSize

Write-Host ""
Write-Host "📝 Next steps:" -ForegroundColor Green
Write-Host "   npm install      # Install dependencies"
Write-Host "   npm pack         # Create npm package"
Write-Host "   npm publish      # Publish to npm"
Write-Host ""
