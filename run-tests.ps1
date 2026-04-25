# Test Runner Script with UTF-8 Encoding Support
# Usage: .\run-tests.ps1 [test-name]
# Example: .\run-tests.ps1 template_compiler

# Enable UTF-8 encoding
$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001 | Out-Null

# Run tests
if ($args.Count -gt 0) {
    $testName = $args -join " "
    Write-Host "Running tests: $testName" -ForegroundColor Cyan
    Write-Host ""
    cargo test -p iris-sfc $testName -- --nocapture
} else {
    Write-Host "Running all iris-sfc tests..." -ForegroundColor Cyan
    Write-Host ""
    cargo test -p iris-sfc -- --nocapture
}
