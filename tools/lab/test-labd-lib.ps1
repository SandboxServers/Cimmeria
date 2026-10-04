<#
.SYNOPSIS
    Tests for tools/lab/labd-lib.ps1 (no Pester needed). Exits non-zero on
    the first failure. Run under both shells the daemon uses:

        pwsh -NoProfile -File tools/lab/test-labd-lib.ps1
        powershell -NoProfile -File tools/lab/test-labd-lib.ps1
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'labd-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("labd-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
try {
    # --- labd.env keeps Unicode (regression: -Encoding ascii wrote '?') ---
    $jose = 'C:\Users\Jos' + [char]0x00E9 + '\Stargate Worlds'
    $mcp = @{ mcpServers = @{ 'cimmeria-lab' = @{ env = @{
        CIMMERIA_LAB_INSTALL_DIR = $jose; CIMMERIA_LAB_MCP_URL = 'http://h/mcp?a=b' } } } } |
        ConvertTo-Json -Depth 5
    $lines = Get-McpEnvLines $mcp
    Check ($lines.Count -eq 2) 'imports both env entries'
    $envPath = Join-Path $tmp 'labd.env'
    Write-LabdEnvFile $envPath (@('# comment') + $lines)
    $bytes = [System.IO.File]::ReadAllBytes($envPath)
    Check (-not ($bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB)) 'no BOM'
    $map = Read-LabdEnvFile $envPath
    Check ($map['CIMMERIA_LAB_INSTALL_DIR'] -eq $jose) 'non-ASCII path survives the round trip'
    Check ($map['CIMMERIA_LAB_MCP_URL'] -eq 'http://h/mcp?a=b') 'a value containing = is kept whole'
    Check ($map.Count -eq 2) 'comments are skipped'
    # A file saved with a BOM by an editor still reads.
    [System.IO.File]::WriteAllText($envPath, "K=$jose`r`n", (New-Object System.Text.UTF8Encoding $true))
    Check ((Read-LabdEnvFile $envPath)['K'] -eq $jose) 'a BOM-prefixed file reads'
    Check ((Get-McpEnvLines '{"mcpServers":{}}') -eq $null) 'no cimmeria-lab entry -> $null'

    # --- a stale labd.pid is not trusted by name alone ---
    $exe = Join-Path $tmp 'labd\cimmeria-lab.exe'
    $t0 = [datetime]::UtcNow.AddMinutes(-30)
    $info = [pscustomobject]@{ pid = 4242; bind = '127.0.0.1:9000'; started_at = $t0.AddSeconds(1).ToString('o') }
    $daemon = [pscustomobject]@{ Path = $exe; StartTime = $t0.ToLocalTime() }
    Check (Test-DaemonProcess $info $daemon $exe) 'the daemon that wrote labd.pid matches'
    $upper = [pscustomobject]@{ Path = $exe.ToUpperInvariant(); StartTime = $t0.ToLocalTime() }
    Check (Test-DaemonProcess $info $upper $exe) 'path compare ignores case'
    $stdio = [pscustomobject]@{ Path = (Join-Path $tmp 'target\debug\cimmeria-lab.exe'); StartTime = $t0.ToLocalTime() }
    Check (-not (Test-DaemonProcess $info $stdio $exe)) 'a cimmeria-lab from another path (reused pid) is refused'
    $later = [pscustomobject]@{ Path = $exe; StartTime = $t0.AddMinutes(20).ToLocalTime() }
    Check (-not (Test-DaemonProcess $info $later $exe)) 'our exe started after labd.pid was written is refused'
    $older = [pscustomobject]@{ Path = $exe; StartTime = $t0.AddHours(-3).ToLocalTime() }
    Check (-not (Test-DaemonProcess $info $older $exe)) 'our exe started long before labd.pid is refused'
    $noTime = [pscustomobject]@{ pid = 4242; bind = 'x' }
    Check (-not (Test-DaemonProcess $noTime $daemon $exe)) 'a pidfile without started_at is refused'
    Check (-not (Test-DaemonProcess $info $null $exe)) 'no process -> refused'

    # --- status probes the recorded bind ---
    Check ((Get-DaemonBind $info '127.0.0.1:8779') -eq '127.0.0.1:9000') 'recorded bind wins'
    Check ((Get-DaemonBind $null '127.0.0.1:8779') -eq '127.0.0.1:8779') 'default without a pidfile'
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) failed"; exit 1 }
Write-Host 'all passed'
