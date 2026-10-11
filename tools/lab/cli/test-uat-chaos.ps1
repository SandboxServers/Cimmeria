<#
.SYNOPSIS
    Tests for `lab uat -Chaos` (lab-chaos CH-02 and CH-08), no Pester and no
    daemon: the header helpers, -Chaos reaching the request body, the plan's
    header, the cpu and -Leases check, and a real run against a loopback MCP
    stub, so the lane's own record carries the run's header into the
    summary. Exits non-zero when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-uat-chaos.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'uat-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

function Rec([bool]$ok) {
    @{ Lane = 1; Instance = 'default'; Index = 1; Ok = $ok; Rows = @(); RunDir = 'd\default-1'; Error = $null; Seconds = 90
       Verdict = @{ Ok = $ok; Row = $null; Result = $null; Reason = $null; Issue = $null } }
}
$lane = @{ Lane = 1; Instance = 'default' }
$uatScript = Join-Path $PSScriptRoot 'uat.ps1'

# --- the typed header (CH-02, D-CH7) ---
Check ((Format-UatChaosHeader @('cpu', 'flush:pack')) -ceq 'chaos=cpu; chaos=flush:pack') 'the chaos header has one chaos=<spec> per value, joined with "; "'
Check ($null -eq (Format-UatChaosHeader @())) 'no chaos gives no header'
Check ((Format-UatChaosHeader @('net:latency=120,loss=1')) -ceq 'chaos=net:latency=120,loss=1') 'a quoted multi-knob spec stays one value'
$cs = Merge-UatResults @((Rec $true)) @($lane) @()
$cs['chaos'] = Format-UatChaosHeader @('cpu', 'flush:pack')
Check ((ConvertTo-UatCompactJson $cs 'C:\b' | ConvertFrom-Json).chaos -ceq 'chaos=cpu; chaos=flush:pack') '-Json has the chaos string when the run had chaos'
Check ((Format-UatSummary $cs 't' 'C:\b').StartsWith('chaos=cpu; chaos=flush:pack')) 'the chaos header leads the summary'
$body = @{}
Check ((Set-UatChaos $body @('', 'cpu')) -ceq 'chaos=cpu' -and (@($body.chaos) -join '|') -ceq 'cpu') 'an empty -Chaos value reaches neither the body nor the header'
$body = @{}
Check ($null -eq (Set-UatChaos $body @()) -and -not $body.ContainsKey('chaos')) 'no -Chaos leaves the body without a chaos key'
$wired = Merge-UatResults @((Rec $true)) @($lane) @()
Add-UatChaosHeader $wired (Set-UatChaos @{} @('flush:pack'))
Check ((ConvertTo-UatCompactJson $wired 'C:\b' | ConvertFrom-Json).chaos -ceq 'chaos=flush:pack') 'the header attached to a merged summary reaches -Json'
$noChaos = Merge-UatResults @((Rec $true)) @($lane) @()
Add-UatChaosHeader $noChaos $null
Check ((ConvertTo-UatCompactJson $noChaos 'C:\b') -notmatch '"chaos"' -and (Format-UatSummary $noChaos 't' 'C:\b') -notmatch 'chaos=') 'a run without chaos has no chaos key or header'

# --- the runner's header (CH-08) ---
Check ((Get-UatRunChaos ([pscustomobject]@{ chaos = @('chaos=cpu seed=5', 'chaos=flush:pack seed=5') })) -ceq 'chaos=cpu seed=5; chaos=flush:pack seed=5') "the runner's headers join with '; '"
Check ($null -eq (Get-UatRunChaos ([pscustomobject]@{ rows = @() }))) 'a result without chaos gives no header'
$oneSeed = @([pscustomobject]@{ Chaos = 'chaos=cpu seed=5' }, [pscustomobject]@{ Chaos = 'chaos=cpu seed=5' })
Check ((Select-UatChaosHeader $oneSeed 'chaos=cpu') -ceq 'chaos=cpu seed=5') "the runs' header replaces the typed one, not appended to it"
$twoSeeds = @([pscustomobject]@{ Chaos = 'chaos=cpu seed=5' }, [pscustomobject]@{ Chaos = 'chaos=cpu seed=9' }, [pscustomobject]@{ Error = 'x' })
Check ((Select-UatChaosHeader $twoSeeds 'chaos=cpu') -ceq 'chaos=cpu seed=5 (+1 other seed(s) in batch.jsonl)') 'runs with different seeds keep the first header and count the rest'
Check ((Select-UatChaosHeader @([pscustomobject]@{ Error = 'x' }) 'chaos=cpu') -ceq 'chaos=cpu') 'no run returned a header: the typed one stays'
Check ($null -eq (Select-UatChaosHeader @() $null)) 'no chaos at all gives no header'

