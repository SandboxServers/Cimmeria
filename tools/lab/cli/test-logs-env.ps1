<#
.SYNOPSIS
    Tests for tools/lab/cli/logs.ps1 and env.ps1 (no Pester needed). Exits
    non-zero on the first failure:

        pwsh -NoProfile -File tools/lab/cli/test-logs-env.ps1

    The pure functions are dot-sourced. The command cases run the real
    scripts on a temp lab home (CIMMERIA_LAB_HOME), so the real labd.env and
    labd.log are never read or written. The environment is restored afterwards.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'logs.ps1')
. (Join-Path $PSScriptRoot 'env.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$hex = 'ab' * 32
$logLines = @(
    '2026-10-10T20:00:00.000001Z  INFO serve: instance=Some("p2") started',
    '2026-10-10T20:00:01.000001Z  WARN serve: instance=None idle',
    '2026-10-10T20:00:02.000001Z DEBUG serve: instance="P2" bearer abc123def',
    "2026-10-10T20:00:03.000001Z ERROR serve: key $hex",
    '2026-10-10T20:00:04.000001Z  TRACE serve: instance="default" ping',
    'continuation line with no level'
)

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lab-logs-env-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
$savedHome = $env:CIMMERIA_LAB_HOME
try {
    # --- Select-LabLogLine: instance ---
    $byInstance = @($logLines | Select-LabLogLine -Instance 'p2')
    Check ($byInstance.Count -eq 2) 'instance p2 keeps instance=Some("p2") and instance="P2" (ignoring case)'
    Check (@($logLines | Select-LabLogLine -Instance 'p').Count -eq 0) 'instance matches the whole name, not a prefix'

    # --- Select-LabLogLine: level ---
    $warnUp = @($logLines | Select-LabLogLine -Level 'warn')
    Check ($warnUp.Count -eq 2 -and $warnUp[0] -match 'WARN' -and $warnUp[1] -match 'ERROR') 'level warn keeps WARN and ERROR only'
    Check (@($logLines | Select-LabLogLine -Level 'info').Count -eq 3) 'level info keeps INFO, WARN and ERROR'
    Check (@($logLines | Select-LabLogLine -Instance 'p2' -Level 'info').Count -eq 1) 'instance and level filters combine'
    Check (@($logLines | Select-LabLogLine).Count -eq 6) 'no filter keeps every line, including the continuation line'

    # --- Select-LabLogLine: continuation lines follow their entry ---
    $panic = @(
        '2026-10-10T20:00:05.000001Z ERROR serve: instance=Some("p2") panicked',
        '   at error handling here',
        '   stack frame 2',
        '2026-10-10T20:00:06.000001Z  INFO serve: instance=Some("p3") fine',
        '   at error handling there'
    )
    $errs = @($panic | Select-LabLogLine -Level 'error')
    Check ($errs.Count -eq 3 -and $errs[2] -match 'frame 2') 'a kept ERROR entry keeps its continuation lines'
    Check (@($errs | Where-Object { $_ -match 'there' }).Count -eq 0) "a continuation line whose second word is a level name is not ranked on its own"
    $p2 = @($panic | Select-LabLogLine -Instance 'p2')
    Check ($p2.Count -eq 3) 'an -Instance match keeps the entry continuation lines'

    # --- Select-LabLogLine: redaction ---
    $all = @($logLines | Select-LabLogLine)
    Check ($all[2] -notmatch 'abc123def' -and $all[2] -match '<redacted>') 'a bearer token is masked'
    Check ($all[3] -notmatch $hex -and $all[3] -match '<redacted>') 'a 64-hex token is masked'

    # --- env: masking (display only) ---
    Check ((Get-MaskedEnvLine 'CIMMERIA_LAB_DAEMON_TOKEN=s3cr3t') -eq 'CIMMERIA_LAB_DAEMON_TOKEN=<redacted>') 'a TOKEN key is masked'
    Check ((Get-MaskedEnvLine 'MY_PASSWORD = x y') -eq 'MY_PASSWORD=<redacted>') 'a PASSWORD key is masked, spaces around = allowed'
    Check ((Get-MaskedEnvLine 'CIMMERIA_LAB_BIND=http://127.0.0.1:8779/mcp') -eq 'CIMMERIA_LAB_BIND=http://<host>:8779/mcp') 'a URL host is shown as <host>, the port stays'
    Check ((Get-MaskedEnvLine '# note TOKEN=abc') -eq '# note TOKEN=<redacted>') 'a comment with a TOKEN assignment is masked'
    Check ((Get-MaskedEnvLine '# keep this comment') -eq '# keep this comment') 'a plain comment is shown as is'
    Check ((Get-MaskedEnvLine 'CIMMERIA_LAB_INSTANCES=default,p2') -eq 'CIMMERIA_LAB_INSTANCES=default,p2') 'an ordinary value is shown as is'
    Check ((Get-MaskedEnvLine ('CIMMERIA_LAB_X=' + $hex)) -eq 'CIMMERIA_LAB_X=<redacted>') 'a 64-hex value is masked under any key'
    Check ((Get-MaskedEnvLine 'U=http://user:p/ss@host.example/x') -eq 'U=http://<host>/x') 'URL userinfo with a / in the password is masked with the host'
    Check ((Get-MaskedEnvLine 'U=postgres://u:pw@db:5432/d') -eq 'U=postgres://<host>:5432/d') 'URL userinfo is masked, the port stays'
    Check ((Get-MaskedEnvLine 'API_KEY=0123456789abcdef0123456789abcdef01234567') -eq 'API_KEY=<redacted>') 'a KEY-named value is masked'
    Check ((Get-MaskedEnvLine 'pasted-secret-on-its-own-line') -eq '<unparsed line>') 'a line that is not KEY=VALUE is not shown'

    # --- env: key rule ---
    Check (Test-LabdEnvKey 'CIMMERIA_LAB_X_1') 'an upper-case key with digits and _ is valid'
    Check (-not (Test-LabdEnvKey 'lower')) 'a lower-case key is rejected'
    Check (-not (Test-LabdEnvKey '1ABC')) 'a key starting with a digit is rejected'
    Check (-not (Test-LabdEnvKey 'A-B')) 'a key with a dash is rejected'
    Check (-not (Test-LabdEnvKey '')) 'an empty key is rejected'

    # --- env: line rewrite keeps comments and order ---
    $src = @('# head', 'FOO=1', 'CIMMERIA_LAB_BAR=a=b', '# tail')
    Check (((Set-LabdEnvLine $src 'FOO' '2') -join '|') -eq '# head|FOO=2|CIMMERIA_LAB_BAR=a=b|# tail') 'set replaces the key in place'
    Check (((Set-LabdEnvLine $src 'NEW_ONE' 'x') -join '|') -eq '# head|FOO=1|CIMMERIA_LAB_BAR=a=b|# tail|NEW_ONE=x') 'set appends an unknown key'
    Check (((Remove-LabdEnvLine $src 'FOO') -join '|') -eq '# head|CIMMERIA_LAB_BAR=a=b|# tail') 'unset removes the key line only'
    Check (((Set-LabdEnvLine @('A=1', 'A=2') 'A' '9') -join '|') -eq 'A=1|A=9') 'set changes the last duplicate, which is the one that counts'
    Check ((Remove-LabdEnvLine @('FOO=1') 'FOO').Count -eq 0) 'unset of the only line gives no lines'
    Check ((Remove-LabdEnvLine @('foo=1', 'BAR=2') 'FOO') -join '|' -eq 'BAR=2') 'unset matches a key stored in another case, as the daemon reads it'
    Check (((Set-LabdEnvLine @('foo=1') 'FOO' '2') -join '|') -eq 'FOO=2') 'set replaces a key stored in another case instead of adding a second'
    Check (($src -join '|') -eq '# head|FOO=1|CIMMERIA_LAB_BAR=a=b|# tail') 'the input lines are not changed'

    # --- logs.ps1 and env.ps1 on a temp lab home ---
    $env:CIMMERIA_LAB_HOME = $tmp
    $logsPs1 = Join-Path $PSScriptRoot 'logs.ps1'
    $envPs1 = Join-Path $PSScriptRoot 'env.ps1'
    $envFile = Join-Path $tmp 'labd.env'
    [System.IO.File]::WriteAllLines((Join-Path $tmp 'labd.log'), [string[]]$logLines)

    $tail = pwsh -NoProfile -File $logsPs1 -Instance p2 -Lines 1
    Check ($LASTEXITCODE -eq 0 -and "$tail" -match 'DEBUG' -and "$tail" -notmatch 'abc123def') 'logs -Instance p2 -Lines 1 prints the last p2 line, masked'

    [System.IO.File]::WriteAllLines($envFile, [string[]]@('# head', 'FOO=1', 'CIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real', '# tail'))
    $before = [System.IO.File]::ReadAllText($envFile)

    $listed = pwsh -NoProfile -File $envPs1
    Check ($LASTEXITCODE -eq 0 -and (@($listed) -join '|') -eq '# head|FOO=1|CIMMERIA_LAB_DAEMON_TOKEN=<redacted>|# tail') 'env lists every line with the token masked'
    $got = pwsh -NoProfile -File $envPs1 get CIMMERIA_LAB_DAEMON_TOKEN
    Check ($LASTEXITCODE -eq 0 -and "$got" -eq '<redacted>') 'env get masks a token key'
    $null = pwsh -NoProfile -File $envPs1 get bad_key 2>$null
    Check ($LASTEXITCODE -eq 2) 'env get with a bad key exits 2'
    $null = pwsh -NoProfile -File $envPs1 set bad_key x 2>$null
    Check ($LASTEXITCODE -eq 2 -and [System.IO.File]::ReadAllText($envFile) -eq $before) 'env set with a bad key exits 2 and changes nothing'
    $null = pwsh -NoProfile -File $envPs1 set FOO 2>$null
    Check ($LASTEXITCODE -eq 2) 'env set without a value exits 2'

    $null = pwsh -NoProfile -File $envPs1 set FOO 2
    Check ($LASTEXITCODE -eq 0) 'env set FOO 2 exits 0'
    $afterSet = [System.IO.File]::ReadAllLines($envFile)
    Check (($afterSet -join '|') -eq '# head|FOO=2|CIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real|# tail') 'env set keeps comments and order'
    $backups = @(Get-ChildItem -LiteralPath $tmp -Filter 'labd.env.bak-*')
    Check ($backups.Count -eq 1 -and $backups[0].Name -match '^labd\.env\.bak-\d{8}-\d{6}$') 'env set writes labd.env.bak-<stamp>'
    Check ([System.IO.File]::ReadAllText($backups[0].FullName) -eq $before) 'the backup holds the old file'

    $null = pwsh -NoProfile -File $envPs1 set NEW_KEY 'a b=c'
    $null = pwsh -NoProfile -File $envPs1 unset FOO
    Check ($LASTEXITCODE -eq 0) 'env unset FOO exits 0'
    $afterUnset = [System.IO.File]::ReadAllLines($envFile)
    Check (($afterUnset -join '|') -eq '# head|CIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real|# tail|NEW_KEY=a b=c') 'env set appends an unknown key, unset removes FOO'
    Check (@(Get-ChildItem -LiteralPath $tmp -Filter 'labd.env.bak-*').Count -eq 3) 'each change writes its own backup, even in the same second'

    $null = pwsh -NoProfile -File $envPs1 unset MISSING_KEY
    Check ($LASTEXITCODE -eq 0 -and @(Get-ChildItem -LiteralPath $tmp -Filter 'labd.env.bak-*').Count -eq 3) 'unset of an absent key exits 0 and writes nothing'

    $null = pwsh -NoProfile -File $envPs1 set PAD_KEY 'val  ' 2>$null
    Check ($LASTEXITCODE -eq 2) 'a VALUE with trailing spaces is refused (exit 2)'
    $null = pwsh -NoProfile -File $envPs1 set DASH_KEY -Value:-Xmx512m
    Check ($LASTEXITCODE -eq 0 -and ([System.IO.File]::ReadAllLines($envFile) -contains 'DASH_KEY=-Xmx512m')) 'a VALUE starting with - is set with -Value:'
    Check (-not (Test-Path -LiteralPath "$envFile.tmp")) 'no labd.env.tmp is left behind'

    $listedAll = (pwsh -NoProfile -File $envPs1) -join "`n"
    Check ($listedAll -notmatch 'test-token-not-real') 'the token never appears in env output'
} finally {
    $env:CIMMERIA_LAB_HOME = $savedHome
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) failed"; exit 1 }
Write-Host 'all passed'
