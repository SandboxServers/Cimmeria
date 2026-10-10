<#
.SYNOPSIS
    Tests for `lab uat` (uat-lib.ps1), no Pester and no daemon. Exits non-zero
    when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-uat.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'uat-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$hosted = @('default', 'p2', 'p3', 'p4', 'p5')

# --- planning ---
$p = Get-UatPlan 1 1 $hosted @() 0 ''
Check ($null -eq $p.Error -and $p.Lanes.Count -eq 1 -and $p.Lanes[0].Instance -eq 'default') 'one lease runs on the first instance'
$p = Get-UatPlan 3 20 $hosted @('p2') 0 ''
Check ((@($p.Lanes.Instance) -join ',') -eq 'default,p3,p4') 'lanes skip an instance another session leased'
Check ((Get-UatPlan 0 1 $hosted @() 0 '').Error -like '-Leases must be 1 to 5') '0 leases is refused'
Check ((Get-UatPlan 6 1 $hosted @() 0 '').Error -like '-Leases must be 1 to 5') '6 leases is refused'
Check ((Get-UatPlan 1 21 $hosted @() 0 '').Error -like '-RunsPerLease must be 1 to 20') '21 runs per lease is refused'
Check ((Get-UatPlan 3 1 @('default', 'p2') @() 0 '').Error -like '*more than the lab allows*') 'more leases than hosted instances is refused'
Check ((Get-UatPlan 3 1 $hosted @() 2 '').Error -like '*max clients 2*') 'the client cap limits leases'
Check ((Get-UatPlan 2 1 @('default', 'p2') @('p2') 0 '').Error -like 'only 1 free instance*') 'too few free instances is refused, naming the busy ones'
$p = Get-UatPlan 1 5 $hosted @() 0 'p3'
Check ($p.Lanes.Count -eq 1 -and $p.Lanes[0].Instance -eq 'p3') '-Instance picks that lane'
Check ((Get-UatPlan 2 1 $hosted @() 0 'p3').Error -like '-Instance runs one lane*') '-Instance with 2 leases is refused'
Check ((Get-UatPlan 1 1 $hosted @() 0 'p9').Error -like "unknown instance 'p9'*") 'an unknown instance is refused'
Check ((Get-UatPlan 1 1 $hosted @('p3') 0 'p3').Error -like '*leased by another session*') 'a leased -Instance is refused'