# --- cpu needs one lease (CH-08 review) ---
Check ((Test-UatChaosLeases @('cpu:cores=2', 'flush:pack') 2) -like '-Chaos cpu loads the whole machine*') 'cpu with -Leases 2 is refused'
Check ($null -eq (Test-UatChaosLeases @('cpu') 1)) 'cpu with -Leases 1 is fine'
Check ($null -eq (Test-UatChaosLeases @('net', 'flush:pack') 3)) 'net and flush with -Leases 3 are fine (each run its own session)'
Check ($null -eq (Test-UatChaosLeases @() 3)) 'no chaos with -Leases 3 is fine'
# Dot-sourced, never a child process: every daemon call is stubbed to throw,
# so a regression fails this check instead of starting runs on the live lab
# (a child `pwsh -File uat.ps1` reaches the real daemon; that happened once).
$script:daemonAsked = $false
$code = & {
    . $uatScript -Section 'first-session' -Chaos 'cpu' -Leases 2
    function Get-LabToken { $script:daemonAsked = $true; throw 'daemon reached' }
    function Get-LabStatus { $script:daemonAsked = $true; throw 'daemon reached' }
    function Get-LabdEnv { $script:daemonAsked = $true; throw 'daemon reached' }
    function Get-LabMcpUrl { $script:daemonAsked = $true; throw 'daemon reached' }
    function New-McpSession($Url, $Token) { $script:daemonAsked = $true; throw 'daemon reached' }
    try { Invoke-LabUat } catch { -1 }
} 2>$null 6>$null
Check ($code -eq 2 -and -not $script:daemonAsked) "-Chaos cpu -Leases 2 exits 2 before the daemon is asked ($code)"

# -Chaos reaches the lab_uat_run request body. uat.ps1 is dot-sourced with
# the daemon calls stubbed, and -PlanOnly stops before anything is driven.
$script:sentArgs = $null
function Invoke-UatPlanWithChaos([string[]]$Chaos) {
    . $uatScript -Section 'first-session' -PlanOnly -Chaos $Chaos
    function Get-LabToken { 'token' }
    function Get-LabStatus { [pscustomobject]@{ instances = @([pscustomobject]@{ instance = 'default'; lease = [pscustomobject]@{ held = $false } }) } }
    function Get-LabdEnv { @{} }
    function Get-LabMcpUrl { 'http://stub' }
    function Get-DefaultSpecsDir { '' }
    function New-McpSession($Url, $Token) { 'session' }
    function Close-McpSession($Session) { }
    function Invoke-McpTool($Session, $Name, $Arguments, $TimeoutSec) {
        $script:sentArgs = $Arguments
        [pscustomobject]@{ rows = @([pscustomobject]@{ row = 'FS-01'; result = 'SKIPPED'; reasons = @() }) }
    }
    Invoke-LabUat
}
Check ((Invoke-UatPlanWithChaos 'cpu', 'flush:pack' 6>$null) -eq 0 -and @($script:sentArgs.chaos) -join '|' -ceq 'cpu|flush:pack') 'two -Chaos values both reach the request body'
Check ((Invoke-UatPlanWithChaos 'net:latency=120,loss=1' 6>$null) -eq 0 -and @($script:sentArgs.chaos).Count -eq 1 -and $script:sentArgs.chaos[0] -ceq 'net:latency=120,loss=1') 'a quoted multi-knob spec reaches the body as one element'
Check ((Invoke-UatPlanWithChaos 6>$null) -eq 0 -and -not $script:sentArgs.ContainsKey('chaos')) 'without -Chaos the body has no chaos key'

