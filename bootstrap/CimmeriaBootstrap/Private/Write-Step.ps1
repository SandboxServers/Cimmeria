function Write-Step {
    <#
    .SYNOPSIS
        Prints a timestamped step header to the console.
    #>
    param([string]$Message)
    $elapsed = (Get-Date) - $script:ScriptStartTime
    $ts = "[{0:mm\:ss}]" -f $elapsed
    Write-Host "`n$ts === $Message ===" -ForegroundColor Cyan
}
