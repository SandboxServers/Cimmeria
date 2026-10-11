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

    # --- labd.env without a stdio entry (the daemon-only setup) ---
    $daemonOnly = '{"mcpServers":{"cimmeria-lab":{"type":"http","url":"http://127.0.0.1:8779/mcp"}}}'
    Check ($null -eq (Get-McpEnvMap $daemonOnly)) 'an http cimmeria-lab entry has no env to import'
    $seed = Get-LabdEnvSeed $null @{ CIMMERIA_LAB_INSTALL_DIR = $jose } @{} @{ CIMMERIA_LAB_MCP_URL = 'http://127.0.0.1:8451/mcp' }
    Check ($seed.lines -contains "CIMMERIA_LAB_INSTALL_DIR=$jose") 'no stdio entry: the -InstallDir parameter seeds the install dir'
    Check ($seed.lines -contains 'CIMMERIA_LAB_MCP_URL=http://127.0.0.1:8451/mcp') 'no stdio entry: the default server endpoint is written'
    Check ($seed.sources['CIMMERIA_LAB_MCP_URL'] -eq 'default') 'the source of a default is reported'
    Check ($seed.missing -contains 'CIMMERIA_LAB_TELEMETRY') 'unset keys are listed as missing, not written'
    # Precedence: parameter > .mcp.json > environment > default.
    $imported = Get-McpEnvMap $mcp
    $seed = Get-LabdEnvSeed $imported @{} @{ CIMMERIA_LAB_INSTALL_DIR = 'C:\from-env'; CIMMERIA_LAB_TELEMETRY = 'optional' } @{}
    Check ($seed.lines -contains "CIMMERIA_LAB_INSTALL_DIR=$jose") 'a stdio entry still wins over the environment'
    Check ($seed.lines -contains 'CIMMERIA_LAB_MCP_URL=http://h/mcp?a=b') 'the stdio entry''s other values are kept'
    Check ($seed.lines -contains 'CIMMERIA_LAB_TELEMETRY=optional') 'the environment fills what the stdio entry lacks'
    $seed = Get-LabdEnvSeed $imported @{ CIMMERIA_LAB_INSTALL_DIR = 'D:\param' } @{} @{}
    Check ($seed.lines -contains 'CIMMERIA_LAB_INSTALL_DIR=D:\param') 'a parameter wins over the stdio entry'
    Check (@($seed.lines | Where-Object { $_ -like 'CIMMERIA_LAB_INSTALL_DIR=*' }).Count -eq 1) 'each key is written once'
    # Template placeholders never reach labd.env.
    $tmpl = [ordered]@{ CIMMERIA_LAB_INSTALL_DIR = '<SGW_INSTALL_DIR>'; CIMMERIA_LAB_MCP_TOKEN = '${CIMMERIA_LAB_MCP_TOKEN}'; CIMMERIA_LAB_BRIDGE = '127.0.0.1:8770' }
    $seed = Get-LabdEnvSeed $tmpl @{} @{ CIMMERIA_LAB_INSTALL_DIR = 'C:\real' } @{}
    Check ($seed.lines -contains 'CIMMERIA_LAB_INSTALL_DIR=C:\real') 'a <placeholder> falls through to the environment'
    Check (-not ($seed.lines -match 'CIMMERIA_LAB_MCP_TOKEN')) 'a ${VAR} token is not written literally'
    Check ($seed.lines -contains 'CIMMERIA_LAB_BRIDGE=127.0.0.1:8770') 'other imported keys are kept'
    Check (Test-LabdPlaceholder 'REPLACE_WITH_LAB_MCP_TOKEN_32B_MIN') 'the old REPLACE_WITH_ token counts as a placeholder'
    # A legacy stdio entry with literal secrets: never written, named in secrets.
    $legacy = [ordered]@{ CIMMERIA_LAB_INSTALL_DIR = 'C:\g'; CIMMERIA_LAB_MCP_TOKEN = ('t' * 40); CIMMERIA_LAB_TOKEN = ('a' * 64); CIMMERIA_TELEMETRY_HMAC_SECRET = ('s' * 40) }
    $seed = Get-LabdEnvSeed $legacy @{} @{} @{}
    Check (-not ($seed.lines -match 'TOKEN|SECRET')) 'literal TOKEN/SECRET values from a stdio entry are not written'
    Check (($seed.secrets -contains 'CIMMERIA_LAB_MCP_TOKEN') -and ($seed.secrets -contains 'CIMMERIA_LAB_TOKEN') -and ($seed.secrets -contains 'CIMMERIA_TELEMETRY_HMAC_SECRET')) 'each left-out secret is named'
    Check (-not (($seed.secrets -join ' ').Contains('t' * 40))) 'secrets lists names, not values'
    Check ($seed.lines -contains 'CIMMERIA_LAB_INSTALL_DIR=C:\g') 'the rest of a legacy entry is still imported'
    # install -Force keeps keys added with `lab env set`.
    $existing = [ordered]@{ CIMMERIA_LAB_UAT_SPECS = 'C:\specs'; CIMMERIA_LAB_OPERATOR = 'op'; CIMMERIA_LAB_INSTALL_DIR = 'C:\old'; CIMMERIA_LAB_MCP_URL = 'http://h:1/mcp'; CIMMERIA_LAB_MCP_TOKEN = 'x' * 40 }
    $seed = Get-LabdEnvSeed $null @{ CIMMERIA_LAB_INSTALL_DIR = 'C:\new' } @{} @{ CIMMERIA_LAB_MCP_URL = 'http://127.0.0.1:8451/mcp' } $existing
    Check ($seed.lines -contains 'CIMMERIA_LAB_UAT_SPECS=C:\specs' -and $seed.lines -contains 'CIMMERIA_LAB_OPERATOR=op') '-Force keeps keys only labd.env held'
    Check ($seed.lines -contains 'CIMMERIA_LAB_INSTALL_DIR=C:\new' -and -not ($seed.lines -contains 'CIMMERIA_LAB_INSTALL_DIR=C:\old')) 'a parameter replaces the old labd.env value'
    Check ($seed.lines -contains 'CIMMERIA_LAB_MCP_URL=http://h:1/mcp') 'a customised labd.env value beats the default'
    Check ($seed.secrets -contains 'CIMMERIA_LAB_MCP_TOKEN' -and -not ($seed.lines -match 'MCP_TOKEN')) 'a token already in labd.env is dropped on rewrite'
    $imp = [ordered]@{ CIMMERIA_LAB_OPERATOR = 'fromjson' }
    $seed = Get-LabdEnvSeed $imp @{} @{} @{} $existing
    Check (@($seed.lines | Where-Object { $_ -like 'CIMMERIA_LAB_OPERATOR=*' }).Count -eq 1 -and $seed.lines -contains 'CIMMERIA_LAB_OPERATOR=fromjson') 'a key both sources hold is written once, the stdio value first'

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

    # --- a daemon stop closes its clients, refusing leased ones (F-LC1) ---
    $status = @'
{"daemon":{"pid":7},"instances":[
 {"instance":"default","client_pid":100,"lease":{"held":false}},
 {"instance":"p2","client_pid":200,"lease":{"held":true,"lease":{"owner":"agent-a","purpose":"uat"}}},
 {"instance":"p3","client_pid":null,"lease":{"held":true,"lease":{"owner":"agent-b"}}},
 {"instance":"p4","lease":{"held":false}}
]}
'@ | ConvertFrom-Json
    $sel = Select-LabClientsToClose $status $false
    Check (@($sel.close).Count -eq 1 -and $sel.close[0].instance -eq 'default' -and $sel.close[0].pid -eq 100) 'an unleased client is closed'
    Check (@($sel.refuse).Count -eq 1 -and $sel.refuse[0].instance -eq 'p2') 'a leased client is refused'
    Check ($sel.refuse[0].holder -eq 'agent-a (uat)') 'the refusal names owner and purpose'
    Check (-not (@($sel.close) | Where-Object { $_.instance -in 'p3', 'p4' })) 'a lease without a client, or no client_pid, closes nothing'
    $forced = Select-LabClientsToClose $status $true
    Check ((@($forced.close) | ForEach-Object { $_.instance }) -join ',' -eq 'default,p2') '-Force closes the leased client too'
    Check (@($forced.refuse).Count -eq 1) '-Force still reports the holder it overrides'
    $none = Select-LabClientsToClose ('{"daemon":{"pid":7},"instances":[]}' | ConvertFrom-Json) $false
    Check (@($none.close).Count -eq 0 -and @($none.refuse).Count -eq 0) 'no instances -> nothing to close'
    $odd = Select-LabClientsToClose ('{"daemon":{"pid":7},"instances":[{"instance":"p5","client_pid":"x","lease":{"held":true,"lease":{}}}]}' | ConvertFrom-Json) $false
    Check (@($odd.close).Count -eq 0 -and @($odd.refuse).Count -eq 0) 'a non-numeric client_pid is skipped'

    # --- the stop plan ---
    $empty = '{"daemon":{"pid":7},"instances":[]}' | ConvertFrom-Json
    Check (Test-LabdStatusShape $empty) 'a daemon hosting no instances still has the status shape'
    Check (-not (Test-LabdStatusShape ('{"instances":[]}' | ConvertFrom-Json))) 'no daemon object -> not the shape'
    Check (-not (Test-LabdStatusShape 'some html')) 'a string body -> not the shape'
    Check (-not (Test-LabdStatusShape $null)) '$null -> not the shape'
    $p = Get-LabdStopPlan $empty $false $true $false
    Check ($p.action -eq 'proceed' -and -not $p.warn -and $p.close.Count -eq 0) 'an empty instance list proceeds without the unreadable-status warning'
    $p = Get-LabdStopPlan $status $false $true $false
    Check ($p.action -eq 'refuse-leased' -and $p.refused.Count -eq 1 -and $p.refused[0].instance -eq 'p2') 'a leased client refuses the stop'
    $p = Get-LabdStopPlan $status $false $true $true
    Check ($p.action -eq 'proceed' -and $p.close.Count -eq 2 -and $p.overridden.Count -eq 1) '-Force proceeds and names the overridden lease'
    $free = '{"daemon":{"pid":7},"instances":[{"instance":"default","client_pid":100,"lease":{"held":false}}]}' | ConvertFrom-Json
    $p = Get-LabdStopPlan $free $false $true $false
    Check ($p.action -eq 'proceed' -and $p.close.Count -eq 1 -and $p.overridden.Count -eq 0) 'unleased clients proceed and are closed'
    $p = Get-LabdStopPlan $null $true $true $false
    Check ($p.action -eq 'refuse-unauthorized') 'a live daemon that refuses the token refuses the stop'
    $p = Get-LabdStopPlan $null $true $true $true
    Check ($p.action -eq 'proceed' -and $p.warn -and $p.close.Count -eq 0) '-Force past a refused token proceeds, warns, closes nothing'
    $p = Get-LabdStopPlan $null $false $true $false
    Check ($p.action -eq 'proceed' -and $p.warn) 'a live daemon without /status proceeds with a warning'
    $p = Get-LabdStopPlan $null $true $false $false
    Check ($p.action -eq 'proceed' -and -not $p.warn) 'no daemon running -> proceed quietly'

    # --- daemon.ps1's Stop-Daemon is wired to the plan (a guard: these fail
    # if the wiring is removed or the client close moves before the kill) ---
    $daemonPath = Join-Path $PSScriptRoot 'daemon.ps1'
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($daemonPath, [ref]$null, [ref]$null)
    $fn = $ast.Find({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Stop-Daemon' }, $true)
    Check ($null -ne $fn) 'daemon.ps1 defines Stop-Daemon'
    $body = $fn.Body.Extent.Text
    Check ($body -match 'Get-LabdStopPlan') 'Stop-Daemon asks Get-LabdStopPlan'
    Check ($body -match "'refuse-leased'[\s\S]*?exit 3") 'a leased refusal exits 3'
    Check ($body -match "'refuse-unauthorized'[\s\S]*?exit 4") 'a token refusal exits 4'
    $kill = $body.LastIndexOf('Stop-Process -Id')
    $wait = $body.IndexOf('WaitForExit')
    $close = $body.LastIndexOf('Close-LabClients')
    Check ($kill -ge 0 -and $wait -gt $kill -and $close -gt $wait) 'clients close after the daemon is killed and waited for'
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) failed"; exit 1 }
Write-Host 'all passed'