# Dot-sourced with the daemon stubbed; -Json output captured from
# [Console]::Out. $script:planReply is the plan call's chaos list, and
# $script:planThrows makes the call fail as the daemon's refusal would.
function Invoke-UatChaosCall([string[]]$Chaos, [switch]$Plan) {
    $sw = [IO.StringWriter]::new(); $old = [Console]::Out; [Console]::SetOut($sw)
    try {
        $code = & {
            if ($Plan) { . $uatScript -Section 'first-session' -PlanOnly -Json -Chaos $Chaos }
            else { . $uatScript -Section 'first-session' -Json -Chaos $Chaos }
            function Get-LabToken { 'token' }
            function Get-LabStatus { [pscustomobject]@{ instances = @([pscustomobject]@{ instance = 'default'; lease = [pscustomobject]@{ held = $false } }) } }
            function Get-LabdEnv { @{} }
            function Get-Process { }
            function Get-LabMcpUrl { 'http://stub' }
            function Get-DefaultSpecsDir { '' }
            function New-McpSession($Url, $Token) { 'session' }
            function Close-McpSession($Session) { }
            function Invoke-McpTool($Session, $Name, $Arguments, $TimeoutSec) {
                $script:sentArgs = $Arguments
                if ($script:planThrows) { throw $script:planThrows }
                $r = [pscustomobject]@{ rows = @([pscustomobject]@{ row = 'FS-01'; result = 'SKIPPED'; reasons = @() }) }
                if ($script:planReply) { $r | Add-Member -NotePropertyName chaos -NotePropertyValue $script:planReply }
                $r
            }
            Invoke-LabUat
        } 6>$null
    } finally { [Console]::SetOut($old) }
    $text = $sw.ToString().Trim()
    return @{ Code = $code; Out = $(if ($text) { $text | ConvertFrom-Json } else { $null }) }
}
$script:planReply = @('chaos=cpu:safe seed=<per run>', 'chaos=flush:pack seed=5'); $script:planThrows = $null
$r = Invoke-UatChaosCall 'cpu', 'flush:pack' -Plan
Check ($r.Code -eq 0 -and $r.Out.chaos -ceq 'chaos=cpu:safe seed=<per run>; chaos=flush:pack seed=5') "the plan prints the runner's header in place of the typed one ($($r.Out.chaos))"
$script:planReply = $null; $script:planThrows = 'Invalid params: chaos "burst": burst is not wired yet; it waits on CH-13 (.spawnrandom ids, .despawn <id>)'
$r = Invoke-UatChaosCall 'burst' -Plan
Check ($r.Code -eq 3 -and -not $r.Out.ok -and $r.Out.error -like '*burst is not wired yet*') "a spec the runner refuses fails the plan call with exit 3 ($($r.Code))"
# A real run is not refused up front: -Chaos reaches the plan call (the stub
# then fails it, so nothing is driven).
$script:sentArgs = $null; $script:planThrows = 'stub: no run'
$r = Invoke-UatChaosCall 'cpu'
Check ($r.Code -eq 3 -and (@($script:sentArgs.chaos) -join '|') -ceq 'cpu' -and $script:sentArgs.plan_only) "a real run with -Chaos sends it to the plan call ($($r.Code))"
$script:planThrows = $null

