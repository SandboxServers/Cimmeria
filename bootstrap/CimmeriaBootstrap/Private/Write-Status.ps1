function Write-Status {
    <#
    .SYNOPSIS
        Prints a timestamped status message with optional color.
    #>
    param(
        [string]$Message,
        [string]$Color = "Gray"
    )
    $elapsed = (Get-Date) - $script:ScriptStartTime
    $ts = "[{0:mm\:ss}]" -f $elapsed
    Write-Host "$ts  $Message" -ForegroundColor $Color
}
