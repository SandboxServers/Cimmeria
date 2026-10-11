<#
.SYNOPSIS
    Tests for tools/lab/cli/local-setup-lib.ps1 (no Pester needed).

.DESCRIPTION
    Exits non-zero when any check fails. It works in a temp folder with a
    hashtable-backed environment store, so it never touches the user
    environment, %LOCALAPPDATA% or a real game install.

.EXAMPLE
    pwsh -NoProfile -File tools/lab/cli/test-local-setup.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'local-setup-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# A store over a plain hashtable, shaped like New-LabEnvStore's.
function New-FakeStore([hashtable]$Backing) {
    $b = $Backing
    return [pscustomobject]@{
        Get = { param($n) $b[$n] }.GetNewClosure()
        Set = { param($n, $v) $b[$n] = $v }.GetNewClosure()
    }
}
function Get-Row($rows, [string]$name) { return @($rows | Where-Object { $_.name -eq $name })[0] }

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lab-local-setup-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
$userBefore = [Environment]::GetEnvironmentVariable('CIMMERIA_LAB_MCP_TOKEN', 'User')
try {
    # --- secrets ---
    $s = New-LabSecret 32
    Check ($s.Length -eq 64 -and $s -match '^[0-9a-f]+$') 'a 32-byte secret is 64 lowercase hex chars'
    Check ((New-LabSecret 64).Length -eq 128) 'a 64-byte secret is 128 hex chars'
    Check ((New-LabSecret 32) -ne (New-LabSecret 32)) 'each secret is fresh'
    Check ((Get-LabSecretByteCount ('ab' * 32)) -eq 32) 'hex counts as decoded bytes'
    Check ((Get-LabSecretByteCount ('x' * 40)) -eq 40) 'non-hex counts as its bytes'
    Check ((Get-LabSecretByteCount $null) -eq 0) 'unset counts as zero bytes'

    # --- a fresh environment gets everything ---
    $env1 = @{}
    $rows = Set-LabLocalEnv (New-FakeStore $env1)
    Check ((Get-LabSecretByteCount $env1['CIMMERIA_LAB_MCP_TOKEN']) -ge 32) 'the lab endpoint token is at least 32 bytes'
    Check ($env1['CIMMERIA_LAB_MCP_TOKEN'] -match '^[0-9a-f]{64}$') 'the generated token is 64 hex chars (32 bytes from the CSPRNG)'
    Check ((Get-LabSecretByteCount $env1['CIMMERIA_TELEMETRY_HMAC_SECRET']) -eq 64) 'the telemetry secret is 64 bytes'
    Check ($env1['CIMMERIA_LAB_MCP_BIND'] -eq '127.0.0.1:8451') 'the bind is the local default'
    Check ($env1['CIMMERIA_LAB_MCP_URL'] -eq 'http://127.0.0.1:8451/mcp') 'the URL follows the bind'
    Check ((Get-Row $rows 'CIMMERIA_LAB_MCP_TOKEN').action -eq 'generated') 'the token row says generated'
    $printed = ($rows | ForEach-Object { "$($_.name) $($_.action) $($_.note)" }) -join "`n"
    Check (-not $printed.Contains($env1['CIMMERIA_LAB_MCP_TOKEN'])) 'no row carries the token'
    Check (-not $printed.Contains($env1['CIMMERIA_TELEMETRY_HMAC_SECRET'])) 'no row carries the telemetry secret'

    # --- a second run changes nothing ---
    $snapshot = $env1.Clone()
    $rows = Set-LabLocalEnv (New-FakeStore $env1)
    Check (@($rows | Where-Object { $_.action -ne 'kept' }).Count -eq 0) 'a second run keeps every value'
    $same = $true
    foreach ($k in $snapshot.Keys) { if ($env1[$k] -ne $snapshot[$k]) { $same = $false } }
    Check $same 'a second run leaves the values unchanged'

    # --- existing values are kept; a short secret is replaced ---
    $env2 = @{ CIMMERIA_LAB_MCP_TOKEN = 'short'; CIMMERIA_TELEMETRY_HMAC_SECRET = ('c' * 64); CIMMERIA_LAB_MCP_BIND = '127.0.0.1:9000' }
    $rows = Set-LabLocalEnv (New-FakeStore $env2)
    Check ($env2['CIMMERIA_LAB_MCP_TOKEN'] -ne 'short' -and $env2['CIMMERIA_LAB_MCP_TOKEN'].Length -eq 64) 'a too-short token is replaced'
    Check ((Get-Row $rows 'CIMMERIA_LAB_MCP_TOKEN').action -eq 'replaced') 'the replacement is reported'
    Check ($env2['CIMMERIA_TELEMETRY_HMAC_SECRET'] -eq ('c' * 64)) 'a 32-byte hex secret is kept'
    Check ($env2['CIMMERIA_LAB_MCP_BIND'] -eq '127.0.0.1:9000') 'an existing bind is kept'
    Check ($env2['CIMMERIA_LAB_MCP_URL'] -eq 'http://127.0.0.1:9000/mcp') 'the URL follows an existing bind'
    $env4 = @{ CIMMERIA_LAB_MCP_TOKEN = ('ab' * 16) }
    $null = Set-LabLocalEnv (New-FakeStore $env4)
    Check ($env4['CIMMERIA_LAB_MCP_TOKEN'] -eq ('ab' * 16)) 'a 32-char lab token the server accepts (string length) is kept'
    $env5 = @{ CIMMERIA_TELEMETRY_HMAC_SECRET = ('ab' * 16) }
    $null = Set-LabLocalEnv (New-FakeStore $env5)
    Check ($env5['CIMMERIA_TELEMETRY_HMAC_SECRET'] -ne ('ab' * 16)) 'a 16-byte hex telemetry secret the server refuses is replaced'
    $env3 = @{ CIMMERIA_LAB_MCP_BIND = '0.0.0.0:8451' }
    $null = Set-LabLocalEnv (New-FakeStore $env3)
    Check ($env3['CIMMERIA_LAB_MCP_URL'] -eq 'http://127.0.0.1:8451/mcp') 'a wildcard bind gives a loopback URL'

    Check ([Environment]::GetEnvironmentVariable('CIMMERIA_LAB_MCP_TOKEN', 'User') -eq $userBefore) 'the real user environment is untouched'

    # --- lab-account.json ---
    $install = Join-Path $tmp 'Stargate Worlds'
    Check ((Write-LabAccountFile $install) -eq 'written') 'a missing account file is written'
    $path = Join-Path $install 'Binaries\sessions\lab-account.json'
    $acct = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    Check ($acct.server -eq 'Local' -and $acct.username -eq 'lab' -and $acct.password -eq 'test' -and $acct.character -eq 'Labone') 'it holds the local row, the seeded account and Labone'
    Set-Content -LiteralPath $path -Value '{"server":"Cimmeria","username":"mine","password":"x"}'
    Check ((Write-LabAccountFile $install) -eq 'kept') 'an existing account file is kept'
    Check ((Get-Content -LiteralPath $path -Raw) -match '"mine"') 'and not overwritten'

    # --- LoginInternal.lua ---
    $launcher = "-- written by the launcher`r`n`r`nLoginMod = {}`r`n`r`nfunction LoginMod.loadServerSystems()`r`n    LoginMod.servers = {}`r`n    LoginMod.servers[`"Cimmeria`"] = `"http://play.cimmeria.app:8081`"`r`nend`r`n"
    $r = Add-LoginInternalRow $launcher 'Local' 'http://127.0.0.1:8081'
    Check ($r.status -eq 'added') 'a file without the row gets it'
    Check ($r.text.Contains("    LoginMod.servers[`"Cimmeria`"] = `"http://play.cimmeria.app:8081`"`r`n    LoginMod.servers[`"Local`"] = `"http://127.0.0.1:8081`"`r`nend")) 'the row goes after the last row, CRLF kept'
    $again = Add-LoginInternalRow $r.text 'Local' 'http://127.0.0.1:8081'
    Check ($again.status -eq 'present' -and $again.text -eq $r.text) 'adding twice is a no-op'
    $other = Add-LoginInternalRow ($launcher -replace 'Cimmeria"\] = "[^"]*"', 'Local"] = "http://10.0.0.9:8081"') 'Local' 'http://127.0.0.1:8081'
    Check ($other.status -eq 'conflict' -and $other.url -eq 'http://10.0.0.9:8081') 'a Local row with another URL is reported, not changed'
    $empty = Add-LoginInternalRow "function LoginMod.loadServerSystems()`n    LoginMod.servers = {}`nend`n" 'Local' 'http://127.0.0.1:8081'
    Check ($empty.status -eq 'added' -and $empty.text -match "servers = \{\}`n    LoginMod.servers\[`"Local`"\]") 'an empty table gets the row after it, LF kept'
    $commented = Add-LoginInternalRow "-- LoginMod.servers[`"Local`"] = `"x`"`nLoginMod.servers = {}`n" 'Local' 'http://127.0.0.1:8081'
    Check ($commented.status -eq 'added') 'a commented-out row does not count'
    Check ((Add-LoginInternalRow 'print("hi")' 'Local' 'u').status -eq 'unrecognised') 'a file without the table is left alone'

    # --- a bind that is not loopback is a WARN, not a silent keep ---
    Check (Test-LabLoopbackBind '127.0.0.1:8451') '127.0.0.1 is loopback'
    Check (Test-LabLoopbackBind '[::1]:8451') '::1 is loopback'
    Check (Test-LabLoopbackBind 'localhost:8451') 'localhost is loopback'
    Check (-not (Test-LabLoopbackBind '0.0.0.0:8451')) '0.0.0.0 is not loopback'
    $env6 = @{ CIMMERIA_LAB_MCP_BIND = '10.0.0.5:8444' }
    $rows = Set-LabLocalEnv (New-FakeStore $env6)
    Check ((Get-Row $rows 'CIMMERIA_LAB_MCP_BIND').action -eq 'WARN') 'a non-loopback bind is reported as WARN'
    Check ($env6['CIMMERIA_LAB_MCP_BIND'] -eq '10.0.0.5:8444') 'and left as it was'

    # --- the install dir variable ---
    $env7 = @{}
    Check ((Set-LabInstallDirEnv (New-FakeStore $env7) 'C:\A' $false).action -eq 'set' -and $env7['CIMMERIA_LAB_INSTALL_DIR'] -eq 'C:\A') 'an unset install dir is set'
    Check ((Set-LabInstallDirEnv (New-FakeStore $env7) 'c:\a\' $true).action -eq 'kept') 'the same path (case, trailing slash) is kept'
    $r = Set-LabInstallDirEnv (New-FakeStore $env7) 'D:\B' $false
    Check ($r.action -eq 'WARN' -and $env7['CIMMERIA_LAB_INSTALL_DIR'] -eq 'C:\A') 'a different install found implicitly warns and keeps the variable'
    $r = Set-LabInstallDirEnv (New-FakeStore $env7) 'D:\B' $true
    Check ($r.action -eq 'replaced' -and $env7['CIMMERIA_LAB_INSTALL_DIR'] -eq 'D:\B' -and $r.note -match 'C:\\A') 'an explicit -InstallDir replaces it and names the old path'

    # --- an existing account file that will not reach the local server ---
    $other = Join-Path $tmp 'Other Install'
    Check (@(Get-LabAccountWarnings $other).Count -eq 0) 'no account file, no warning'
    $null = Write-LabAccountFile $other
    Check (@(Get-LabAccountWarnings $other).Count -eq 0) 'the file local-setup writes needs no warning'
    $otherPath = Join-Path $other 'Binaries\sessions\lab-account.json'
    Set-Content -LiteralPath $otherPath -Value '{"server":"Cimmeria","username":"labaccount","password":"secretpw"}'
    $w = @(Get-LabAccountWarnings $other)
    Check ($w.Count -eq 2) 'another row and another account are both warned about'
    Check (-not (($w -join ' ').Contains('secretpw'))) 'the warning never shows the password'

    # --- the checkout rustup acts on ---
    $notRepo = Join-Path $tmp 'not-a-checkout'
    New-Item -ItemType Directory -Force $notRepo | Out-Null
    Check ($null -eq (Resolve-LabCheckout $notRepo $PSScriptRoot)) 'a -Worktree without rust-toolchain.toml is refused'
    $otherRepo = Join-Path $tmp 'other-repo'
    New-Item -ItemType Directory -Force $otherRepo | Out-Null
    git -C $otherRepo init -q 2>$null
    Push-Location $otherRepo
    try { $resolved = Resolve-LabCheckout '' $PSScriptRoot } finally { Pop-Location }
    $own = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
    Check ($resolved -eq $own) 'inside another git repo, the script''s own checkout is used'

    # --- the command itself, in a child process: Process scope, temp install and lab home ---
    $inst = Join-Path $tmp 'Glue Install'
    $loginDir = Join-Path $inst 'SGWGame\Content\UI\Startup\Login'
    New-Item -ItemType Directory -Force (Join-Path $inst 'Binaries'), $loginDir | Out-Null
    Set-Content -LiteralPath (Join-Path $inst 'Binaries\SGW.exe') -Value 'x'
    $lua = Join-Path $loginDir 'LoginInternal.lua'
    [System.IO.File]::WriteAllText($lua, $launcher)
    $cmd = Join-Path $PSScriptRoot 'local-setup.ps1'
    $runSetup = {
        param([string[]]$Extra, [hashtable]$Preset)
        $script = @'
param($Cmd, $Inst, $LabHome, $Wt, $PresetJson, [string[]]$Extra)
foreach ($n in 'CIMMERIA_LAB_MCP_TOKEN','CIMMERIA_TELEMETRY_HMAC_SECRET','CIMMERIA_LAB_MCP_BIND','CIMMERIA_LAB_MCP_URL','CIMMERIA_LAB_INSTALL_DIR') { Remove-Item "env:$n" -ErrorAction SilentlyContinue }
$preset = Get-Content -LiteralPath $PresetJson -Raw | ConvertFrom-Json
foreach ($p in $preset.PSObject.Properties) { Set-Item "env:$($p.Name)" $p.Value }
$env:CIMMERIA_LAB_HOME = $LabHome
$a = @{ InstallDir = $Inst; Worktree = $Wt; EnvScope = 'Process' }
foreach ($e in $Extra) { $a[$e] = $true }
& $Cmd @a
'@
        $presetJson = (($Preset, @{})[$null -eq $Preset]) | ConvertTo-Json -Compress
        $f = Join-Path $tmp 'run-setup.ps1'
        Set-Content -LiteralPath $f -Value $script
        $pf = Join-Path $tmp 'preset.json'
        Set-Content -LiteralPath $pf -Value $presetJson
        return (pwsh -NoProfile -File $f $cmd $inst (Join-Path $tmp 'home') $notRepo $pf $Extra 2>&1 | Out-String)
    }
    $out = & $runSetup @() $null
    Check ($out -match 'account: wrote') 'the command writes the account file'
    Check ($out -match 'not changed \(pass -PatchLoginInternal') 'without the switch LoginInternal.lua is not changed'
    Check (([System.IO.File]::ReadAllText($lua)) -eq $launcher) 'and its bytes are untouched'
    Check ($out -match 'no Cimmeria checkout found') 'a non-checkout -Worktree skips the target step'
    Check ($out -match 'account: ' -and $out -match 'Left to do') 'the later steps still run'
    Check (-not [regex]::IsMatch($out, '[0-9a-f]{32,}')) 'no secret-looking value is printed'
    $out = & $runSetup @('PatchLoginInternal') $null
    $patched = [System.IO.File]::ReadAllText($lua)
    Check ($out -match 'added the Local row; backup') '-PatchLoginInternal reports the write and the backup'
    Check ($patched.Contains('    LoginMod.servers["Local"] = "http://127.0.0.1:8081"' + "`r`n")) 'the row lands in the file, CRLF kept'
    $baks = @(Get-ChildItem -LiteralPath $loginDir -Filter 'LoginInternal.lua.*.bak')
    Check ($baks.Count -eq 1 -and ([System.IO.File]::ReadAllText($baks[0].FullName)) -eq $launcher) 'the backup holds the original'
    $out = & $runSetup @('PatchLoginInternal') $null
    Check ($out -match 'Local row present' -and @(Get-ChildItem -LiteralPath $loginDir -Filter 'LoginInternal.lua.*.bak').Count -eq 1) 'a rerun finds the row and makes no second backup'
    Set-Content -LiteralPath (Join-Path $inst 'Binaries\sessions\lab-account.json') -Value '{"server":"Cimmeria","username":"lab","password":"test"}'
    $out = & $runSetup @() @{ CIMMERIA_LAB_MCP_BIND = '0.0.0.0:8451' }
    Check ($out -match 'account: WARN kept') 'an account file for another row is a WARN'
    Check ($out -match 'CIMMERIA_LAB_MCP_BIND\s+WARN') 'a non-loopback bind is a WARN in the output'
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
