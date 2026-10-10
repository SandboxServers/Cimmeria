<#
.SYNOPSIS
    Install-LabCli and the user PATH helpers behind lab install and lab setup.

.DESCRIPTION
    Dot-source it; it defines functions only. It is self-contained (it does not
    dot-source common.ps1), so tools/lab/cli/test-install-lib.ps1 runs without the
    dispatcher. The dispatcher skips *-lib.ps1, so it is not a command.

.EXAMPLE
    . tools/lab/cli/install-lib.ps1; Install-LabCli C:\src\Cimmeria
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# The lab home: CIMMERIA_LAB_HOME when set (the tests use it), else %LOCALAPPDATA%\cimmeria-lab.
function Get-InstallLabHome {
    if ($env:CIMMERIA_LAB_HOME) { return $env:CIMMERIA_LAB_HOME }
    return (Join-Path $env:LOCALAPPDATA 'cimmeria-lab')
}

# The PATH value with $Entry appended, or $null when $Entry is already in it.
# Entries compare case-insensitively, with %VARIABLES% expanded, quotes removed
# and trailing slashes ignored; the other entries are kept exactly as they are,
# less any empty ones.
function Add-PathEntryText([string]$PathValue, [string]$Entry) {
    $norm = { param($p) [Environment]::ExpandEnvironmentVariables($p.Trim().Trim('"')).TrimEnd('\', '/') }
    $want = & $norm $Entry
    $parts = @($PathValue -split ';' | Where-Object { $_.Trim() })
    foreach ($part in $parts) {
        if ((& $norm $part) -ieq $want) { return $null }
    }
    return (@($parts) + $Entry) -join ';'
}

# The user PATH as stored, %VARIABLES% unexpanded; '' when it is not set. Read-only.
function Get-UserPathRaw {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $false)
    if ($null -eq $key) { return '' }
    try {
        return [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    } finally {
        $key.Close()
    }
}

# Adds $Entry to the user PATH unless it is there. It goes through HKCU\Environment
# because [Environment]::SetEnvironmentVariable(..., 'User') writes REG_SZ, which
# would turn a REG_EXPAND_SZ Path into one that no longer expands %VARIABLES%.
# Returns $true when it changed the value.
function Add-UserPathEntry([string]$Entry) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    if ($null -eq $key) { throw 'cannot open HKCU\Environment for writing' }
    try {
        $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
        $raw = ''
        if ($key.GetValueNames() -contains 'Path') {
            $kind = $key.GetValueKind('Path')
            $raw = [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        }
        $new = Add-PathEntryText $raw $Entry
        if ($null -eq $new) { return $false }
        $key.SetValue('Path', $new, $kind)
        Send-EnvironmentChanged
        return $true
    } finally {
        $key.Close()
    }
}

# Broadcasts WM_SETTINGCHANGE("Environment"), as SetEnvironmentVariable(..., 'User')
# does, so Explorer and the terminals it starts pick up a registry PATH change
# without a sign-out.
function Send-EnvironmentChanged {
    if (-not ('CimmeriaLab.EnvBroadcast' -as [type])) {
        Add-Type -Namespace CimmeriaLab -Name EnvBroadcast -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true, CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
public static extern System.IntPtr SendMessageTimeout(System.IntPtr hWnd, uint Msg, System.UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out System.UIntPtr lpdwResult);
'@
    }
    $result = [UIntPtr]::Zero
    # HWND_BROADCAST, WM_SETTINGCHANGE, SMTO_ABORTIFHUNG, 5 s.
    [void][CimmeriaLab.EnvBroadcast]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result)
}

# The checkout lab setup installs from: $From when given, else the checkout this
# script sits in ($ScriptRoot\..\..\..), which must have .git (a folder, or a file in
# a worktree) and tools\lab\lab.ps1. The installed copy is not a checkout, so
# setup run from it needs -From.
function Resolve-SetupRoot([string]$From, [string]$ScriptRoot) {
    $hint = 'lab setup must run from a checkout: pwsh <checkout>\tools\lab\cli\setup.ps1, or lab setup -From <checkout>'
    if ($From) {
        $root = (Resolve-Path -LiteralPath $From).Path
    } else {
        $root = [System.IO.Path]::GetFullPath((Join-Path $ScriptRoot '..\..\..'))
        if (-not (Test-Path -LiteralPath (Join-Path $root '.git'))) { throw $hint }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $root 'tools\lab\lab.ps1'))) {
        throw "$root has no tools\lab\lab.ps1. $hint"
    }
    return $root
}

# Copies the lab CLI from the checkout at $Root into <LabHome>\cli, so the installed
# lab.ps1 runs without the checkout. Layout: cli\lab.ps1, cli\cli\*.ps1, and
# daemon.ps1, instances.ps1, labd-lib.ps1 beside lab.ps1, so cli\..\daemon.ps1
# resolves. The test-*.ps1 files are not copied, and command files no longer in
# the checkout are removed, so lab help never lists a stale command. Writes
# cli\cli\VERSION with the checkout's short commit, last, so a copy that fails part
# way leaves no VERSION. Refuses a root that has no tools\lab\lab.ps1.
function Install-LabCli([string]$Root) {
    $Root = (Resolve-Path -LiteralPath $Root).Path
    $src = Join-Path $Root 'tools\lab'
    if (-not (Test-Path -LiteralPath (Join-Path $src 'lab.ps1'))) {
        throw "$Root is not a Cimmeria checkout with the lab CLI (no tools\lab\lab.ps1)"
    }
    $out = git -C $Root rev-parse --short HEAD 2>$null
    if ($LASTEXITCODE -ne 0 -or -not "$out".Trim()) { throw "git rev-parse failed in $Root" }
    $sha = "$out".Trim()

    $dest = Join-Path (Get-InstallLabHome) 'cli'
    $destCli = Join-Path $dest 'cli'
    New-Item -ItemType Directory -Force -Path $destCli | Out-Null
    Remove-Item -LiteralPath (Join-Path $destCli 'VERSION') -Force -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $destCli -Filter '*.ps1' | Remove-Item -Force
    Copy-Item -LiteralPath (Join-Path $src 'lab.ps1') -Destination $dest -Force
    Get-ChildItem -LiteralPath (Join-Path $src 'cli') -Filter '*.ps1' |
        Where-Object { $_.Name -notlike 'test-*' } |
        Copy-Item -Destination $destCli -Force
    foreach ($name in 'daemon.ps1', 'instances.ps1', 'labd-lib.ps1') {
        Copy-Item -LiteralPath (Join-Path $src $name) -Destination $dest -Force
    }
    Set-Content -LiteralPath (Join-Path $destCli 'VERSION') -Value $sha -Encoding ascii
}
