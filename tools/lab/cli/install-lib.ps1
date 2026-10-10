<#
.SYNOPSIS
    Install-LabCli and the user PATH helpers behind lab install and lab setup.
    Dot-source it; it defines functions only. It is self-contained (it does not
    dot-source common.ps1), so tools/lab/cli/test-install-lib.ps1 runs without the
    dispatcher.
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# The lab home: CIMMERIA_LAB_HOME when set (the tests use it), else %LOCALAPPDATA%\cimmeria-lab.
function Get-InstallLabHome {
    if ($env:CIMMERIA_LAB_HOME) { return $env:CIMMERIA_LAB_HOME }
    return (Join-Path $env:LOCALAPPDATA 'cimmeria-lab')
}

# The PATH value with $Entry appended, or $null when $Entry is already in it.
# Entries compare case-insensitively and ignore trailing slashes; the other
# entries are kept exactly as they are.
function Add-PathEntryText([string]$PathValue, [string]$Entry) {
    if (-not $PathValue) { return $Entry }
    $want = $Entry.TrimEnd('\', '/')
    foreach ($part in @($PathValue -split ';')) {
        if ($part.TrimEnd('\', '/') -ieq $want) { return $null }
    }
    return "$($PathValue.TrimEnd(';'));$Entry"
}

# Adds $Entry to the user PATH unless it is there. It goes through HKCU\Environment
# because [Environment]::SetEnvironmentVariable(..., 'User') writes REG_SZ, which
# would turn a REG_EXPAND_SZ Path into one that no longer expands %VARIABLES%.
# Returns $true when it changed the value.
function Add-UserPathEntry([string]$Entry) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
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
        return $true
    } finally {
        $key.Close()
    }
}

# Copies the lab CLI from the checkout at $Root into <LabHome>\cli, so the installed
# lab.ps1 runs without the checkout. Layout: cli\lab.ps1, cli\cli\*.ps1, and
# daemon.ps1, instances.ps1, labd-lib.ps1 beside lab.ps1, so cli\..\daemon.ps1
# resolves. Writes cli\cli\VERSION with the checkout's short commit. Refuses a
# root that has no tools\lab\lab.ps1.
function Install-LabCli([string]$Root) {
    $Root = (Resolve-Path -LiteralPath $Root).Path
    $src = Join-Path $Root 'tools\lab'
    if (-not (Test-Path -LiteralPath (Join-Path $src 'lab.ps1'))) {
        throw "$Root is not a Cimmeria checkout with the lab CLI (no tools\lab\lab.ps1)"
    }
    $dest = Join-Path (Get-InstallLabHome) 'cli'
    $destCli = Join-Path $dest 'cli'
    New-Item -ItemType Directory -Force -Path $destCli | Out-Null
    Copy-Item -LiteralPath (Join-Path $src 'lab.ps1') -Destination $dest -Force
    Get-ChildItem -LiteralPath (Join-Path $src 'cli') -Filter '*.ps1' |
        Copy-Item -Destination $destCli -Force
    foreach ($name in 'daemon.ps1', 'instances.ps1', 'labd-lib.ps1') {
        Copy-Item -LiteralPath (Join-Path $src $name) -Destination $dest -Force
    }
    $sha = (git -C $Root rev-parse --short HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or -not $sha) { throw "git rev-parse failed in $Root" }
    Set-Content -LiteralPath (Join-Path $destCli 'VERSION') -Value $sha -Encoding ascii
}
