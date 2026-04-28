@echo off
REM ========================================
REM iris-jetcrab-engine WASM 构建脚本 (Windows)
REM ========================================
REM 用法: .\build-wasm-engine.ps1 [debug|release]
REM ========================================

param(
    [ValidateSet("debug", "release")]
    [string]$Mode = "release"
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  iris-jetcrab-engine WASM Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$PKG_DIR = "$PSScriptRoot\pkg-engine"

if ($Mode -eq "release") {
    Write-Host "[1/3] 编译 WASM (release 模式)..." -ForegroundColor Yellow
    wasm-pack build --target web --release --out-dir $PKG_DIR
} else {
    Write-Host "[1/3] 编译 WASM (debug 模式)..." -ForegroundColor Yellow
    wasm-pack build --target web --out-dir $PKG_DIR
}

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "❌ 编译失败！" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "[2/3] 构建完成" -ForegroundColor Green
Write-Host ""

# 显示生成文件
Write-Host "生成的文件:" -ForegroundColor Cyan
Get-ChildItem $PKG_DIR | ForEach-Object {
    $size = if ($_.Length -gt 1MB) {
        "$([math]::Round($_.Length / 1MB, 2)) MB"
    } elseif ($_.Length -gt 1KB) {
        "$([math]::Round($_.Length / 1KB, 2)) KB"
    } else {
        "$($_.Length) B"
    }
    Write-Host "  - $($_.Name) ($size)" -ForegroundColor White
}

Write-Host ""
Write-Host "[3/3] 输出目录: $PKG_DIR" -ForegroundColor Green
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  使用示例:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "import initEngine, { IrisEngine } from './pkg-engine/iris_jetcrab_engine.js';"
Write-Host ""
Write-Host "await initEngine();"
Write-Host "const engine = new IrisEngine();"
Write-Host ""
Write-Host "// 编译 Vue SFC"
Write-Host "const result = engine.compileSfc(source, 'App.vue');"
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