# --- MCP body parsing ---
Check ($null -eq (Read-McpBody '')) 'an empty body (a notification) reads as nothing'
Check ((Read-McpBody '{"id":2,"result":{"x":1}}').result.x -eq 1) 'plain JSON body'
Check ((Read-McpBody "event: message`ndata: {`"id`":2,`"result`":{`"x`":3}}`n`n").result.x -eq 3) 'SSE body: the data line'

# --- verdicts ---
function Row($id, $result, $reason) { [pscustomobject]@{ row = $id; result = $result; reasons = @($reason) } }
$v = Get-UatRunVerdict @((Row 'FS-01' 'PASS' ''), (Row 'FS-P1' 'PASS' ''))
Check ($v.Ok) 'all PASS is ok'
$v = Get-UatRunVerdict @((Row 'FS-P1' 'PASS' ''), (Row 'FS-P2' 'FAIL' 'clause movie-over failed: client_entity_find/count gte 1'),
    (Row 'FS-P3' 'FAIL' 'step action 1 failed'))
Check (-not $v.Ok -and $v.Row -eq 'FS-P2' -and $v.Issue -eq '#1341') 'the first failing row decides; FS-P2 movie-over is known #1341'
$v = Get-UatRunVerdict @((Row 'FS-P3' 'FAIL' 'clause aim failed'))
Check (-not $v.Ok -and $null -eq $v.Issue) 'any other failure is new'
$v = Get-UatRunVerdict @((Row 'FS-P1' 'BLOCKED' 'setup failed'))
Check (-not $v.Ok -and $v.Result -eq 'BLOCKED') 'BLOCKED fails a run'

# --- brake ---
Check (-not (Test-UatBrake @($false, $false, $true, $false) 3)) 'a pass resets the failure streak'
Check (Test-UatBrake @($true, $false, $false, $false) 3) 'three failures in a row brake the lane'
Check (-not (Test-UatBrake @($false, $false, $false, $false) 0)) 'a brake of 0 never stops'

# --- aggregation ---
function Rec($inst, $i, [bool]$ok, $rows, $err = $null) {
    @{ Lane = 1; Instance = $inst; Index = $i; Ok = $ok; Rows = $rows; RunDir = "d\$inst-$i"; Error = $err; Seconds = 90
       Verdict = $(if ($err) { @{ Ok = $false; Row = $null; Result = $null; Reason = $err; Issue = $null } } else { Get-UatRunVerdict $rows }) }
}
$good = @((Row 'FS-P1' 'PASS' ''), (Row 'FS-P2' 'PASS' ''))
$frost = @((Row 'FS-P1' 'PASS' ''), (Row 'FS-P2' 'FAIL' 'clause movie-over failed'))
$runs = @((Rec 'default' 1 $true $good), (Rec 'default' 2 $false $frost), (Rec 'p2' 1 $false $frost),
          (Rec 'p2' 2 $false @() 'lab_uat_run failed: bridge down'))
$lanes = @(@{ Lane = 1; Instance = 'default' }, @{ Lane = 2; Instance = 'p2' })
$s = Merge-UatResults $runs $lanes @('p2')
Check ($s.runs -eq 4 -and $s.passed -eq 1 -and $s.failed -eq 3) 'totals'
Check ($s.failed_known -eq 2 -and $s.failed_new -eq 1) 'known (#1341) and new failures are counted apart'
Check ($s.rows['FS-P2'].pass -eq 1 -and $s.rows['FS-P2'].total -eq 3) 'per-row pass counts (an errored run has no rows)'
$frostGroup = @($s.failures | Where-Object { $_.issue -eq '#1341' })[0]
Check ($frostGroup.count -eq 2 -and (@($frostGroup.runs) -join ',') -eq 'default#2,p2#1') 'identical failures group with their runs'
Check ((@($s.lanes | Where-Object { $_.instance -eq 'p2' })[0]).braked) 'a braked lane is marked'
Check ((Get-UatExitCode $s) -eq 4) 'a braked lane exits 4'
$s2 = Merge-UatResults @((Rec 'default' 1 $false $frost)) @($lanes[0]) @()
Check ((Get-UatExitCode $s2) -eq 1) 'a failed run exits 1'
$s3 = Merge-UatResults @((Rec 'default' 1 $true $good)) @($lanes[0]) @()
Check ((Get-UatExitCode $s3) -eq 0 -and $s3.ok) 'all passed exits 0'

# --- compact JSON for agents ---
$j = ConvertTo-UatCompactJson $s 'C:\b' 1 | ConvertFrom-Json
Check ($j.rows.'FS-P2' -eq '1/3') 'rows read as pass/total'
Check (@($j.failures).Count -eq 1 -and $j.more_failure_groups -eq 1) 'failure groups are capped, with a count of the rest'
Check ($j.failures[0].issue -eq '#1341' -and $j.failures[0].n -eq 2) 'the biggest group comes first, tagged'
$jsonText = ConvertTo-UatCompactJson $s3 'C:\b'
Check ($jsonText -notmatch 'null' -and $jsonText -notmatch 'failures') 'a clean batch has no nulls and no failure list'
Check ($jsonText.Length -lt 300) "a clean batch is short ($($jsonText.Length) chars)"

$md = Format-UatSummary $s 'lab uat first-session' 'C:\b'
Check ($md -match 'FAIL \(1/4 runs passed\)' -and $md -match 'known: #1341') 'the Markdown summary names the verdict and the known issue'

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
