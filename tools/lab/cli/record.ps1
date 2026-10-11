#requires -Version 7
<#
.SYNOPSIS
    Record a person playing a lab instance and draft a spec from it: lab record start|status|stop|transform.

.DESCRIPTION
    start <section> calls lab_record_start on the shared lab daemon: the
    daemon samples the instance every 500 ms while a person plays it by hand
    (it only reads), under a lease of its own, so no agent drives that client
    meanwhile. It stops by itself after -MaxMinutes (default and at most 30)
    or when its lease is taken over. The recording id and instance go to
    %LOCALAPPDATA%\cimmeria-lab\record-<instance>.json, so stop needs no id.

    status calls lab_record_status: the instance's recording, its age, ticks
    and anchors (interact, useItem, moveItem and useAbility calls seen).

    stop calls lab_record_stop with the saved id: the recording ends, its raw
    log is kept under <uat root>\records\<recording id>\record.jsonl, and a
    draft spec is written to <specs>\drafts\<section>-<yyyyMMdd-HHmm>.toml,
    never over an existing file. -CharacterExpr replaces the recorded
    character's name in the draft's SQL (Px${run_id} for a fresh character
    per run). -NoDraft keeps the log only. The state file is deleted once the
    stop succeeds; on a failure the recording stays on the daemon and the
    file stays, so run stop again (with other options if they were the
    problem). -RecordingId stops a recording this machine did not start
    (lab record status shows its id).

    transform <recording id or folder> calls lab_record_transform: the same
    draft again from a raw log already on disk, with other -Section,
    -CharacterExpr or -Profile, as <name>-2.toml, -3, ... when that draft
    exists. It needs the daemon running, but no lease and no client. The
    folder must be under <uat root>\records\, and the recording stopped.

    -SpecsDir names the spec folder the draft goes under (default: the
    checkout's docs\guides\uat-specs; away from a checkout, the daemon's).
    A recording's spec folder is fixed at start.

    -Json prints only one compact JSON object: the tool's answer, or
    {"ok":false,"exit":N,"error":"..."} as the daemon returned it (exit 4 for
    a busy instance).

    Exit codes: 0 ok; 1 the tool failed; 2 usage, or refused with nothing
    changed; 3 no daemon; 4 the instance is busy (leased, or already
    recording).

.EXAMPLE
    lab record start first-session -Instance p2
    lab record status -Instance p2
    lab record stop -Instance p2 -CharacterExpr 'Px${run_id}'
    lab record transform 20261012-143000-p2 -Section first-session -Json
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Action,
    [Parameter(Position = 1)]
    [string]$Target,
    [string]$Instance,
    [string]$Character,
    # A string, checked by Test-RecordArguments, so `-MaxMinutes abc` exits 2
    # with the command's own message (and one JSON line under -Json).
    [string]$MaxMinutes,
    [string]$CharacterExpr,
    [string]$Profile,
    [string]$Section,
    [string]$SpecsDir,
    [string]$RecordingId,
    [switch]$NoDraft,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'record-lib.ps1')

function Invoke-LabRecord {
    $opt = @{
        Action = $Action; Target = $Target; Instance = $Instance; Character = $Character
        MaxMinutes = $MaxMinutes; CharacterExpr = $CharacterExpr; Profile = $Profile
        Section = $Section; SpecsDir = $SpecsDir; RecordingId = $RecordingId; NoDraft = $NoDraft.IsPresent
    }
    $url = Get-LabMcpUrl
    $token = Get-LabToken
    $ctx = @{
        LabHome = Get-LabHome
        UatRoot = Get-UatRoot
        SpecsDir = Get-DefaultSpecsDir
        Probe = { [bool](Get-LabToken) -and $null -ne (Get-LabStatus) }
        # One MCP session per call: a command makes one call. $url and
        # $token resolve here, in this function's scope, when it runs (not
        # GetNewClosure: its module scope cannot see this script's functions).
        Call = {
            param($Name, $Arguments, $TimeoutSec)
            $s = New-McpSession $url $token
            try { Invoke-McpTool $s $Name $Arguments $TimeoutSec } finally { Close-McpSession $s }
        }
    }
    $out = Invoke-RecordAction $opt $ctx
    # Straight to the streams: a bare expression would join the return value.
    if ($Json) {
        [Console]::Out.WriteLine((Hide-LeaseIds (ConvertTo-RecordCompactJson $out.Body)))
    } elseif ($out.Exit -eq 0) {
        [Console]::Out.WriteLine((Hide-LeaseIds $out.Text))
    } else {
        [Console]::Error.WriteLine((Hide-LeaseIds $out.Text))
    }
    return [int]$out.Exit
}

if ($MyInvocation.InvocationName -ne '.') {
    # Anything unexpected still keeps the contract: exit 1, and under -Json
    # one {ok:false,exit:1,error} line. (An unknown switch fails earlier, at
    # parameter binding, with PowerShell's own message and exit.)
    $code = 1
    try {
        $code = Invoke-LabRecord
    } catch {
        $b = New-RecordFailure 1 "lab record: $($_.Exception.Message)"
        if ($Json) {
            [Console]::Out.WriteLine((ConvertTo-RecordCompactJson $b))
        } else {
            [Console]::Error.WriteLine($b.error)
        }
    }
    exit $code
}
