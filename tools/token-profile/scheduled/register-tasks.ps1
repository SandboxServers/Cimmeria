<#
.SYNOPSIS
  Registers, updates or removes the token profiler's two Task Scheduler tasks for the current user.

.DESCRIPTION
  \Cimmeria\TokenProfile-Weekly   Mondays at 08:00 local time   weekly.ps1
  \Cimmeria\TokenProfile-PrSweep  daily at 07:30 local time     pr-sweep.ps1

  Both run as the current user, only while that user is logged on (no stored password),
  without elevation, and start late if the machine was off or asleep at the set time. A run
  that is still going when the next is due makes the next one wait its turn rather than start
  a second copy; jobs.py's lock covers a manual run at the same time. Re-running this script
  updates the tasks in place.

  The tasks run the scripts from the main checkout, never a worktree (a worktree is deleted
  when its PR merges), so run this after the scripts are on main. Python, gh and the
  TOKEN_PROFILE_* variables come from the user's environment, as in an interactive shell.

.PARAMETER Unregister
  Remove both tasks instead.

.EXAMPLE
  pwsh tools/token-profile/scheduled/register-tasks.ps1 -WhatIf
  pwsh tools/token-profile/scheduled/register-tasks.ps1
  pwsh tools/token-profile/scheduled/register-tasks.ps1 -Unregister
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [switch]$Unregister
)
$ErrorActionPreference = 'Stop'

$taskPath = '\Cimmeria\'
$tasks = @(
    @{ Name = 'TokenProfile-Weekly'; Script = 'weekly.ps1'; Schedule = 'Mondays 08:00';
       Trigger = { New-ScheduledTaskTrigger -Weekly -DaysOfWeek Monday -At '08:00' } },
    @{ Name = 'TokenProfile-PrSweep'; Script = 'pr-sweep.ps1'; Schedule = 'daily 07:30';
       Trigger = { New-ScheduledTaskTrigger -Daily -At '07:30' } }
)

if ($Unregister) {
    foreach ($t in $tasks) {
        $existing = Get-ScheduledTask -TaskPath $taskPath -TaskName $t.Name -ErrorAction SilentlyContinue
        if (-not $existing) {
            Write-Host "$taskPath$($t.Name): not registered"
        } elseif ($PSCmdlet.ShouldProcess("$taskPath$($t.Name)", 'Unregister scheduled task')) {
            Unregister-ScheduledTask -TaskPath $taskPath -TaskName $t.Name -Confirm:$false
            Write-Host "$taskPath$($t.Name): removed"
        }
    }
    exit 0
}

# The main checkout: the parent of the repository's common git dir, which a worktree shares.
$common = git -C $PSScriptRoot rev-parse --path-format=absolute --git-common-dir
if ($LASTEXITCODE -ne 0 -or -not $common) { throw "not in a git checkout: $PSScriptRoot" }
$main = Split-Path -Parent $common
$scripts = Join-Path $main 'tools\token-profile\scheduled'
foreach ($t in $tasks) {
    if (-not (Test-Path (Join-Path $scripts $t.Script))) {
        throw "$($t.Script) is not in the main checkout ($scripts); merge it to main and pull first"
    }
}

$shell = (Get-Command pwsh -ErrorAction SilentlyContinue).Source
if (-not $shell) { $shell = (Get-Command powershell).Source }
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
    -MultipleInstances Queue -ExecutionTimeLimit (New-TimeSpan -Hours 4)

foreach ($t in $tasks) {
    $script = Join-Path $scripts $t.Script
    $arguments = "-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$script`""
    $action = New-ScheduledTaskAction -Execute $shell -Argument $arguments -WorkingDirectory $main
    $description = "Cimmeria token profiler ($($t.Script)). Registered by tools/token-profile/scheduled/register-tasks.ps1."
    if ($PSCmdlet.ShouldProcess("$taskPath$($t.Name) ($($t.Schedule), as $user): $shell $arguments",
                                'Register scheduled task')) {
        $trigger = & $t.Trigger
        # New-ScheduledTaskTrigger writes the start as UTC ("...Z"), which pins the run to UTC and
        # moves it an hour at each daylight-saving change. Without an offset it follows local time.
        $trigger.StartBoundary = ([datetime]$trigger.StartBoundary).ToLocalTime().ToString('s')
        Register-ScheduledTask -TaskPath $taskPath -TaskName $t.Name -Action $action -Trigger $trigger `
            -Principal $principal -Settings $settings -Description $description -Force | Out-Null
        Write-Host "$taskPath$($t.Name): registered, $($t.Schedule) local time"
    }
}
