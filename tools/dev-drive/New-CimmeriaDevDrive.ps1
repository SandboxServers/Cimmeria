#Requires -RunAsAdministrator
<#
.SYNOPSIS
  Creates a Windows Dev Drive for Cimmeria build output (target dirs and the sccache cache).

.DESCRIPTION
  A Dev Drive is a ReFS volume that Windows Defender scans in performance mode
  (asynchronously), which removes most of the per-file scanning cost of a Rust build.
  ReFS also supports block cloning: copying a file within the volume shares its blocks
  until one copy changes, so seeding a new worktree's target dir from a warm one costs
  seconds and almost no disk (tools/dev-drive/Copy-WarmTarget.ps1).

  This script creates a dynamically sized VHDX on an existing NTFS volume (default C:),
  formats it as a Dev Drive, mounts it at a drive letter, marks it trusted, and sets the
  user environment variables the build lane reads:
    CIMMERIA_TARGET_ROOT = <letter>:\targets     (per-worktree target dirs)
    CIMMERIA_SCCACHE_DIR = <letter>:\sccache     (shared compiler cache)
  Source code stays where it is. Only build output moves.

  Requirements: Windows 11 22H2 or later (block cloning in the copy engine needs 24H2),
  an elevated PowerShell, and Hyper-V's VHD cmdlets or `diskpart` (the script uses
  diskpart, which ships with Windows).

.PARAMETER VhdPath
  Where the VHDX file lives. Default C:\DevDrives\cimmeria-build.vhdx.
.PARAMETER SizeGB
  Maximum size. The file is dynamic: it only takes up the space actually used.
.PARAMETER DriveLetter
  Letter to mount at. Default B (rarely used on modern machines).
#>
param(
    [string] $VhdPath = 'C:\DevDrives\cimmeria-build.vhdx',
    [int] $SizeGB = 400,
    [char] $DriveLetter = 'B'
)
$ErrorActionPreference = 'Stop'
$build = [Environment]::OSVersion.Version.Build
if ($build -lt 22621) { throw "Dev Drive needs Windows 11 22H2 (build 22621) or later; this is build $build." }
if (Test-Path "${DriveLetter}:\") { throw "Drive ${DriveLetter}: already exists. Pick another letter with -DriveLetter." }

New-Item -ItemType Directory -Force (Split-Path $VhdPath) | Out-Null
if (-not (Test-Path $VhdPath)) {
    $script = @"
create vdisk file="$VhdPath" maximum=$($SizeGB * 1024) type=expandable
select vdisk file="$VhdPath"
attach vdisk
create partition primary
assign letter=$DriveLetter
"@
    $tmp = New-TemporaryFile
    Set-Content $tmp $script -Encoding ascii
    diskpart /s $tmp | Out-Host
    Remove-Item $tmp
    Start-Sleep -Seconds 2
    Format-Volume -DriveLetter $DriveLetter -DevDrive -NewFileSystemLabel 'CimmeriaBuild' -Confirm:$false | Out-Host
}
else {
    Write-Host "VHDX exists; attaching."
    $tmp = New-TemporaryFile
    Set-Content $tmp "select vdisk file=`"$VhdPath`"`nattach vdisk" -Encoding ascii
    diskpart /s $tmp | Out-Host
    Remove-Item $tmp
}

# Trust the volume so Defender uses performance mode on it, and confirm the filter state.
fsutil devdrv trust "${DriveLetter}:" | Out-Host
fsutil devdrv query "${DriveLetter}:" | Out-Host

# Re-attach at every logon (a VHDX does not auto-mount).
$taskName = 'Cimmeria Dev Drive attach'
$attach = "select vdisk file=`"$VhdPath`"`r`nattach vdisk"
$attachScript = Join-Path (Split-Path $VhdPath) 'attach.txt'
Set-Content $attachScript $attach -Encoding ascii
$action = New-ScheduledTaskAction -Execute 'diskpart.exe' -Argument "/s `"$attachScript`""
$trigger = New-ScheduledTaskTrigger -AtLogOn
Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -RunLevel Highest -Force | Out-Null

New-Item -ItemType Directory -Force "${DriveLetter}:\targets", "${DriveLetter}:\sccache" | Out-Null
[Environment]::SetEnvironmentVariable('CIMMERIA_TARGET_ROOT', "${DriveLetter}:\targets", 'User')
[Environment]::SetEnvironmentVariable('CIMMERIA_SCCACHE_DIR', "${DriveLetter}:\sccache", 'User')

Write-Host ""
Write-Host "Dev Drive ready at ${DriveLetter}:\ (VHDX $VhdPath, max $SizeGB GB, dynamic)."
Write-Host "Set for your user: CIMMERIA_TARGET_ROOT=${DriveLetter}:\targets, CIMMERIA_SCCACHE_DIR=${DriveLetter}:\sccache"
Write-Host "Open a new terminal (and restart Claude Code sessions) so they pick up the variables."
Write-Host "Existing target dirs are not moved; tools/build-hygiene/sweep.ps1 -RemoveLegacyTargets clears them once you're happy."
