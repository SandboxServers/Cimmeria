function Invoke-BatchScript {
    <#
    .SYNOPSIS
        Runs a .bat file via cmd /c with output capture, returning output lines and exit code.
    #>
    param(
        [Parameter(Mandatory)][string]$ScriptPath,
        [string]$WorkingDirectory
    )

    if (-not $WorkingDirectory) {
        $WorkingDirectory = Split-Path $ScriptPath
    }

    $scriptName = Split-Path -Leaf $ScriptPath
    Write-Status "  Running $scriptName..." "DarkGray"

    $output = cmd /c "cd /d `"$WorkingDirectory`" && `"$scriptName`" < nul" 2>&1
    $exitCode = $LASTEXITCODE

    return @{
        Output   = $output
        ExitCode = $exitCode
    }
}