# --- a real run through the lanes, against a loopback MCP stub ---
# The lanes run in ForEach-Object -Parallel runspaces with the real
# uat-lib.ps1, so a function stub cannot reach them; a TCP listener speaking
# just enough MCP over HTTP can. The plan answers seed=<per run>, the run
# its own seed: the summary must carry the run's.
$listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
$listener.Start()
$port = $listener.LocalEndpoint.Port
$server = [powershell]::Create()
[void]$server.AddScript({
    param($Listener)
    while ($true) {
        try { $client = $Listener.AcceptTcpClient() } catch { return }
        try {
            $stream = $client.GetStream()
            $reader = [IO.StreamReader]::new($stream, [Text.Encoding]::UTF8, $false, 4096, $true)
            $method = ($reader.ReadLine() -split ' ')[0]
            $len = 0
            while ($line = $reader.ReadLine()) { if ($line -match '^Content-Length:\s*(\d+)') { $len = [int]$Matches[1] } }
            $buf = [char[]]::new([Math]::Max($len, 1)); $got = 0
            while ($got -lt $len) { $n = $reader.Read($buf, $got, $len - $got); if ($n -le 0) { break }; $got += $n }
            $req = if ($got) { [string]::new($buf, 0, $got) | ConvertFrom-Json } else { $null }
            $reply = ''
            if ($method -eq 'POST' -and $req.method -eq 'initialize') {
                $reply = @{ jsonrpc = '2.0'; id = $req.id; result = @{ protocolVersion = '2025-06-18'; capabilities = @{} } } | ConvertTo-Json -Compress -Depth 10
            } elseif ($method -eq 'POST' -and $req.method -eq 'tools/call') {
                $plan = [bool]$req.params.arguments.plan_only
                $result = if ($plan) {
                    @{ rows = @(@{ row = 'FS-01'; result = 'SKIPPED'; reasons = @() }); chaos = @('chaos=cpu:safe seed=<per run>') }
                } else {
                    @{ run_dir = 'C:\stub\run'; rows = @(@{ row = 'FS-01'; result = 'PASS'; reasons = @() }); chaos = @('chaos=cpu:safe seed=4242') }
                }
                $text = $result | ConvertTo-Json -Compress -Depth 10
                $reply = @{ jsonrpc = '2.0'; id = $req.id; result = @{ content = @(@{ type = 'text'; text = $text }) } } | ConvertTo-Json -Compress -Depth 10
            }
            $bytes = [Text.Encoding]::UTF8.GetBytes($reply)
            $head = [Text.Encoding]::ASCII.GetBytes("HTTP/1.1 200 OK`r`nContent-Type: application/json`r`nContent-Length: $($bytes.Length)`r`nConnection: close`r`n`r`n")
            $stream.Write($head, 0, $head.Length); $stream.Write($bytes, 0, $bytes.Length); $stream.Flush()
        } catch { } finally { $client.Close() }
    }
}).AddArgument($listener)
$serverRun = $server.BeginInvoke()
$uatDir = Join-Path ([IO.Path]::GetTempPath()) ("uat-chaos-" + [guid]::NewGuid())
$oldUatDir = $env:CIMMERIA_LAB_UAT_DIR
$env:CIMMERIA_LAB_UAT_DIR = $uatDir
$script:stubUrl = "http://127.0.0.1:$port/mcp"
try {
    $sw = [IO.StringWriter]::new(); $old = [Console]::Out; [Console]::SetOut($sw)
    try {
        $code = & {
            . $uatScript -Section 'first-session' -Json -Chaos 'cpu'
            function Get-LabToken { 'token' }
            function Get-LabStatus { [pscustomobject]@{ instances = @([pscustomobject]@{ instance = 'default'; lease = [pscustomobject]@{ held = $false } }) } }
            function Get-LabdEnv { @{} }
            function Get-Process { }
            function Get-LabMcpUrl { $script:stubUrl }
            function Get-DefaultSpecsDir { '' }
            Invoke-LabUat
        } 6>$null
    } finally { [Console]::SetOut($old) }
    $text = $sw.ToString().Trim()
    $real = if ($text) { $text | ConvertFrom-Json } else { $null }
    Check ($code -eq 0 -and $real -and $real.chaos -ceq 'chaos=cpu:safe seed=4242') "a real run's summary carries the run's header, not the plan's ($code, $($real | ConvertTo-Json -Compress))"
    $jsonl = @(Get-ChildItem -LiteralPath $uatDir -Recurse -Filter 'batch.jsonl')
    $recs = @(if ($jsonl.Count) { Read-UatRecords $jsonl[0].FullName })
    Check ($recs.Count -eq 1 -and (Get-UatField $recs[0] 'Chaos') -ceq 'chaos=cpu:safe seed=4242') 'the lane writes the run header into its batch.jsonl record'
} finally {
    $env:CIMMERIA_LAB_UAT_DIR = $oldUatDir
    $listener.Stop()
    try { $null = $server.EndInvoke($serverRun) } catch { }
    $server.Dispose()
    Remove-Item -LiteralPath $uatDir -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
