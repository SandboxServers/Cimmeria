<#
.SYNOPSIS
    Tests for `lab golden` (golden-lib.ps1, golden.ps1's pre-daemon paths),
    no Pester and no daemon. Exits non-zero when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-golden.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'golden-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$sha = '1a2b3c4'

# --- argument rules ---
Check ($null -eq (Test-GoldenArguments 'record' 'first-session' 5 1 $sha)) 'five serial runs with a build are valid'
Check ($null -eq (Test-GoldenArguments 'record' 'first-session' 10 2 ('0123456789abcdef' * 2 + '01234567'))) 'ten runs on two lanes with a 40-hex build are valid'
Check ($null -eq (Test-GoldenArguments 'show' 'first-session' 5 1 '')) 'show needs no build'
Check ((Test-GoldenArguments 'bless' 'first-session' 5 1 $sha) -like 'usage: lab golden record|show*') 'an unknown action is refused'
Check ((Test-GoldenArguments '' 'first-session' 5 1 $sha) -like 'usage:*') 'no action is refused'
Check ((Test-GoldenArguments 'record' '' 5 1 $sha) -like 'usage: lab golden record <section>') 'a section is required'
Check ((Test-GoldenArguments 'show' '..\..\secrets' 5 1 '') -like '*is not a section id*') 'a section cannot reach outside golden/'
Check ((Test-GoldenArguments 'record' 'a..b' 5 1 $sha) -like '*is not a section id*') 'a section with .. is refused'
Check ((Test-GoldenArguments 'record' 'first-session' 4 1 $sha) -like '-Runs must be 5 to 20*') 'fewer than five runs is impossible'
Check ((Test-GoldenArguments 'record' 'first-session' 1 1 $sha) -like '-Runs must be 5 to 20*') 'one run is impossible'
Check ((Test-GoldenArguments 'record' 'first-session' 21 1 $sha) -like '-Runs must be 5 to 20*') 'more than twenty runs is refused'
Check ((Test-GoldenArguments 'show' 'first-session' 4 1 '') -like '-Runs must be*') 'the run rule holds for show too'
Check ((Test-GoldenArguments 'record' 'first-session' 5 0 $sha) -like '-Leases must be 1 to 5') 'zero leases is refused'
Check ((Test-GoldenArguments 'record' 'first-session' 12 6 $sha) -like '-Leases must be 1 to 5') 'six leases is refused'
Check ((Test-GoldenArguments 'record' 'first-session' 5 2 $sha) -like '*does not split evenly*') 'runs that do not split over the lanes are refused'
Check ((Test-GoldenArguments 'record' 'first-session' 5 1 '') -like '-ServerVersion must be*') 'record without a build is refused'
Check ((Test-GoldenArguments 'record' 'first-session' 5 1 '1a2b3c') -like '-ServerVersion must be*') 'a 6-character build is refused'
Check ((Test-GoldenArguments 'record' 'first-session' 5 1 '1A2B3C4') -like '-ServerVersion must be*') 'an uppercase build is refused'
Check ((Test-GoldenArguments 'record' 'first-session' 5 1 ('a' * 41)) -like '-ServerVersion must be*') 'a 41-character build is refused'
Check ((Test-GoldenArguments 'record' 'first-session' 5 1 'main') -like '-ServerVersion must be*') 'a branch name is not a build'
Check ((Get-GoldenPath 'C:\specs' 'first-session') -eq (Join-Path 'C:\specs' 'golden\first-session.json')) 'the golden sits at <specs>\golden\<section>.json'
# The daemon names the file with evidence.rs path_safe: outside [A-Za-z0-9_-] is '_'.
Check ($null -eq (Test-GoldenArguments 'record' 'v1.2' 5 1 $sha)) 'a section with a dot is valid'
Check ((Get-GoldenPath 'C:\specs' 'v1.2') -eq (Join-Path 'C:\specs' 'golden\v1_2.json')) 'v1.2 is stored as golden\v1_2.json, as the daemon names it'
Check ((Get-GoldenPath 'C:\specs' 'gm_Parity-2') -eq (Join-Path 'C:\specs' 'golden\gm_Parity-2.json')) 'letters, digits, _ and - keep their case'
Check ((Test-GoldenArguments 'show' 'a/b' 5 1 '') -like '*is not a section id*') 'a section with / is refused'
Check ((Test-GoldenArguments 'show' 'a\b' 5 1 '') -like '*is not a section id*') 'a section with \ is refused'
Check ((Test-GoldenArguments 'show' ([string][char]0x212A) 5 1 '') -like '*is not a section id*') 'a non-ASCII letter (Kelvin sign) is refused'

# --- the child lab uat arguments ---
$specsSp = 'C:\My Specs\uat specs'
$ba = Get-GoldenBatchArgs 'first-session' @('FS-01', 'FS-P1') 10 2 $sha $specsSp
function ArgAfter($list, [string]$name) { $i = [array]::IndexOf(@($list), $name); if ($i -lt 0) { return $null }; return @($list)[$i + 1] }
Check ($ba[0] -eq 'first-session') 'the section comes first'
Check ($ba -contains '-Fingerprint') 'the runs are fingerprint runs'
Check ((ArgAfter $ba '-MaxConsecutiveFailures') -eq 1) 'the lane brake is one failed run'
Check ((ArgAfter $ba '-RunsPerLease') -eq 5 -and (ArgAfter $ba '-Leases') -eq 2) '-RunsPerLease is Runs / Leases'
Check ((ArgAfter $ba '-SpecsDir') -ceq $specsSp) 'a -SpecsDir with spaces stays one element'
Check ((ArgAfter $ba '-ServerVersion') -ceq $sha) 'the build goes to the runs'
Check ((ArgAfter $ba '-Rows') -ceq 'FS-01,FS-P1') 'the rows go as one comma list'
Check ($ba -contains '-ProgressToStderr' -and $ba[-1] -eq '-Json') 'progress goes to stderr and the result is JSON'
$ba1 = Get-GoldenBatchArgs 'first-session' @() 5 1 $sha ''
Check ($ba1 -notcontains '-Rows' -and $ba1 -notcontains '-SpecsDir' -and (ArgAfter $ba1 '-RunsPerLease') -eq 5) 'no rows and no spec folder add no arguments'
Check ((Get-GoldenBatchArgs 'first-session' $null 5 1 $sha '') -notcontains '-Rows') 'null rows add no -Rows'
$threw = $false; try { Get-GoldenBatchArgs 'first-session' @() 5 2 $sha '' | Out-Null } catch { $threw = $_.Exception.Message -like '*does not split evenly*' }
Check $threw 'an uneven split is refused, as Test-GoldenArguments refuses it'

# --- run folders from batch.jsonl records ---
function Rec([bool]$ok, $dir) { [pscustomobject]@{ Lane = 1; Instance = 'default'; Ok = $ok; RunDir = $dir } }
$dirs = Get-GoldenRunDirs @((Rec $true 'C:\r\a'), (Rec $true 'C:\r\b'), (Rec $true 'C:\r\c'))
Check ((@($dirs) -join '|') -eq 'C:\r\a|C:\r\b|C:\r\c') 'every record''s RunDir, in order'
Check ($null -eq (Get-GoldenRunDirs @((Rec $true 'C:\r\a'), (Rec $false 'C:\r\b')))) 'a failed run gives no run folders'
Check ($null -eq (Get-GoldenRunDirs @((Rec $true 'C:\r\a'), (Rec $true $null)))) 'a run without a RunDir gives no run folders'
Check ($null -eq (Get-GoldenRunDirs @((Rec $true 'C:\r\a'), (Rec $true '')))) 'a run with an empty RunDir gives no run folders'
Check ($null -eq (Get-GoldenRunDirs @())) 'no records give no run folders'
Check ($null -eq (Get-GoldenRunDirs $null)) 'a missing batch gives no run folders'
$one = Get-GoldenRunDirs @((Rec $true 'C:\r\only'))
Check (@($one).Count -eq 1 -and @($one)[0] -eq 'C:\r\only') 'one record gives one folder'
# Through the real JSONL writer and reader, as golden.ps1 reads them.
$jl = Join-Path ([System.IO.Path]::GetTempPath()) ("golden-test-" + [guid]::NewGuid() + '.jsonl')
try {
    foreach ($i in 1..5) {
        Add-UatRecord $jl ([ordered]@{ Lane = 1; Instance = 'default'; Index = $i; Ok = $true; Rows = @(); RunDir = "C:\runs\run-$i"; Error = $null })
    }
    $back = Get-GoldenRunDirs (Read-UatRecords $jl)
    Check ((@($back) -join ',') -eq 'C:\runs\run-1,C:\runs\run-2,C:\runs\run-3,C:\runs\run-4,C:\runs\run-5') 'five passing batch.jsonl records give five run folders'
    Add-UatRecord $jl ([ordered]@{ Lane = 1; Instance = 'default'; Index = 0; Ok = $false; Rows = @(); RunDir = $null; Error = 'lane: boom' })
    Check ($null -eq (Get-GoldenRunDirs (Read-UatRecords $jl))) 'a lane error record in batch.jsonl gives no run folders'
} finally { Remove-Item -LiteralPath $jl -ErrorAction SilentlyContinue }

# --- the child lab uat -Json result ---
$batchDir = 'C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261011-101500'
$okLine = '{"ok":true,"runs":5,"passed":5,"failed":0,"rows":{"FS-01":"5/5"},"lanes":[{"instance":"default","passed":"5/5"}],"batch":"' + $batchDir.Replace('\', '\\') + '"}'
$r = ConvertFrom-GoldenBatchOutput @($okLine) 0
Check ($r.Exit -eq 0 -and $r.Batch -eq $batchDir) 'a passing batch gives its folder'
$r = ConvertFrom-GoldenBatchOutput @('stray host text', $okLine) 0
Check ($r.Exit -eq 0 -and $r.Batch -eq $batchDir) 'the JSON line is found among other output'
$r = ConvertFrom-GoldenBatchOutput @('{"ok":false,"exit":2,"error":"section first-session has no row(s) FS-99"}') 2
Check ($r.Exit -eq 2 -and $r.Error -like '*FS-99*') 'a usage error passes through as 2'
$r = ConvertFrom-GoldenBatchOutput @('{"ok":false,"exit":3,"error":"the lab daemon is not answering"}') 3
Check ($r.Exit -eq 3 -and $r.Error -like '*not answering*') 'a pre-flight failure passes through as 3'
$failLine = '{"ok":false,"runs":2,"passed":1,"failed":1,"failed_known":1,"failed_new":0,"rows":{},"lanes":[{"instance":"default","passed":"1/2","braked":true}],' +
    '"failures":[{"n":1,"why":"FS-P2 FAIL: clause movie-over failed","issue":"#1341","runs":["default#2"]}],"batch":"C:\\b"}'
$r = ConvertFrom-GoldenBatchOutput @($failLine) 4
Check ($r.Exit -eq 1 -and $r.Error -like '*1/2 passed*FS-P2 FAIL: clause movie-over failed (runs: default#2)*') 'a braked lane is exit 1 with its first failure'
Check ($r.Batch -eq 'C:\b') 'a failed batch still names its folder'
$r = ConvertFrom-GoldenBatchOutput @($failLine) 1
Check ($r.Exit -eq 1 -and $r.Error -like 'every run must pass*') 'a failed run is exit 1: every run must pass'
$r = ConvertFrom-GoldenBatchOutput @() 1
Check ($r.Exit -eq 3 -and $r.Error -like '*without a result*') 'a child with no JSON is a pre-flight failure, not a pass'
$r = ConvertFrom-GoldenBatchOutput @('{"ok":true,"runs":5}') 0
Check ($r.Exit -eq 3) 'a pass that names no batch folder cannot go on'

# --- lab_golden_record reply -> exit code ---
Check ((ConvertTo-GoldenExit ([pscustomobject]@{ ok = $true; path = 'p'; rows = 7 })) -eq 0) 'ok is 0'
foreach ($c in @(@('exists', 2), @('disagree', 6), @('not_passing', 1), @('cannot_record', 7), @('cannot_diff', 7), @('no_golden', 7), @('', 7))) {
    $reply = [pscustomobject]@{ ok = $false; code = $c[0]; error = 'x' }
    Check ((ConvertTo-GoldenExit $reply) -eq $c[1]) "code '$($c[0])' is exit $($c[1])"
}
Check ((ConvertTo-GoldenExit ([pscustomobject]@{ ok = $false })) -eq 7) 'a refusal without a code is 7'

# --- compact JSON ---
$okReply = [pscustomobject]@{ ok = $true; path = 'C:\Users\someone\source\projects\Cimmeria\docs\guides\uat-specs\golden\first-session.json'
    rows = 7; events = 312; bytes = 21456; runs = 5 }
$okJson = ConvertTo-GoldenCompactJson $okReply 0
Check ($okJson.Length -lt 300) "a success stays under 300 characters ($($okJson.Length))"
Check ($okJson -notmatch 'null') 'a success has no nulls'
$o = $okJson | ConvertFrom-Json
Check ($o.ok -and $o.exit -eq 0 -and $o.rows -eq 7 -and $o.events -eq 312 -and $o.bytes -eq 21456 -and $o.runs -eq 5) 'a success carries rows, events, bytes and runs'
$bad = ConvertTo-GoldenCompactJson ([pscustomobject]@{ ok = $false; code = 'disagree'; error = 'run b differs: FS-P3 event 4' }) 6
$b = $bad | ConvertFrom-Json
Check (-not $b.ok -and $b.exit -eq 6 -and $b.code -eq 'disagree' -and $b.error -like 'run b*') 'a refusal carries its code and error'
Check ($bad -notmatch 'null' -and $bad -notmatch 'path') 'a refusal has no nulls and no success fields'
$sparse = ConvertTo-GoldenCompactJson ([pscustomobject]@{ ok = $false; code = 'cannot_record'; error = $null }) 7
Check ($sparse -notmatch 'null' -and $sparse -notmatch 'error') 'a missing error is left out, not null'
foreach ($c in @(@('disagree', 6), @('not_passing', 1), @('cannot_record', 7))) {
    $j = ConvertTo-GoldenCompactJson ([pscustomobject]@{ ok = $false; code = $c[0]; error = 'x' }) $c[1] $batchDir | ConvertFrom-Json
    Check ($j.exit -eq $c[1] -and $j.batch -eq $batchDir) "a $($c[0]) refusal names the batch folder"
}
Check ((ConvertTo-GoldenCompactJson $okReply 0 $batchDir) -notmatch 'batch') 'a success names the golden path, not the batch'
Check ((ConvertTo-GoldenCompactJson ([pscustomobject]@{ ok = $false; code = 'disagree'; error = 'x' }) 6 '') -notmatch 'batch') 'no batch folder is left out, not empty'

Check ((Format-GoldenRecordLine 'first-session' $okReply) -ceq "golden first-session: 7 rows, 312 events, 21456 bytes from 5 runs; $($okReply.path)") 'the record line'

# --- show ---
$golden = @'
{"schema":1,"section":"first-session","rows":["FS-01","FS-P1"],"recorded_utc":"2026-10-11T10:00:00Z",
 "spec_sha256":"ab","build":{"server":"1a2b3c4","sgw_sha256":"cd"},
 "evidence":[{"run_id":"r1","run_dir":"d1"},{"run_id":"r2","run_dir":"d2"}],
 "fingerprint":{"schema":1,"section":"first-session","rows":[
   {"row":"FS-01","result":"PASS","tap":"full","events":["c2s a","s2c b"],"variance":{"late_bind_settle_ms":1000}},
   {"row":"FS-P1","result":"PASS","tap":"full","events":["c2s c"],"variance":{"late_bind_settle_ms":1000}}]}}
'@ | ConvertFrom-Json -Depth 50
$sum = Get-GoldenSummary $golden
Check ($sum.section -eq 'first-session' -and (@($sum.rows) -join ',') -eq 'FS-01,FS-P1') 'show names the section and rows'
Check ($sum.events -eq 3) 'show totals the ordered events over the rows'
Check ($sum.server -eq '1a2b3c4' -and $sum.sgw_sha256 -eq 'cd' -and (@($sum.runs) -join ',') -eq 'r1,r2') 'show names the build and the evidence run ids'
$noEvents = '{"section":"s","rows":["R"],"build":{"server":"abc1234"},"evidence":[],"fingerprint":{"rows":[{"row":"R","variance":{}}]}}' | ConvertFrom-Json
$s2 = Get-GoldenSummary $noEvents
Check ($s2.events -eq 0 -and -not $s2.Contains('sgw_sha256')) 'a row without events counts 0 and a missing SGW hash is left out'
Check (($s2 | ConvertTo-Json -Compress) -notmatch 'null') 'the show summary has no nulls'
$text = Format-GoldenSummary $sum
Check ($text -match 'golden first-session: 2 rows, 3 events' -and $text -match 'runs:\s+r1, r2') 'the show text'

# --- golden.ps1 before the daemon: usage, the run rule and an existing golden ---
$cmd = Join-Path $PSScriptRoot 'golden.ps1'
$pwshExe = (Get-Process -Id $PID).Path
$specs = Join-Path ([System.IO.Path]::GetTempPath()) ("golden-specs-" + [guid]::NewGuid())
try {
    New-Item -ItemType Directory -Force -Path (Join-Path $specs 'golden') | Out-Null
    $out = & $pwshExe -NoProfile -File $cmd record first-session -Runs 3 -ServerVersion $sha -SpecsDir $specs -Json
    Check ($LASTEXITCODE -eq 2 -and ($out | ConvertFrom-Json).error -like '-Runs must be 5*') "-Runs 3 exits 2 before anything runs ($LASTEXITCODE)"
    $out = & $pwshExe -NoProfile -File $cmd record first-session -SpecsDir $specs -Json
    Check ($LASTEXITCODE -eq 2 -and ($out | ConvertFrom-Json).error -like '-ServerVersion*') 'record without -ServerVersion exits 2'
    $out = & $pwshExe -NoProfile -File $cmd show first-session -SpecsDir $specs -Json
    Check ($LASTEXITCODE -eq 2 -and ($out | ConvertFrom-Json).error -like 'no golden for first-session*') 'show without a golden exits 2'
    Set-Content -LiteralPath (Join-Path $specs 'golden\first-session.json') -Value ($golden | ConvertTo-Json -Depth 50) -Encoding utf8
    $out = & $pwshExe -NoProfile -File $cmd record first-session -ServerVersion $sha -SpecsDir $specs -Json
    Check ($LASTEXITCODE -eq 2 -and ($out | ConvertFrom-Json).error -eq 'a golden exists; re-blessing needs -Rebless and five fresh runs') 'an existing golden without -Rebless exits 2'
    $out = & $pwshExe -NoProfile -File $cmd show first-session -SpecsDir $specs -Json
    $shown = $out | ConvertFrom-Json
    Check ($LASTEXITCODE -eq 0 -and $shown.ok -and $shown.events -eq 3 -and (@($shown.runs) -join ',') -eq 'r1,r2') 'show prints the committed golden as JSON'
    Check ("$out" -notmatch 'null') 'show JSON has no nulls'
    # A dotted section is read from the file the daemon writes.
    Set-Content -LiteralPath (Join-Path $specs 'golden\v1_2.json') -Value ($golden | ConvertTo-Json -Depth 50) -Encoding utf8
    $out = & $pwshExe -NoProfile -File $cmd show v1.2 -SpecsDir $specs -Json
    Check ($LASTEXITCODE -eq 0 -and ($out | ConvertFrom-Json).ok) 'show v1.2 reads golden\v1_2.json'
    $out = & $pwshExe -NoProfile -File $cmd record v1.2 -ServerVersion $sha -SpecsDir $specs -Json
    Check ($LASTEXITCODE -eq 2 -and ($out | ConvertFrom-Json).error -like 'a golden exists*') 'record v1.2 sees golden\v1_2.json exists'
    # An unreadable golden is not "a run did not pass" (1).
    Set-Content -LiteralPath (Join-Path $specs 'golden\broken.json') -Value '{ not json' -Encoding utf8
    $out = & $pwshExe -NoProfile -File $cmd show broken -SpecsDir $specs -Json
    $o = $out | ConvertFrom-Json
    Check ($LASTEXITCODE -eq 7 -and $o.exit -eq 7 -and $o.error -like 'cannot read*') "an unreadable golden exits 7 ($LASTEXITCODE)"
} finally { Remove-Item -LiteralPath $specs -Recurse -Force -ErrorAction SilentlyContinue }

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
