<#
.SYNOPSIS
    Tests for `lab uat -Chaos` (lab-chaos CH-02, CH-08 and CH-09), no Pester
    and no daemon: the header helpers, -Chaos reaching the request body, the
    plan's header, the cpu and -Leases check, the golden -Chaos needs, the
    chaos_divergence report, and a real run against a loopback MCP stub, so
    the lane's own record carries the run's header into the summary. Exits
    non-zero when any check fails:

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
# -Chaos needs the section's golden (CH-09): a temp spec folder holding one,
# and an empty one without. Removed at the end.
$script:goldenSpecs = Join-Path ([IO.Path]::GetTempPath()) ("uat-chaos-specs-" + [guid]::NewGuid())
$script:noGoldenSpecs = Join-Path ([IO.Path]::GetTempPath()) ("uat-chaos-nogolden-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path (Join-Path $script:goldenSpecs 'golden'), $script:noGoldenSpecs | Out-Null
Set-Content -LiteralPath (Join-Path $script:goldenSpecs 'golden\first-session.json') -Encoding utf8NoBOM `
    -Value '{"schema":1,"section":"first-session","fingerprint":{"rows":[]}}'

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
Check ((Select-UatChaosHeader $twoSeeds 'chaos=cpu') -ceq 'chaos=cpu seed=<per run>') 'runs that differ only by seed name the seed once as <per run>'
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
    function Get-DefaultSpecsDir { $script:goldenSpecs }
    function New-McpSession($Url, $Token) { 'session' }
    function Close-McpSession($Session) { }
    function Invoke-McpTool($Session, $Name, $Arguments, $TimeoutSec) {
        $script:sentArgs = $Arguments
        [pscustomobject]@{ rows = @([pscustomobject]@{ row = 'FS-01'; result = 'SKIPPED'; reasons = @() }) }
    }
    Invoke-LabUat
}
Check ((Invoke-UatPlanWithChaos 'cpu', 'flush:pack' 6>$null) -eq 0 -and @($script:sentArgs.chaos) -join '|' -ceq 'cpu|flush:pack') 'two -Chaos values both reach the request body'
Check ((Get-UatField $script:sentArgs 'fingerprint') -eq $true) '-Chaos implies -Fingerprint: the request asks for fingerprint capture (CH-09)'
Check ((Invoke-UatPlanWithChaos 'net:latency=120,loss=1' 6>$null) -eq 0 -and @($script:sentArgs.chaos).Count -eq 1 -and $script:sentArgs.chaos[0] -ceq 'net:latency=120,loss=1') 'a quoted multi-knob spec reaches the body as one element'
Check ((Invoke-UatPlanWithChaos 6>$null) -eq 0 -and -not $script:sentArgs.ContainsKey('chaos') -and -not $script:sentArgs.ContainsKey('fingerprint')) 'without -Chaos the body has no chaos or fingerprint key'

# A chaos run without a golden stops before the daemon is asked (CH-09).
# Every daemon call is stubbed to throw, so a regression fails here instead
# of reaching the live lab.
$script:daemonAsked = $false
$sw = [IO.StringWriter]::new(); $old = [Console]::Out; [Console]::SetOut($sw)
try {
    $code = & {
        . $uatScript -Section 'first-session' -Chaos 'flush:pack' -Json
        function Get-DefaultSpecsDir { $script:noGoldenSpecs }
        function Get-LabToken { $script:daemonAsked = $true; throw 'daemon reached' }
        function Get-LabStatus { $script:daemonAsked = $true; throw 'daemon reached' }
        function Get-LabdEnv { $script:daemonAsked = $true; throw 'daemon reached' }
        function Get-LabMcpUrl { $script:daemonAsked = $true; throw 'daemon reached' }
        function New-McpSession($Url, $Token) { $script:daemonAsked = $true; throw 'daemon reached' }
        function Invoke-McpTool($Session, $Name, $Arguments, $TimeoutSec) { $script:daemonAsked = $true; throw 'daemon reached' }
        try { Invoke-LabUat } catch { -1 }
    } 2>$null 6>$null
} finally { [Console]::SetOut($old) }
$refusalText = $sw.ToString().Trim()
$refusal = [string](Get-UatField $(if ($refusalText) { $refusalText | ConvertFrom-Json }) 'error')
Check ($code -eq 2 -and -not $script:daemonAsked -and $refusal -like 'no golden for first-session; -Chaos*lab golden record (lab-golden GD-10)') "-Chaos with no golden exits 2 before the daemon is asked ($code, $refusal)"

# Dot-sourced with the daemon stubbed; -Json output captured from
# [Console]::Out. $script:planReply is the plan call's chaos list, and
# $script:planThrows makes the call fail as the daemon's refusal would.
# $script:callSpecs is the local spec folder the call sees ($null: none).
$script:callSpecs = $script:goldenSpecs
function Invoke-UatChaosCall([string[]]$Chaos, [switch]$Plan, [switch]$DiffGolden) {
    $sw = [IO.StringWriter]::new(); $old = [Console]::Out; [Console]::SetOut($sw)
    try {
        $code = & {
            if ($Plan) { . $uatScript -Section 'first-session' -PlanOnly -Json -Chaos $Chaos -DiffGolden:$DiffGolden }
            else { . $uatScript -Section 'first-session' -Json -Chaos $Chaos -DiffGolden:$DiffGolden }
            function Get-LabToken { 'token' }
            function Get-LabStatus { [pscustomobject]@{ instances = @([pscustomobject]@{ instance = 'default'; lease = [pscustomobject]@{ held = $false } }) } }
            function Get-LabdEnv { @{} }
            function Get-Process { }
            function Get-LabMcpUrl { 'http://stub' }
            function Get-DefaultSpecsDir { $script:callSpecs }
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

# -Chaos -PlanOnly drives nothing, so it needs no golden (CH-09 review); a
# spec the runner refuses still fails the plan call.
$script:callSpecs = $script:noGoldenSpecs; $script:sentArgs = $null
$script:planReply = @('chaos=flush:pack seed=<per run>')
$r = Invoke-UatChaosCall 'flush:pack' -Plan
Check ($r.Code -eq 0 -and $r.Out.ok -and $script:sentArgs.plan_only) "-Chaos -PlanOnly with no golden plans and exits 0 ($($r.Code), $($r.Out | ConvertTo-Json -Compress))"
$script:planReply = $null; $script:planThrows = 'Invalid params: chaos "burst": burst is not wired yet; it waits on CH-13 (.spawnrandom ids, .despawn <id>)'
$r = Invoke-UatChaosCall 'burst' -Plan
Check ($r.Code -eq 3 -and $r.Out.error -like '*burst is not wired yet*') "-Chaos burst -PlanOnly with no golden still exits 3 from the plan call ($($r.Code))"
$script:planThrows = 'Invalid params: chaos "storm": unknown profile'
$r = Invoke-UatChaosCall 'storm' -Plan
Check ($r.Code -eq 3 -and $r.Out.error -like '*unknown profile*') "an unknown profile with -PlanOnly and no golden still exits 3 ($($r.Code))"
# No local spec folder: a chaos run leaves the golden to the daemon (it
# reaches the plan call, here failed by the stub), -DiffGolden still exits 2.
$script:callSpecs = $null; $script:sentArgs = $null; $script:planThrows = 'stub: no run'
$r = Invoke-UatChaosCall 'cpu'
Check ($r.Code -eq 3 -and $script:sentArgs -and -not $script:sentArgs.ContainsKey('specs_dir')) "with no local spec folder a chaos run is not refused for -SpecsDir ($($r.Code))"
$script:sentArgs = $null
$r = Invoke-UatChaosCall @() -DiffGolden
Check ($r.Code -eq 2 -and $null -eq $script:sentArgs -and $r.Out.error -like '-DiffGolden needs -SpecsDir*') "with no local spec folder -DiffGolden alone still exits 2 ($($r.Code))"
$script:callSpecs = $script:goldenSpecs; $script:planThrows = $null

# --- the report against the golden (CH-09, D-CH7) ---
$sixRows = @('FS-01', 'FS-02', 'FS-P1', 'FS-P2', 'FS-P3', 'FS-P4' | ForEach-Object { [pscustomobject]@{ row = $_; result = 'PASS'; reasons = @() } })
$divLine = 'row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995` after `c2s interact target=tag:ArmYourself_FrostBody`, got `s2c onDialogDisplay@self dialog_id=3996`'
function ChaosRec([string]$Golden, [string]$Chaos = 'chaos=flush:pack seed=7', [int]$Index = 1) {
    $r = Rec $true; $r.Rows = $sixRows; $r.Golden = $Golden; $r.Index = $Index
    if ($Chaos) { $r.Chaos = $Chaos }
    $r
}
$batchDir = 'C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261011-120000'
$diverged = Merge-UatResults @((ChaosRec $divLine)) @($lane) @()
Add-UatChaosHeader $diverged (Select-UatChaosHeader @((ChaosRec $divLine)) $null)
$dj = ConvertTo-UatCompactJson $diverged $batchDir
$d = $dj | ConvertFrom-Json
Check ($diverged.ok -and $d.ok -and $d.passed -eq 1 -and $d.rows.'FS-P3' -ceq '1/1') 'a chaos row that passed its expects but diverged keeps status pass'
Check (-not $diverged.Contains('golden') -and $d.PSObject.Properties.Name -notcontains 'golden') 'a chaos batch reports no golden key'
$dcd = Get-UatField $diverged 'chaos_divergence'
Check ((Get-UatField (Get-UatField $d 'chaos_divergence') 'diverged') -eq 1 -and $dcd -and @($dcd.groups).Count -eq 1 -and $dcd.groups[0].line -ceq $divLine) 'the divergence is one chaos_divergence line, unprefixed, as golden groups it'
Check ((@($dcd.groups[0].runs) -join '|') -ceq 'default#1 seed=7') "the group names its run with the run's seed ($(@($dcd.groups[0].runs) -join '|'))"
$dfirst = [string](Get-UatField (Get-UatField $d 'chaos_divergence') 'first')
Check ($dfirst.StartsWith('row FS-P3: expected') -and $dfirst.Length -le 120) "-Json's chaos_divergence.first is the divergence only, cut to 120 as golden's is ($($dfirst.Length))"
Check ($d.chaos -ceq 'chaos=flush:pack seed=7') 'the chaos header is the top-level chaos key, once'
Check ((Get-UatExitCode $diverged) -eq 0) 'a chaos-only divergence does not exit 5'
$dmd = Format-UatSummary $diverged 't' $batchDir
Check ($dmd -match 'Chaos divergence from the golden: 0 match, 1 diverged' -and $dmd.Contains("- 1x $divLine; runs: default#1 seed=7")) 'summary.md lists the chaos divergence with its run and seed'

# The -Json budget with realistic data (CH-09 review): 5 runs, 6 rows, a
# two-spec header with knobs, 20-digit seeds, one divergence on every run.
$seeds = @('12345678901234567890', '98765432109876543210', '11112222333344445555', '55554444333322221111', '10293847561029384756')
$fiveRuns = @(for ($i = 0; $i -lt 5; $i++) {
    ChaosRec $divLine "chaos=cpu:safe seed=$($seeds[$i]); chaos=net:latency=120,loss=1 seed=$($seeds[$i])" ($i + 1)
})
$big = Merge-UatResults $fiveRuns @($lane) @()
Add-UatChaosHeader $big (Select-UatChaosHeader $fiveRuns $null)
$bj = ConvertTo-UatCompactJson $big $batchDir
$bcd = Get-UatField $big 'chaos_divergence'
Check (@($bcd.groups).Count -eq 1 -and $bcd.groups[0].count -eq 5 -and $bcd.diverged -eq 5) "5 runs with the same divergence and different seeds are one group of 5 ($(@($bcd.groups).Count) group(s))"
Check ((@($bcd.groups[0].runs) -join '|') -ceq (@(for ($i = 0; $i -lt 5; $i++) { "default#$($i + 1) seed=$($seeds[$i])" }) -join '|')) 'the group lists each run by its seed (summary.md, batch.json)'
Check ($bj.Length -lt 600) "-Json for 5 chaos runs of 6 rows, a two-spec header and 20-digit seeds stays under 600 characters ($($bj.Length))"
Check ($bj -notmatch '12345678901234567890 row|seed=98765432109876543210') "-Json does not repeat the seeds per divergence: only the header's first ($bj)"

$matched = Merge-UatResults @((ChaosRec 'match')) @($lane) @()
$mj = ConvertTo-UatCompactJson $matched $batchDir
$mcd = Get-UatField ($mj | ConvertFrom-Json) 'chaos_divergence'
Check ($mcd -and $mcd.match -eq 1 -and $mcd.diverged -eq 0 -and -not $matched.Contains('golden') -and (Get-UatExitCode $matched) -eq 0) "a chaos run that matched gives chaos_divergence {match: 1, diverged: 0}, exit 0 ($mj)"
$never = Merge-UatResults @((ChaosRec $null)) @($lane) @()
Check (-not $never.Contains('chaos_divergence') -and -not $never.Contains('golden')) 'a chaos run never compared has no chaos_divergence key'
$unread = Merge-UatResults @((ChaosRec 'error: cannot_diff: run x has no run.json'), (ChaosRec $divLine)) @($lane) @()
$ucd = Get-UatField $unread 'chaos_divergence'
Check ($ucd -and $ucd.errors -eq 1 -and @($ucd.groups | ForEach-Object { $_.line }) -contains 'error: cannot_diff: run x has no run.json' -and (Get-UatExitCode $unread) -eq 5) 'under chaos a diff that could not be made still exits 5'
$plain = Merge-UatResults @((ChaosRec $divLine -Chaos '')) @($lane) @()
Check ((Get-UatField (Get-UatField $plain 'golden') 'diverged') -eq 1 -and -not $plain.Contains('chaos_divergence') -and (Get-UatExitCode $plain) -eq 5) 'without chaos the same divergence stays under golden and exits 5'
$mixed = Merge-UatResults @((ChaosRec $divLine), (ChaosRec 'row FS-01: expected `a`, got `b`' -Chaos '' -Index 2)) @($lane) @()
$mg = Get-UatField $mixed 'golden'; $mc = Get-UatField $mixed 'chaos_divergence'
Check ($mg -and $mg.diverged -eq 1 -and $mg.groups[0].line -ceq 'row FS-01: expected `a`, got `b`' -and $mc -and $mc.diverged -eq 1 -and $mc.groups[0].line -ceq $divLine -and (Get-UatExitCode $mixed) -eq 5) 'a mixed batch: the run without a header stays under golden (exit 5), the chaos run under chaos_divergence'

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
                $result = if ($req.params.name -eq 'lab_golden_diff') {
                    @{ ok = $true; match = $false; first = 'row FS-01: expected `a`, got `b`' }
                } elseif ($plan) {
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
            function Get-DefaultSpecsDir { $script:goldenSpecs }
            Invoke-LabUat
        } 6>$null
    } finally { [Console]::SetOut($old) }
    $text = $sw.ToString().Trim()
    $real = if ($text) { $text | ConvertFrom-Json } else { $null }
    Check ($code -eq 0 -and $real -and $real.chaos -ceq 'chaos=cpu:safe seed=4242') "a real run's summary carries the run's header, not the plan's ($code, $($real | ConvertTo-Json -Compress))"
    Check ($real -and (Get-UatField (Get-UatField $real 'chaos_divergence') 'first') -ceq 'row FS-01: expected `a`, got `b`' -and $real.PSObject.Properties.Name -notcontains 'golden') "a real chaos run is diffed and reports chaos_divergence (the header as the chaos key), exit 0 ($code)"
    $jsonl = @(Get-ChildItem -LiteralPath $uatDir -Recurse -Filter 'batch.jsonl')
    $recs = @(if ($jsonl.Count) { Read-UatRecords $jsonl[0].FullName })
    Check ($recs.Count -eq 1 -and (Get-UatField $recs[0] 'Chaos') -ceq 'chaos=cpu:safe seed=4242') 'the lane writes the run header into its batch.jsonl record'
} finally {
    $env:CIMMERIA_LAB_UAT_DIR = $oldUatDir
    $listener.Stop()
    try { $null = $server.EndInvoke($serverRun) } catch { }
    $server.Dispose()
    Remove-Item -LiteralPath $uatDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $script:goldenSpecs, $script:noGoldenSpecs -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
