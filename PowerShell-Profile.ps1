# PowerShell Profile Configuration
# This script runs every time PowerShell starts
# Location: $PROFILE

# Auto-enable UTF-8 encoding for proper Chinese character display
function Enable-UTF8Encoding {
    $OutputEncoding = [System.Text.Encoding]::UTF8
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    chcp 65001 | Out-Null
}

# Enable UTF-8 on startup
Enable-UTF8Encoding

# Add convenient aliases for Iris development
Set-Alias -Name test-iris -Value "cargo test -p iris-sfc -- --nocapture"
Set-Alias -Name run-demo -Value "cargo run -p iris-sfc --example sfc_demo"

# Custom prompt function
function prompt {
    $path = (Get-Location).Path
    $projectName = "Iris"
    
    # Show git branch if in a repository
    $gitBranch = git branch --show-current 2>$null
    if ($gitBranch) {
        Write-Host "$projectName [$gitBranch] " -NoNewline -ForegroundColor Green
    } else {
        Write-Host "$projectName " -NoNewline -ForegroundColor Green
    }
    
    Write-Host "❯ " -NoNewline -ForegroundColor Cyan
    return " "
}

# Display startup message
Write-Host ""
Write-Host "🚀 Iris Development Environment" -ForegroundColor Cyan
Write-Host "   UTF-8 encoding: Enabled" -ForegroundColor Gray
Write-Host "   Workspace: $((Get-Location).Path)" -ForegroundColor Gray
Write-Host ""
