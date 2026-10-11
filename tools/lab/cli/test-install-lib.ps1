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

    # --- the lab.cmd shim and lab.exe (LX-03) ---
    $homeBin = Join-Path $env:CIMMERIA_LAB_HOME 'bin'
    $shimPath = Join-Path $homeBin 'lab.cmd'
    $exePath = Join-Path $homeBin 'lab.exe'
    Remove-Item -LiteralPath $homeBin -Recurse -Force -ErrorAction SilentlyContinue

    Check (-not (Test-LabExeInstalled $env:CIMMERIA_LAB_HOME)) 'no lab.exe: Test-LabExeInstalled is false'
    $written = Write-LabShim $env:CIMMERIA_LAB_HOME
    Check (($written -eq $shimPath) -and (Test-Path -LiteralPath $shimPath)) 'without lab.exe, Write-LabShim writes bin\lab.cmd and returns its path'
    $shimText = Get-Content -Raw -LiteralPath $shimPath
    Check (($shimText -like '*pwsh -NoProfile -File*') -and ($shimText -like '*cli\lab.ps1*') -and ($shimText -like '*%*')) 'the shim runs cli\lab.ps1 with the arguments'

    Install-LabCli $root
    Check (Test-Path -LiteralPath $shimPath) 'without lab.exe, Install-LabCli leaves lab.cmd in place'

    Set-Content -LiteralPath $exePath -Value 'fake exe' -Encoding ascii
    Check (Test-LabExeInstalled $env:CIMMERIA_LAB_HOME) 'with a bin\lab.exe, Test-LabExeInstalled is true'
    $said = @(Install-LabCli $root 6>&1 | ForEach-Object { "$_" })
    Check (-not (Test-Path -LiteralPath $shimPath)) 'with lab.exe, Install-LabCli deletes bin\lab.cmd'
    Check (Test-Path -LiteralPath $exePath) 'deleting the shim leaves lab.exe alone'
    Check (($said -join "`n") -like '*lab.exe installed; the lab.cmd shim is no longer needed*') 'the shim removal prints the notice'

    Install-LabCli $root
    Check (-not (Test-Path -LiteralPath $shimPath)) 'with lab.exe and no shim, a second run leaves none'

    # setup.ps1 as a whole, in a child process; -NoPath keeps it off the user PATH.
    $setup = Join-Path $root 'tools\lab\cli\setup.ps1'
    $savedHomeForSetup = $env:CIMMERIA_LAB_HOME
    $env:CIMMERIA_LAB_HOME = Join-Path $tmp 'home-setup'
    try {
        & pwsh -NoProfile -File $setup -From $root -NoPath | Out-Null
        Check (($LASTEXITCODE -eq 0) -and (Test-Path -LiteralPath (Join-Path $env:CIMMERIA_LAB_HOME 'bin\lab.cmd'))) 'setup without lab.exe writes bin\lab.cmd'
        Set-Content -LiteralPath (Join-Path $env:CIMMERIA_LAB_HOME 'bin\lab.exe') -Value 'fake exe' -Encoding ascii
        & pwsh -NoProfile -File $setup -From $root -NoPath | Out-Null
        Check (($LASTEXITCODE -eq 0) -and -not (Test-Path -LiteralPath (Join-Path $env:CIMMERIA_LAB_HOME 'bin\lab.cmd'))) 'setup with lab.exe removes bin\lab.cmd and does not write it again'
    } finally {
        $env:CIMMERIA_LAB_HOME = $savedHomeForSetup
    }

    # --- Install-LabFile: rename, then copy ---
    $fileDir = Join-Path $tmp 'files'
    New-Item -ItemType Directory $fileDir | Out-Null
    $src = Join-Path $fileDir 'src.exe'
    $dst = Join-Path $fileDir 'dst.exe'
    Set-Content -LiteralPath $src -Value 'new build' -Encoding ascii
    Install-LabFile $src $dst
    Check ((Get-Content -Raw -LiteralPath $dst).Trim() -eq 'new build') 'Install-LabFile copies to a missing destination'
    Check (@(Get-ChildItem -LiteralPath $fileDir -Filter '*.old').Count -eq 0) 'a first install leaves no backup'

    Install-LabFile $src $dst
    Check (@(Get-ChildItem -LiteralPath $fileDir -Filter '*.old').Count -eq 0) 'an identical destination is skipped, with no backup'

    Set-Content -LiteralPath $src -Value 'newer build' -Encoding ascii
    Install-LabFile $src $dst
    $olds = @(Get-ChildItem -LiteralPath $fileDir -Filter 'dst.exe.*.old')
    Check ((Get-Content -Raw -LiteralPath $dst).Trim() -eq 'newer build') 'a changed file replaces the destination'
    Check (($olds.Count -eq 1) -and ((Get-Content -Raw -LiteralPath $olds[0].FullName).Trim() -eq 'new build')) 'the replaced destination is kept as <name>.<date>.old'

    Set-Content -LiteralPath $src -Value 'third build' -Encoding ascii
    Install-LabFile $src $dst
    Check (@(Get-ChildItem -LiteralPath $fileDir -Filter 'dst.exe.*.old').Count -eq 2) 'a second backup the same day gets a time-stamped name, not an overwrite'

    Set-Content -LiteralPath $src -Value 'dry build' -Encoding ascii
    Install-LabFile $src $dst -DryRun
    Check ((Get-Content -Raw -LiteralPath $dst).Trim() -eq 'third build') '-DryRun does not change the destination'
    Check (@(Get-ChildItem -LiteralPath $fileDir -Filter 'dst.exe.*.old').Count -eq 2) '-DryRun makes no backup'

    # A missing source must not cost the destination its file (the rename comes first).
    $gone = $false
    try { Install-LabFile (Join-Path $fileDir 'absent.exe') $dst } catch { $gone = $true }
    Check ($gone -and ((Get-Content -Raw -LiteralPath $dst).Trim() -eq 'third build') -and (@(Get-ChildItem -LiteralPath $fileDir -Filter 'dst.exe.*.old').Count -eq 2)) 'a missing source throws and leaves the destination in place'

    # Replacing a running exe (lab install runs from lab.exe itself): rename works, overwrite does not.
    $runDir = Join-Path $tmp 'running'
    New-Item -ItemType Directory $runDir | Out-Null
    $runExe = Join-Path $runDir 'lab.exe'
    Copy-Item -LiteralPath (Join-Path $env:WINDIR 'System32\PING.EXE') -Destination $runExe
    $proc = Start-Process -FilePath $runExe -ArgumentList '-n', '30', '127.0.0.1' -PassThru -WindowStyle Hidden
    try {
        Start-Sleep -Milliseconds 500
        $newExe = Join-Path $runDir 'new.exe'
        Copy-Item -LiteralPath (Join-Path $env:WINDIR 'System32\whoami.exe') -Destination $newExe
        Install-LabFile $newExe $runExe
        Check (((Get-FileHash -LiteralPath $runExe).Hash -eq (Get-FileHash -LiteralPath $newExe).Hash) -and (@(Get-ChildItem -LiteralPath $runDir -Filter 'lab.exe.*.old').Count -eq 1)) 'a running exe is replaced, the old one kept as .old'
    } finally {
        $proc.Kill()
        $proc.WaitForExit()
    }
    # --- tools\lab\install.ps1 builds and installs lab.exe (text guard; it cannot run without a game install) ---
    $installText = Get-Content -Raw -LiteralPath (Join-Path $repoLab 'install.ps1')
    Check ($installText -match "'-p', 'cimmeria-lab', '-p', 'cimmeria-lab-cli', '--release'") 'install.ps1 builds cimmeria-lab and cimmeria-lab-cli in one lane call'
    Check ($installText -match 'CIMMERIA_LAB_CLI_SHA') 'install.ps1 sets CIMMERIA_LAB_CLI_SHA for the CLI build'
    Check ($installText -match "'release\\lab\.exe'") 'install.ps1 plans release\lab.exe'
    Check ($installText -match "Join-Path \`$bin 'lab\.exe'") 'install.ps1 installs it to bin\lab.exe'
    Check ($installText -match 'Install-LabFile -From') 'install.ps1 copies through Install-LabFile'
    Check ($installText -notmatch 'Move-Item') 'install.ps1 no longer has its own rename-then-copy'
} finally {
    $env:CIMMERIA_LAB_HOME = $savedHome
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) failed"; exit 1 }
Write-Host 'all passed'
