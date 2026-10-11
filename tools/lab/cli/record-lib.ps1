<#
.SYNOPSIS
    Library for `lab record` (lab-record LR-08): argument checks, the state
    file, the compact JSON, the exit codes and the four actions. Dot-source
    it; it defines functions only.

.DESCRIPTION
    The actions call the daemon's record tools through an injected
    scriptblock (& $Call <tool> <arguments> <timeout seconds>), which returns
    the tool's answer parsed, or throws: "<tool> failed: <text>" for a tool
    error (uat-lib.ps1's Invoke-McpTool), anything else when the daemon did
    not answer. So tools/lab/cli/test-record.ps1 runs every action with the
    calls stubbed and no daemon.

    A tool refusal or failure is D-LR16's {"ok":false,"exit":N,"error":"..."};
    it is printed as it comes back, except that a refusal because the
    instance is busy (leased, already recording) says exit 4, as the command
    exits.

    Needs uat-lib.ps1 (Hide-LeaseIds) dot-sourced first. Never print the
    token or a lease id from here.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:RecordBodyCap = 400
$script:RecordMaxMinutes = 30
# The tool's shapes (server/record/checks.rs), checked here first so a typo
# exits 2 without a daemon.
$script:RecordSectionPattern = '^[a-z0-9][a-z0-9-]{0,47}$'
$script:RecordCharacterExprPattern = '^[A-Za-z0-9 _${}]{1,64}$'
$script:RecordInstancePattern = '^[A-Za-z0-9_-]{1,32}$'
# A tool refusal (exit 2) with one of these is "the instance is busy" (exit 4).
$script:RecordBusyPattern = 'busy|leased|already running|is starting|still finishing'
$script:RecordUsage = @'
usage: lab record start <section> [-Instance p2] [-Character <name>] [-MaxMinutes 30] [-SpecsDir <dir>] [-Json]
       lab record status [-Instance p2] [-Json]
       lab record stop [-Instance p2] [-CharacterExpr 'Px${run_id}'] [-Profile <name>] [-NoDraft] [-RecordingId <id>] [-Json]
       lab record transform <recording id or folder> [-Section <id>] [-CharacterExpr ...] [-Profile ...] [-SpecsDir <dir>] [-Json]
'@

# The argument checks that need no daemon: an error message, or $null.
# $Opt holds the command's parameters (Action, Target, Instance, MaxMinutes,
# CharacterExpr, Section, ...); a key that is absent or empty was not given.
function Test-RecordArguments([hashtable]$Opt) {
    $action = [string]$Opt['Action']
    if (@('start', 'status', 'stop', 'transform') -cnotcontains $action) { return $script:RecordUsage }
    $target = [string]$Opt['Target']
    if ($action -eq 'start') {
        if (-not $target) { return "start needs a section: lab record start <section>`n$script:RecordUsage" }
        if ($target -cnotmatch $script:RecordSectionPattern) { return "section: give a section id, ^[a-z0-9][a-z0-9-]{0,47}$ (first-session); got '$target'" }
    } elseif ($action -eq 'transform') {
        if (-not $target) { return "transform needs a recording: lab record transform <recording id or folder>`n$script:RecordUsage" }
    } elseif ($target) {
        return "$action takes no positional argument; got '$target'"
    }
    $instance = [string]$Opt['Instance']
    if ($instance -and $instance -cnotmatch $script:RecordInstancePattern) { return "-Instance: give an instance name like p2 ([A-Za-z0-9_-]); got '$instance'" }
    if ($null -ne $Opt['MaxMinutes']) {
        $m = [int]$Opt['MaxMinutes']
        if ($m -lt 1 -or $m -gt $script:RecordMaxMinutes) { return "-MaxMinutes must be 1 to $script:RecordMaxMinutes" }
    }
    $expr = [string]$Opt['CharacterExpr']
    if ($expr -and $expr -cnotmatch $script:RecordCharacterExprPattern) { return '-CharacterExpr: give ^[A-Za-z0-9 _${}]{1,64}$ (Px${run_id})' }
    $section = [string]$Opt['Section']
    if ($section -and $section -cnotmatch $script:RecordSectionPattern) { return "-Section: give a section id, ^[a-z0-9][a-z0-9-]{0,47}$; got '$section'" }
    $only = @{ start = @('Character', 'MaxMinutes'); stop = @('NoDraft', 'RecordingId'); transform = @('Section') }
    foreach ($k in $only.Keys) {
        foreach ($p in $only[$k]) {
            if ($action -ne $k -and $Opt[$p]) { return "-$p is for lab record $k" }
        }
    }
    if ($action -in @('start', 'status') -and ($Opt['CharacterExpr'] -or $Opt['Profile'])) { return "-CharacterExpr and -Profile are for stop and transform" }
    if ($action -in @('status', 'stop') -and $Opt['SpecsDir']) { return '-SpecsDir is for start and transform' }
    if ($action -eq 'transform' -and $instance) { return 'transform runs on no instance; drop -Instance' }
    return $null
}

# %LOCALAPPDATA%\cimmeria-lab\record-<instance>.json (the lab home, so the
# tests' CIMMERIA_LAB_HOME moves it). The name is checked first: it becomes
# part of a path.
function Get-RecordStatePath([string]$LabHome, [string]$Instance) {
    if ($Instance -cnotmatch $script:RecordInstancePattern) { throw "bad instance name '$Instance'" }
    return Join-Path $LabHome "record-$Instance.json"
}

function Save-RecordState([string]$Path, [string]$RecordingId, [string]$Instance) {
    $dir = Split-Path -Parent $Path
    if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    $o = [ordered]@{ recording_id = $RecordingId; instance = $Instance }
    Set-Content -LiteralPath $Path -Value ($o | ConvertTo-Json -Compress) -Encoding utf8NoBOM
}

# The state as an object (recording_id, instance), or $null when there is
# none or it cannot be read.
function Read-RecordState([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    try { $s = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json } catch { return $null }
    if ($s -isnot [pscustomobject] -or $s.PSObject.Properties.Name -notcontains 'recording_id' -or -not $s.recording_id) { return $null }
    return $s
}

# One compact JSON line, null and empty top-level values dropped (the tools
# already drop theirs). A failure ({ok:false, exit, error}) has its error cut until
# the line is under 400 characters (JSON escaping counts).
function ConvertTo-RecordCompactJson($Object) {
    $o = [ordered]@{}
    $props = if ($Object -is [System.Collections.IDictionary]) { $Object.Keys | ForEach-Object { @{ Name = $_; Value = $Object[$_] } } }
             else { $Object.PSObject.Properties | ForEach-Object { @{ Name = $_.Name; Value = $_.Value } } }
    foreach ($p in @($props)) {
        if ($null -eq $p.Value -or ($p.Value -is [string] -and $p.Value -eq '')) { continue }
        $o[$p.Name] = $p.Value
    }
    $line = $o | ConvertTo-Json -Depth 5 -Compress
    if ($o.Contains('error') -and $o['error'] -is [string]) {
        $err = [string]$o['error']
        $n = $err.Length
        while ($line.Length -ge $script:RecordBodyCap -and $n -gt 0) {
            $n = [math]::Max(0, $n - [math]::Max(1, $line.Length + 1 - $script:RecordBodyCap))
            $o['error'] = $err.Substring(0, $n)
            $line = $o | ConvertTo-Json -Depth 5 -Compress
        }
    }
    return $line
}

# The failure object the command prints.
function New-RecordFailure([int]$Exit, [string]$Message) {
    return [ordered]@{ ok = $false; exit = $Exit; error = (Hide-LeaseIds $Message) }
}

# A thrown call error as @{ Exit; Body }: a tool error whose text is the
# D-LR16 shape keeps its body, with exit 2 for busy turned into 4; any other
# tool error is exit 1 (4 when it says leased); anything else is the daemon
# not answering (3).
function ConvertFrom-RecordCallError([string]$Tool, [string]$Message) {
    $Message = Hide-LeaseIds $Message
    $prefix = "$Tool failed: "
    if (-not $Message.StartsWith($prefix)) {
        $first = ($Message -split "`n")[0]
        if ($first.StartsWith("$Tool returned")) { return @{ Exit = 1; Body = New-RecordFailure 1 $first } }
        return @{ Exit = 3; Body = New-RecordFailure 3 "the lab daemon did not answer: $first" }
    }
    $text = $Message.Substring($prefix.Length)
    $body = $null
    try { $body = $text | ConvertFrom-Json -AsHashtable } catch { }
    if ($body -is [hashtable] -and $body.ContainsKey('exit') -and $body.ContainsKey('error') -and $body['ok'] -eq $false) {
        $exit = [int]$body['exit']
        if ($exit -eq 2 -and [string]$body['error'] -match $script:RecordBusyPattern) { $exit = 4 }
        $o = [ordered]@{ ok = $false; exit = $exit; error = [string]$body['error'] }
        return @{ Exit = $exit; Body = $o }
    }
    $exit = if ($text -match $script:RecordBusyPattern) { 4 } else { 1 }
    return @{ Exit = $exit; Body = New-RecordFailure $exit $text }
}

# One tool call through $Call: @{ Exit = 0; Reply } or @{ Exit; Body }.
function Invoke-RecordCall([scriptblock]$Call, [string]$Tool, [hashtable]$Arguments, [int]$TimeoutSec) {
    try {
        $reply = & $Call $Tool $Arguments $TimeoutSec
        return @{ Exit = 0; Reply = $reply }
    } catch {
        return ConvertFrom-RecordCallError $Tool $_.Exception.Message
    }
}

function Get-RecordField($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    if ($Object -is [System.Collections.IDictionary]) { return $Object[$Name] }
    if ($Object.PSObject.Properties.Name -contains $Name) { return $Object.$Name }
    return $null
}

# Runs one action. $Ctx: LabHome, UatRoot (for the raw log path), SpecsDir
# (the default spec folder, or $null), Call (the tool scriptblock), Probe
# (a scriptblock: is the daemon answering?). Returns @{ Exit; Body; Text }:
# Body is what -Json prints, Text the line for a person.
function Invoke-RecordAction([hashtable]$Opt, [hashtable]$Ctx) {
    $fail = { param($code, $msg) $b = New-RecordFailure $code $msg; @{ Exit = $code; Body = $b; Text = $b.error } }
    $bad = Test-RecordArguments $Opt
    if ($bad) { return & $fail 2 $bad }
    $action = [string]$Opt['Action']
    $instance = [string]$Opt['Instance']
    $specs = if ($Opt['SpecsDir']) { [string]$Opt['SpecsDir'] } else { $Ctx.SpecsDir }

    # stop's state file is read before the daemon is asked: without it there is nothing to stop.
    $state = $null
    $statePath = $null
    if ($action -eq 'stop') {
        $statePath = Get-RecordStatePath $Ctx.LabHome $(if ($instance) { $instance } else { 'default' })
        $state = Read-RecordState $statePath
        if (-not $state -and -not $Opt['RecordingId']) {
            return & $fail 2 "no recording started here for instance $(if ($instance) { $instance } else { 'default' }) ($statePath); lab record status shows one, -RecordingId <id> stops it"
        }
    }
    if (-not (& $Ctx.Probe)) {
        $msg = if ($action -eq 'transform') { 'the transform runs in the lab daemon; start it with `lab start`' }
               else { 'the lab daemon is not answering (lab status; lab start)' }
        return & $fail 3 $msg
    }

    $a = @{}
    if ($instance) { $a.instance = $instance }
    switch ($action) {
        'start' {
            $a.section = [string]$Opt['Target']
            if ($Opt['Character']) { $a.character = [string]$Opt['Character'] }
            if ($null -ne $Opt['MaxMinutes']) { $a.max_minutes = [int]$Opt['MaxMinutes'] }
            if ($specs) { $a.specs_dir = $specs }
            $r = Invoke-RecordCall $Ctx.Call 'lab_record_start' $a 120
            if ($r.Exit) { return @{ Exit = $r.Exit; Body = $r.Body; Text = $r.Body.error } }
            $id = [string](Get-RecordField $r.Reply 'recording_id')
            $on = [string](Get-RecordField $r.Reply 'instance')
            if (-not $on) { $on = if ($instance) { $instance } else { 'default' } }
            if ($on -cnotmatch $script:RecordInstancePattern -or -not $id) {
                return & $fail 1 "lab_record_start answered without a usable recording_id or instance; lab record status -Instance $on shows it"
            }
            Save-RecordState (Get-RecordStatePath $Ctx.LabHome $on) $id $on
            $stopHint = if ($on -eq 'default') { 'lab record stop' } else { "lab record stop -Instance $on" }
            return @{ Exit = 0; Body = $r.Reply; Text = "recording $id on $on (section $($a.section)); play, then: $stopHint" }
        }
        'status' {
            $r = Invoke-RecordCall $Ctx.Call 'lab_record_status' $a 30
            if ($r.Exit) { return @{ Exit = $r.Exit; Body = $r.Body; Text = $r.Body.error } }
            $v = $r.Reply
            $on = [string](Get-RecordField $v 'instance')
            $text = if (-not (Get-RecordField $v 'recording_id')) { "no recording on $on" }
                    else {
                        $what = if ([bool](Get-RecordField $v 'recording')) { 'recording' } else { 'stopped by itself, waiting for lab record stop:' }
                        "$what $(Get-RecordField $v 'recording_id') on $on (section $(Get-RecordField $v 'section')): $(Get-RecordField $v 'age_s') s, $(Get-RecordField $v 'ticks') ticks, $(Get-RecordField $v 'anchors') anchors"
                    }
            return @{ Exit = 0; Body = $v; Text = $text }
        }
        'stop' {
            $a.recording_id = if ($Opt['RecordingId']) { [string]$Opt['RecordingId'] } else { [string]$state.recording_id }
            if ($state -and -not $instance) { $a.instance = [string]$state.instance }
            if ($Opt['CharacterExpr']) { $a.character_expr = [string]$Opt['CharacterExpr'] }
            if ($Opt['Profile']) { $a.profile = [string]$Opt['Profile'] }
            if ($Opt['NoDraft']) { $a.no_draft = $true }
            $r = Invoke-RecordCall $Ctx.Call 'lab_record_stop' $a 180
            if ($r.Exit) {
                # The recording stays on a failure (LR-07): keep the file so a retry needs no id.
                $text = $r.Body.error
                if ($r.Exit -eq 1 -and $state) { $text += "`nthe recording is kept; retry with lab record stop$(if ($instance) { " -Instance $instance" })" }
                return @{ Exit = $r.Exit; Body = $r.Body; Text = $text }
            }
            if ($statePath -and (Test-Path -LiteralPath $statePath)) { Remove-Item -LiteralPath $statePath -Force }
            return @{ Exit = 0; Body = $r.Reply; Text = (Format-RecordDraftLine $r.Reply $specs $Ctx.UatRoot) }
        }
        'transform' {
            $dir = [string]$Opt['Target']
            # A folder given relative to here is sent in full; a bare id as it is.
            if ((Test-Path -LiteralPath $dir -PathType Container) -and -not [IO.Path]::IsPathRooted($dir)) {
                $dir = (Resolve-Path -LiteralPath $dir).Path
            }
            $a.dir = $dir
            if ($Opt['Section']) { $a.section = [string]$Opt['Section'] }
            if ($Opt['CharacterExpr']) { $a.character_expr = [string]$Opt['CharacterExpr'] }
            if ($Opt['Profile']) { $a.profile = [string]$Opt['Profile'] }
            if ($specs) { $a.specs_dir = $specs }
            $r = Invoke-RecordCall $Ctx.Call 'lab_record_transform' $a 120
            if ($r.Exit) { return @{ Exit = $r.Exit; Body = $r.Body; Text = $r.Body.error } }
            return @{ Exit = 0; Body = $r.Reply; Text = (Format-RecordDraftLine $r.Reply $specs $Ctx.UatRoot) }
        }
    }
}

# A stop or transform answer for a person: the draft's and raw log's paths
# (the tools answer with names only, LR-07) and the counts.
function Format-RecordDraftLine($Reply, [string]$SpecsDir, [string]$UatRoot) {
    $raw = [string](Get-RecordField $Reply 'raw')
    $rawPath = if ($UatRoot -and $raw) { Join-Path (Join-Path (Join-Path $UatRoot 'records') $raw) 'record.jsonl' } else { $raw }
    $draft = [string](Get-RecordField $Reply 'draft')
    $lines = @()
    if ($draft) {
        $draftPath = if ($SpecsDir) { Join-Path (Join-Path $SpecsDir 'drafts') $draft } else { "<specs>/drafts/$draft" }
        $lines += "draft $draftPath"
        $lines += "  $(Get-RecordField $Reply 'rows') row(s), $(Get-RecordField $Reply 'steps') step(s), $(Get-RecordField $Reply 'todo') TODO(s), $(Get-RecordField $Reply 'dropped') dropped"
    } else {
        $lines += "stopped; $(Get-RecordField $Reply 'ticks') tick(s), no draft"
    }
    $lines += "raw   $rawPath"
    $ended = Get-RecordField $Reply 'ended'
    if ($ended) { $lines += "the recorder stopped early: $ended" }
    return (Hide-LeaseIds ($lines -join "`n"))
}
