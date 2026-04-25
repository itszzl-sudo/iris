# Auto-configure PowerShell for UTF-8 encoding
# Run this script once to permanently enable UTF-8 support

Write-Host "🔧 PowerShell UTF-8 Auto-Configuration" -ForegroundColor Cyan
Write-Host ""

# Check execution policy
$execPolicy = Get-ExecutionPolicy -Scope CurrentUser
Write-Host "Current execution policy: $execPolicy" -ForegroundColor Gray

if ($execPolicy -eq "Restricted") {
    Write-Host ""
    Write-Host "⚠️  Execution policy is Restricted. Updating to RemoteSigned..." -ForegroundColor Yellow
    try {
        Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
        Write-Host "✓ Execution policy updated" -ForegroundColor Green
    } catch {
        Write-Host "✗ Failed to update execution policy: $_" -ForegroundColor Red
        Write-Host "  Please run PowerShell as Administrator and try again" -ForegroundColor Yellow
        exit 1
    }
}

# Check if profile exists
Write-Host ""
Write-Host "Profile location: $PROFILE" -ForegroundColor Gray

if (Test-Path $PROFILE) {
    Write-Host "✓ Profile file exists" -ForegroundColor Green
    
    # Backup existing profile
    $backupPath = "$PROFILE.backup.$(Get-Date -Format 'yyyyMMddHHmmss')"
    Copy-Item $PROFILE $backupPath
    Write-Host "  Backed up to: $backupPath" -ForegroundColor Gray
} else {
    Write-Host "✗ Profile file doesn't exist" -ForegroundColor Yellow
    $profileDir = Split-Path $PROFILE -Parent
    if (!(Test-Path $profileDir)) {
        New-Item -ItemType Directory -Path $profileDir -Force | Out-Null
        Write-Host "  Created profile directory" -ForegroundColor Gray
    }
}

# Add UTF-8 configuration to profile
$utf8Config = @"

# ===== Auto-added by Iris UTF-8 Setup =====
# Enable UTF-8 encoding for proper character display
`$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001 | Out-Null
# ============================================

"@

# Check if UTF-8 config already exists
$profileContent = if (Test-Path $PROFILE) { Get-Content $PROFILE -Raw } else { "" }

if ($profileContent -match "Auto-added by Iris UTF-8 Setup") {
    Write-Host "✓ UTF-8 configuration already exists in profile" -ForegroundColor Green
} else {
    # Add UTF-8 config to profile
    Add-Content -Path $PROFILE -Value $utf8Config -Force
    Write-Host "✓ UTF-8 configuration added to profile" -ForegroundColor Green
}

# Reload profile
Write-Host ""
Write-Host "📝 Reloading profile..." -ForegroundColor Cyan
. $PROFILE

# Verify settings
Write-Host ""
Write-Host "✅ Configuration Complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Current encoding settings:" -ForegroundColor Gray
Write-Host "  - OutputEncoding: $([Console]::OutputEncoding.EncodingName)" -ForegroundColor Gray
Write-Host "  - Console CodePage: $([Console]::OutputEncoding.CodePage)" -ForegroundColor Gray
Write-Host ""
Write-Host "🎉 PowerShell is now configured for UTF-8!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Close and reopen PowerShell to apply changes" -ForegroundColor Gray
Write-Host "  2. Run tests: .\run-tests.ps1" -ForegroundColor Gray
Write-Host "  3. Run demo: cargo run -p iris-sfc --example sfc_demo" -ForegroundColor Gray
Write-Host ""
