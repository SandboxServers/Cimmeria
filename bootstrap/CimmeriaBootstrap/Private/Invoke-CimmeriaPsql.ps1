function Invoke-CimmeriaPsql {
    <#
    .SYNOPSIS
        Runs psql (or pg_dump) once against the local PostgreSQL and returns stdout lines.
    .DESCRIPTION
        Admin mode connects as postgres, work mode as w-testing. Always passes
        ON_ERROR_STOP=1 for psql. Throws "<what> failed: <last stderr line>" on a
        non-zero exit, so callers need no exit-code checks. No $PSCmdlet use.
    #>
    param(
        [Parameter(Mandatory)][string]$PsqlExe,
        [Parameter(Mandatory)][int]$Port,
        [switch]$UseDocker,
        [switch]$Admin,
        [string]$Database = 'postgres',
        [string]$Sql,
        [string]$File,
        [string[]]$ExtraArgs = @(),
        [string]$Exe,
        [string]$What = 'psql'
    )

    $hostArgs = if ($UseDocker) { @('-h', 'localhost') } else { @() }
    $user = if ($Admin) { 'postgres' } else { 'w-testing' }
    $target = if ($Exe) { $Exe } else { $PsqlExe }
    $cmdArgs = @($hostArgs) + @('-p', $Port, '-U', $user)
    if (-not $Exe) {
        $cmdArgs += @('-X', '-q', '-v', 'ON_ERROR_STOP=1', '-d', $Database)
        if ($Sql) { $cmdArgs += @('-tAc', $Sql) }
        elseif ($File) { $cmdArgs += @('-f', $File) }
    } else {
        $cmdArgs += @('-d', $Database)
    }
    $cmdArgs += $ExtraArgs

    $out = & $target @cmdArgs 2>&1
    $code = $LASTEXITCODE
    $stdout = @($out | Where-Object { $_ -isnot [System.Management.Automation.ErrorRecord] } | ForEach-Object { "$_" })
    if ($code -ne 0) {
        $errs = @($out | Where-Object { $_ -is [System.Management.Automation.ErrorRecord] } |
            ForEach-Object { "$_".Trim() } | Where-Object { $_ })
        $last = if ($errs.Count) { $errs[-1] } else { ($stdout | Select-Object -Last 1) }
        throw "$What failed: $last"
    }
    return $stdout
}
