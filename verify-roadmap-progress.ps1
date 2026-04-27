# Phase Progress Verification Script
# This script checks if Phase titles in ROADMAP match actual completion status

param(
    [string]$RoadmapPath = "ROADMAP_AND_PROGRESS.md"
)

Write-Host "Verifying Phase progress status..." -ForegroundColor Cyan
Write-Host ""

# Read ROADMAP file
$content = Get-Content $RoadmapPath -Raw -Encoding UTF8

# Extract all Phase titles
$phasePattern = '## .* Phase (\d+): .+?\(([\d+%) 完成\])(.*)'
$phaseMatches = [regex]::Matches($content, $phasePattern)

$phases = @()
foreach ($match in $phaseMatches) {
    $phaseNum = $match.Groups[1].Value
    $progress = $match.Groups[2].Value -replace '[()]', ''
    $status = $match.Groups[3].Value.Trim()
    $phases += @{
        Number = $phaseNum
        Progress = $progress
        Status = $status
    }
}

# Verify each Phase
$totalPhases = $phases.Count
$completedPhases = 0
$inProgressPhases = 0

foreach ($phase in $phases) {
    $phaseNum = $phase.Number
    $progress = $phase.Progress
    $status = $phase.Status
    
    # Check status markers using Unicode code points
    $checkmark = [char]0x2705
    $refresh = [char]0x1F504
    
    if ($status -match [regex]::Escape($checkmark) -or $progress -eq "100%") {
        $completedPhases++
        $statusIcon = $checkmark
        $statusColor = "Green"
    } elseif ($status -match [regex]::Escape($refresh)) {
        $inProgressPhases++
        $statusIcon = $refresh
        $statusColor = "Yellow"
    } else {
        $statusIcon = "[PENDING]"
        $statusColor = "Gray"
    }
    
    # Output Phase status
    Write-Host "Phase $phaseNum`: " -NoNewline
    Write-Host "$progress $statusIcon" -ForegroundColor $statusColor
    
    # Verify consistency
    if ($progress -eq "100%" -and $status -notmatch [regex]::Escape($checkmark)) {
        Write-Host "  WARNING: Progress is 100% but missing checkmark" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Overall Statistics:" -ForegroundColor Cyan
Write-Host "  Total Phases: $totalPhases"
Write-Host "  Completed: $completedPhases"
Write-Host "  In Progress: $inProgressPhases"
Write-Host "  Pending: $($totalPhases - $completedPhases - $inProgressPhases)"
Write-Host "==========================================" -ForegroundColor Cyan

# Calculate overall progress
$overallProgress = [math]::Round(($completedPhases / $totalPhases) * 100)
Write-Host ""
Write-Host "Overall Progress: $overallProgress%" -ForegroundColor $(if ($overallProgress -ge 100) { "Green" } elseif ($overallProgress -ge 50) { "Yellow" } else { "Red" })

# Check if overall progress needs update
if ($content -match "\*\*总体进度\*\*:\s*约\s*(\d+)%") {
    $currentOverall = [int]$Matches[1]
    if ($currentOverall -ne $overallProgress) {
        Write-Host ""
        Write-Host "WARNING: Overall progress needs update: $currentOverall% -> $overallProgress%" -ForegroundColor Yellow
        Write-Host "   Suggestion: Update the overall progress in the document" -ForegroundColor Yellow
    } else {
        Write-Host "Overall progress marker is correct ($overallProgress%)" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "Verification complete!" -ForegroundColor Green
