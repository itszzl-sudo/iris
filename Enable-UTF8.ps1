# PowerShell UTF-8 Encoding Setup Script
# Run this script before running tests to fix Chinese character encoding issues

# Set pipeline output encoding to UTF-8
$OutputEncoding = [System.Text.Encoding]::UTF8

# Set console output encoding to UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# Set code page to UTF-8 (65001)
chcp 65001 | Out-Null

# Verify settings
Write-Host "✓ PowerShell UTF-8 encoding configured successfully" -ForegroundColor Green
Write-Host "  - OutputEncoding: UTF-8" -ForegroundColor Gray
Write-Host "  - Console OutputEncoding: UTF-8" -ForegroundColor Gray
Write-Host "  - Code Page: 65001 (UTF-8)" -ForegroundColor Gray
Write-Host ""
