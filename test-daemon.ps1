Write-Host "╔══════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  Iris JetCrab Daemon 功能测试              ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════╝" -ForegroundColor Cyan

# 1. 进程检查
Write-Host "[1/8] 守护进程运行状态" -ForegroundColor Yellow
$proc = tasklist /fi "imagename eq iris-jetcrab-daemon.exe" 2>$null | Select-String "iris-jetcrab"
if ($proc) { Write-Host "  [PASS] PID 存在" } else { Write-Host "  [FAIL] 未运行"; exit 1 }

# 2. API status
Write-Host "[2/8] API 连通性" -ForegroundColor Yellow
try {
    $r = Invoke-WebRequest -Uri http://127.0.0.1:19999/api/status -UseBasicParsing -TimeoutSec 3
    $j = $r.Content | ConvertFrom-Json
    if ($j.status -eq "running") { Write-Host "  [PASS] status=running" } else { Write-Host "  [FAIL] $($j.status)" }
} catch { Write-Host "  [FAIL] $_" }

# 3. 无客户端 API
Write-Host "[3/8] 无客户端连接" -ForegroundColor Yellow
try {
    $r = Invoke-WebRequest http://127.0.0.1:19999/api/connected-clients -UseBasicParsing
    $j = $r.Content | ConvertFrom-Json
    if ($j.count -eq 0) { Write-Host "  [PASS] count=0" } else { Write-Host "  [FAIL] count=$($j.count)" }
} catch { Write-Host "  [FAIL] $_" }

# 4. 主页
Write-Host "[4/8] 管理页面" -ForegroundColor Yellow
try {
    $r = Invoke-WebRequest http://127.0.0.1:19999/ -UseBasicParsing
    if ($r.Content -match "Iris JetCrab 管理面板") { Write-Host "  [PASS] 主页内容正确" } else { Write-Host "  [FAIL] 内容异常" }
} catch { Write-Host "  [FAIL] $_" }

# 5. /open
Write-Host "[5/8] 确认页" -ForegroundColor Yellow
try {
    $r = Invoke-WebRequest http://127.0.0.1:19999/open -UseBasicParsing
    if ($r.Content -match "确认打开" -and $r.Content -match "loadClients") { Write-Host "  [PASS] /open 页面正确" } else { Write-Host "  [FAIL] 内容异常" }
} catch { Write-Host "  [FAIL] $_" }

# 6. WS 测试
Write-Host "[6/8] WebSocket 连接/断开" -ForegroundColor Yellow
try {
    $ws = New-Object System.Net.WebSockets.ClientWebSocket
    $uri = [System.Uri]"ws://127.0.0.1:19999/ws"
    $ws.ConnectAsync($uri, [System.Threading.CancellationToken]::None).GetAwaiter().GetResult()
    Start-Sleep -Milliseconds 200
    $r2 = Invoke-WebRequest http://127.0.0.1:19999/api/connected-clients -UseBasicParsing
    $j2 = $r2.Content | ConvertFrom-Json
    if ($j2.count -eq 1) { Write-Host "  [PASS] 连接后 count=1" } else { Write-Host "  [FAIL] count=$($j2.count)" }
    Write-Host "        IP=$($j2.clients[0].ip)"
    $ws.Dispose()
    Start-Sleep -Milliseconds 500
    $r3 = Invoke-WebRequest http://127.0.0.1:19999/api/connected-clients -UseBasicParsing
    $j3 = $r3.Content | ConvertFrom-Json
    if ($j3.count -eq 0) { Write-Host "  [PASS] 断开后 count=0" } else { Write-Host "  [FAIL] count=$($j3.count)" }
} catch { Write-Host "  [FAIL] $_" }

# 7. 代码检查
Write-Host "[7/8] 渲染代码完整性" -ForegroundColor Yellow
$g1 = Select-String "C:\Users\a\Documents\lingma\leivueruntime\crates\iris-jetcrab-daemon\src\renderer.rs" -Pattern "pub fn draw_breathe_glow" | Select-Object -First 1
if ($g1) { Write-Host "  [PASS] draw_breathe_glow: $($g1.LineNumber)行" } else { Write-Host "  [FAIL] 未找到 draw_breathe_glow" }
$g2 = Select-String "C:\Users\a\Documents\lingma\leivueruntime\crates\iris-jetcrab-daemon\src\floating_window.rs" -Pattern "breathe_glow_alpha" | Select-Object -First 1
if ($g2) { Write-Host "  [PASS] 光晕呼吸: $($g2.LineNumber)行" } else { Write-Host "  [FAIL] 未找到 breathe_glow_alpha" }
$g3 = Select-String "C:\Users\a\Documents\lingma\leivueruntime\crates\iris-jetcrab-daemon\src\floating_window.rs" -Pattern "idx\].*> 30" | Select-Object -First 1
if ($g3) { Write-Host "  [PASS] 像素悬停: $($g3.LineNumber)行" } else { Write-Host "  [FAIL] 未找到像素悬停检测" }
$g4 = Select-String "C:\Users\a\Documents\lingma\leivueruntime\crates\iris-jetcrab-daemon\src\floating_window.rs" -Pattern "has_clients" | Select-Object -First 1
if ($g4) { Write-Host "  [PASS] 双击检测: $($g4.LineNumber)行" } else { Write-Host "  [FAIL] 未找到 has_clients" }

# 8. 构建验证
Write-Host "[8/8] 编译验证" -ForegroundColor Yellow
$buildOutput = cargo check -p iris-jetcrab-daemon 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "  [PASS] 编译通过"
} else {
    Write-Host "  [FAIL] 编译失败"
    $buildOutput | Select-String "error" | ForEach-Object { Write-Host "     $_" }
}

Write-Host ""
Write-Host "==============================" -ForegroundColor Green
Write-Host "  测试完成" -ForegroundColor Green
Write-Host "==============================" -ForegroundColor Green
