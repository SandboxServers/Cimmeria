<#
.SYNOPSIS
    Tests for tools/lab/cli/install-lib.ps1 (no Pester needed).

.DESCRIPTION
    Exits non-zero when any check fails. It installs into a temp
    CIMMERIA_LAB_HOME, never into %LOCALAPPDATA%, and it never touches the user
    PATH.

.EXAMPLE
    pwsh -NoProfile -File tools/lab/cli/test-install-lib.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lab-install-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
$savedHome = $env:CIMMERIA_LAB_HOME
try {
    # --- PATH merge (pure) ---
    $bin = 'C:\Users\Test\AppData\Local\cimmeria-lab\bin'
    Check ((Add-PathEntryText 'C:\a;C:\b' $bin) -eq "C:\a;C:\b;$bin") 'an entry is appended after the existing ones'
    Check ((Add-PathEntryText '' $bin) -eq $bin) 'an empty PATH takes the entry alone'
    Check ((Add-PathEntryText 'C:\a;' $bin) -eq "C:\a;$bin") 'a trailing ; does not leave an empty entry'
    Check ($null -eq (Add-PathEntryText "C:\a;$bin;C:\b" $bin)) 'a present entry is not added twice'
    Check ($null -eq (Add-PathEntryText ($bin.ToUpperInvariant()) $bin)) 'the compare ignores case'
    Check ($null -eq (Add-PathEntryText "C:\a;$bin\" $bin)) 'a stored trailing slash still matches'
    Check ($null -eq (Add-PathEntryText "C:\a;$bin" "$bin\")) 'a trailing slash on the entry still matches'
    Check ((Add-PathEntryText 'C:\a;C:\lab\binx' 'C:\lab\bin') -eq 'C:\a;C:\lab\binx;C:\lab\bin') 'a prefix of an entry is not a match'
    Check ($null -eq (Add-PathEntryText '%LOCALAPPDATA%\cimmeria-lab\bin' (Join-Path $env:LOCALAPPDATA 'cimmeria-lab\bin'))) 'an unexpanded %VAR% entry still matches'
    Check ($null -eq (Add-PathEntryText 'C:\a;"C:\lab\bin"' 'C:\lab\bin')) 'a quoted entry still matches'
    Check ((Add-PathEntryText ';' $bin) -eq $bin) 'a lone ; leaves no empty entry'
    Check ((Add-PathEntryText '%SystemRoot%;C:\a' $bin) -eq "%SystemRoot%;C:\a;$bin") 'other %VAR% entries are kept unexpanded'

    # --- Install-LabCli: a throwaway checkout with the real CLI files ---
    $env:CIMMERIA_LAB_HOME = Join-Path $tmp 'home'
    $root = Join-Path $tmp 'checkout'
    $labDir = Join-Path $root 'tools\lab'
    $repoLab = Split-Path $PSScriptRoot -Parent
    New-Item -ItemType Directory -Force (Join-Path $labDir 'cli') | Out-Null
    # lab.ps1 is LC-02's file, so the test uses a stub.
    Set-Content -LiteralPath (Join-Path $labDir 'lab.ps1') -Value '# stub v1' -Encoding ascii
    Copy-Item -Path (Join-Path $PSScriptRoot '*.ps1') -Destination (Join-Path $labDir 'cli')
    foreach ($name in 'daemon.ps1', 'instances.ps1', 'labd-lib.ps1') {
        Copy-Item -LiteralPath (Join-Path $repoLab $name) -Destination $labDir
    }
    git -C $root init -q
    git -C $root add -A
    git -C $root -c user.name=test -c user.email=test@example.invalid -c commit.gpgsign=false commit -q -m 'stub checkout'
    $sha = (git -C $root rev-parse --short HEAD).Trim()

    $cli = Join-Path $env:CIMMERIA_LAB_HOME 'cli'
    Install-LabCli $root
    Check (Test-Path (Join-Path $cli 'lab.ps1')) 'lab.ps1 lands in cli\'
    Check (Test-Path (Join-Path $cli 'daemon.ps1')) 'daemon.ps1 lands in cli\'
    Check (Test-Path (Join-Path $cli 'instances.ps1')) 'instances.ps1 lands in cli\'
    Check (Test-Path (Join-Path $cli 'labd-lib.ps1')) 'labd-lib.ps1 lands in cli\'
    Check (Test-Path (Join-Path $cli 'cli\install-lib.ps1')) 'the cli\*.ps1 files land in cli\cli\'
    Check ((Get-Content -Raw (Join-Path $cli 'cli\VERSION')).Trim() -eq $sha) 'VERSION holds the checkout sha'
    Check ((Get-Content -Raw (Join-Path $cli 'lab.ps1')).Trim() -eq '# stub v1') 'the first run copies lab.ps1'

    Check (-not (Test-Path (Join-Path $cli 'cli\test-install-lib.ps1'))) 'test-*.ps1 files are not installed'

    # A second run overwrites, and drops a command the checkout no longer has.
    Set-Content -LiteralPath (Join-Path $cli 'cli\gone.ps1') -Value '# stale' -Encoding ascii
    Set-Content -LiteralPath (Join-Path $labDir 'lab.ps1') -Value '# stub v2' -Encoding ascii
    Install-LabCli $root
    Check ((Get-Content -Raw (Join-Path $cli 'lab.ps1')).Trim() -eq '# stub v2') 'a second run overwrites the installed files'
    Check (-not (Test-Path (Join-Path $cli 'cli\gone.ps1'))) 'a second run removes a command no longer in the checkout'
    Check (Test-Path (Join-Path $cli 'cli\install-lib.ps1')) 'a second run keeps the current commands'

    # Resolve-SetupRoot: the checkout the script sits in, -From, or a refusal.
    $scriptRoot = Join-Path $labDir 'cli'
    Check ((Resolve-SetupRoot '' $scriptRoot) -eq $root) 'setup from a checkout installs from that checkout'
    $installedRoot = Join-Path $cli 'cli'
    $hinted = $false
    try { Resolve-SetupRoot '' $installedRoot } catch { $hinted = $_.Exception.Message -like '*-From*' }
    Check $hinted 'setup from the installed copy without -From is refused, naming -From'
    Check ((Resolve-SetupRoot $root $installedRoot) -eq $root) 'setup from the installed copy with -From uses -From'

    # A root without the CLI is refused.
    $bare = Join-Path $tmp 'bare'
    New-Item -ItemType Directory $bare | Out-Null
    $refused = $false
    try { Install-LabCli $bare } catch { $refused = $_.Exception.Message -like '*lab.ps1*' }
    Check $refused 'a root without tools\lab\lab.ps1 is refused'
} finally {
    $env:CIMMERIA_LAB_HOME = $savedHome
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) failed"; exit 1 }
Write-Host 'all passed'
