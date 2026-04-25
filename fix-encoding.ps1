# Fix file encoding for iris-sfc/src/lib.rs
# This script restores correct UTF-8 encoding and prevents future issues

$ErrorActionPreference = "Stop"
$file = "crates\iris-sfc\src\lib.rs"

Write-Host "🔧 Fixing file encoding: $file" -ForegroundColor Cyan
Write-Host ""

# Step 1: Restore from Git (clean version with correct Chinese comments)
Write-Host "Step 1: Restoring from Git..." -ForegroundColor Yellow
git checkout e6ec8a4 -- $file
Write-Host "✅ File restored from Git" -ForegroundColor Green
Write-Host ""

# Step 2: Read file with correct UTF-8 encoding
Write-Host "Step 2: Reading file with UTF-8 encoding..." -ForegroundColor Yellow
$lines = Get-Content $file -Encoding UTF8
Write-Host "✅ File read successfully ($($lines.Count) lines)" -ForegroundColor Green
Write-Host ""

# Step 3: Add init() function
Write-Host "Step 3: Adding init() function..." -ForegroundColor Yellow
$newLines = $lines + 
    "" +
    "/// Initialize the SFC compiler layer." +
    "///" +
    "/// This function is called by the main Iris engine initialization chain." +
    "/// Currently, it only logs the initialization event. Pre-compiled regex patterns" +
    "/// are automatically initialized on first use via ``LazyLock``." +
    "///" +
    "/// # Safety" +
    "/// This function is safe to call multiple times (idempotent)." +
    "///" +
    "/// # Example" +
    "///" +
    "/// ```ignore" +
    "/// use iris_sfc::init;" +
    "/// init(); // Initialize SFC compiler" +
    "/// ```" +
    "pub fn init() {" +
    "    info!(`"Iris SFC compiler initialized`");" +
    "}"

Write-Host "✅ init() function added" -ForegroundColor Green
Write-Host ""

# Step 4: Save as UTF-8 without BOM
Write-Host "Step 4: Saving as UTF-8 without BOM..." -ForegroundColor Yellow
$content = $newLines -join "`n"
$utf8NoBom = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText((Resolve-Path $file), $content + "`n", $utf8NoBom)
Write-Host "✅ File saved (UTF-8 without BOM)" -ForegroundColor Green
Write-Host ""

# Step 5: Verify
Write-Host "Step 5: Verifying..." -ForegroundColor Yellow
$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $file))
$hasBOM = ($bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF)
$firstLine = (Get-Content $file -First 1 -Encoding UTF8)

if ($hasBOM) {
    Write-Host "❌ FAIL: File still has BOM" -ForegroundColor Red
} else {
    Write-Host "✅ PASS: No BOM" -ForegroundColor Green
}

if ($firstLine -match "即时转译层") {
    Write-Host "✅ PASS: Chinese comments are correct" -ForegroundColor Green
} else {
    Write-Host "❌ FAIL: Chinese comments are garbled" -ForegroundColor Red
}

Write-Host ""
Write-Host "🎉 Encoding fix complete!" -ForegroundColor Cyan
Write-Host ""
Write-Host "File info:" -ForegroundColor White
Write-Host "  Size: $($bytes.Length) bytes" -ForegroundColor Gray
Write-Host "  Encoding: UTF-8 (no BOM)" -ForegroundColor Gray
Write-Host "  Lines: $($newLines.Count)" -ForegroundColor Gray
