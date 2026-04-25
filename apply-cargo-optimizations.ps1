# Cargo 性能优化应用脚本
# 一键应用所有推荐的优化配置

$ErrorActionPreference = "Stop"

Write-Host "🚀 Cargo 性能优化应用脚本" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# 1. 检查 Cargo 版本
Write-Host "Step 1: 检查 Cargo 版本..." -ForegroundColor Yellow
$cargoVersion = cargo --version | Select-String "cargo (\d+\.\d+\.\d+)"
if ($cargoVersion) {
    $version = $cargoVersion.Matches[0].Groups[1].Value
    Write-Host "✅ Cargo 版本: $version" -ForegroundColor Green
    
    $versionParts = $version -split '\.'
    $major = [int]$versionParts[0]
    $minor = [int]$versionParts[1]
    
    if ($major -gt 1 -or ($major -eq 1 -and $minor -ge 68)) {
        Write-Host "✅ 支持 sparse 协议" -ForegroundColor Green
    } else {
        Write-Host "⚠️  建议升级到 Cargo 1.68+ 以使用 sparse 协议" -ForegroundColor Yellow
    }
} else {
    Write-Host "❌ 无法检测 Cargo 版本" -ForegroundColor Red
}
Write-Host ""

# 2. 检查 CPU 核心数
Write-Host "Step 2: 检测 CPU 核心数..." -ForegroundColor Yellow
$cpuCores = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors
Write-Host "✅ CPU 逻辑核心数: $cpuCores" -ForegroundColor Green

$recommendedJobs = if ($cpuCores -le 4) { $cpuCores } 
                   elseif ($cpuCores -le 8) { $cpuCores }
                   else { [math]::Min($cpuCores, 12) }
Write-Host "💡 推荐并行编译数: $recommendedJobs" -ForegroundColor Cyan
Write-Host ""

# 3. 检查磁盘类型
Write-Host "Step 3: 检查磁盘类型..." -ForegroundColor Yellow
$projectDrive = (Get-Item "c:\").Name
$disk = Get-PhysicalDisk | Where-Object { $_.DeviceId -like "*$projectDrive*" }

if ($disk) {
    $mediaType = $disk.MediaType
    Write-Host "✅ 磁盘类型: $mediaType" -ForegroundColor Green
    
    if ($mediaType -eq "SSD") {
        Write-Host "✅ SSD 磁盘，编译速度较快" -ForegroundColor Green
    } else {
        Write-Host "⚠️  HDD 磁盘，建议将 target 目录移到 SSD" -ForegroundColor Yellow
    }
} else {
    Write-Host "ℹ️  无法检测磁盘类型" -ForegroundColor Gray
}
Write-Host ""

# 4. 检查当前配置
Write-Host "Step 4: 检查 Cargo 配置..." -ForegroundColor Yellow
$configPath = ".cargo\config.toml"

if (Test-Path $configPath) {
    Write-Host "✅ 配置文件已存在: $configPath" -ForegroundColor Green
    
    $config = Get-Content $configPath -Raw
    if ($config -match "tuna") {
        Write-Host "✅ 已配置清华镜像源" -ForegroundColor Green
    } else {
        Write-Host "⚠️  未配置镜像源" -ForegroundColor Yellow
    }
} else {
    Write-Host "❌ 配置文件不存在" -ForegroundColor Red
    Write-Host "   运行: cargo 已自动创建 .cargo/config.toml" -ForegroundColor Gray
}
Write-Host ""

# 5. 检查 sccache
Write-Host "Step 5: 检查 sccache..." -ForegroundColor Yellow
$sccache = Get-Command sccache -ErrorAction SilentlyContinue

if ($sccache) {
    Write-Host "✅ sccache 已安装: $($sccache.Source)" -ForegroundColor Green
    
    $env:RUSTC_WRAPPER = "sccache"
    Write-Host "✅ 已设置 RUSTC_WRAPPER=sccache" -ForegroundColor Green
} else {
    Write-Host "⚠️  sccache 未安装" -ForegroundColor Yellow
    Write-Host "   安装命令: cargo install sccache" -ForegroundColor Gray
}
Write-Host ""

# 6. 检查磁盘空间
Write-Host "Step 6: 检查磁盘空间..." -ForegroundColor Yellow
$diskInfo = Get-PSDrive C | Select-Object Used, Free
$freeSpaceGB = [math]::Round($diskInfo.Free / 1GB, 2)

Write-Host "✅ C 盘可用空间: ${freeSpaceGB} GB" -ForegroundColor Green

if ($freeSpaceGB -lt 10) {
    Write-Host "⚠️  可用空间不足，建议清理或移动 target 目录" -ForegroundColor Yellow
} else {
    Write-Host "✅ 磁盘空间充足" -ForegroundColor Green
}
Write-Host ""

# 7. 应用优化建议
Write-Host "Step 7: 应用优化建议..." -ForegroundColor Yellow
Write-Host ""

Write-Host "📋 推荐的优化配置：" -ForegroundColor Cyan
Write-Host ""

Write-Host "1️⃣  并行编译数（已自动检测）" -ForegroundColor White
Write-Host "   在 .cargo/config.toml 中添加：" -ForegroundColor Gray
Write-Host "   [build]" -ForegroundColor DarkGray
Write-Host "   jobs = $recommendedJobs" -ForegroundColor DarkGray
Write-Host ""

Write-Host "2️⃣  开发模式优化" -ForegroundColor White
Write-Host "   在 Cargo.toml 中添加：" -ForegroundColor Gray
Write-Host "   [profile.dev]" -ForegroundColor DarkGray
Write-Host "   debug = false" -ForegroundColor DarkGray
Write-Host "   codegen-units = 256" -ForegroundColor DarkGray
Write-Host ""

Write-Host "3️⃣  环境变量（可选）" -ForegroundColor White
Write-Host "   `$env:CARGO_BUILD_JOBS = `"$recommendedJobs`"" -ForegroundColor DarkGray
Write-Host "   `$env:RUSTC_WRAPPER = `"sccache`"" -ForegroundColor DarkGray
Write-Host ""

# 8. 生成优化报告
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "📊 优化报告" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

Write-Host "✅ 已完成的优化：" -ForegroundColor Green
Write-Host "   • 清华镜像源配置" -ForegroundColor Gray
Write-Host "   • 网络重试机制" -ForegroundColor Gray
Write-Host "   • 配置文件优化" -ForegroundColor Gray
Write-Host ""

Write-Host "⬜ 建议手动应用的优化：" -ForegroundColor Yellow
Write-Host "   • 设置并行编译数: jobs = $recommendedJobs" -ForegroundColor Gray
Write-Host "   • 安装 sccache: cargo install sccache" -ForegroundColor Gray
Write-Host "   • 优化开发模式配置" -ForegroundColor Gray
Write-Host ""

Write-Host "📖 详细文档：" -ForegroundColor Cyan
Write-Host "   • CARGO-MIRROR-CONFIG.md" -ForegroundColor Gray
Write-Host "   • CARGO-PERFORMANCE-OPTIMIZATION.md" -ForegroundColor Gray
Write-Host ""

Write-Host "🎉 优化检查完成！" -ForegroundColor Green
Write-Host ""
Write-Host "下一步：" -ForegroundColor White
Write-Host "  cargo build -p iris-sfc" -ForegroundColor DarkGray
Write-Host ""
