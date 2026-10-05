<#
.SYNOPSIS
    Build the Live Research Lab binaries from a worktree and install them.

.DESCRIPTION
    Builds, through the build lane (tools/build-lane/lane.sh):
      - cimmeria-lab (release, host): the MCP supervisor
      - cimmeria-client-telemetry --features lab-bridge (release, i686): the bridge DLL
      - cimmeria-start32 and cimmeria-client-patches (release, i686): the injector
        helper and the client patch DLL
    then installs them:
      %LOCALAPPDATA%\cimmeria-lab\bin\cimmeria-lab.exe
      %LOCALAPPDATA%\cimmeria-lab\bin\sgw-start32.exe
      %LOCALAPPDATA%\cimmeria-lab\bin\cimmeria_client_patches.dll
      %LOCALAPPDATA%\cimmeria-lab\bin\cimmeria_client_telemetry.dll
      <CIMMERIA_LAB_INSTALL_DIR>\Binaries\cimmeria-client-telemetry.dll
    The last one is the DLL the supervisor injects by default
    (CIMMERIA_LAB_DLL overrides it).

    Refuses while any SGW.exe runs, and names the supervisor that owns it.
    Each replaced file is kept as <name>.<yyyymmdd>.old. The running
    cimmeria-lab.exe is renamed, not overwritten (Windows lets you rename a
    running executable), so the old supervisor keeps running until you
    reconnect the MCP server.

    How-to: docs/guides/live-research-lab.md#install-or-update-the-lab

.PARAMETER Worktree
    The checkout to build from. Default: the repository the current
    directory is in, else the one this script is in.

.PARAMETER InstallDir
    The SGW install (the folder holding Binaries\SGW.exe). Default:
    CIMMERIA_LAB_INSTALL_DIR, else the cimmeria-lab entry of the main
    checkout's .mcp.json.

.PARAMETER LabHome
    Where the lab lives. Default: %LOCALAPPDATA%\cimmeria-lab.

.PARAMETER SkipBuild
    Install what the target dir already holds; build nothing.

.PARAMETER DryRun
    Print what would be built, refused, backed up and copied; change nothing.

.EXAMPLE
    pwsh tools/lab/install.ps1 -DryRun
.EXAMPLE
    pwsh tools/lab/install.ps1 -Worktree C:\src\Cimmeria\.claude\worktrees\lab-fix
#>
[CmdletBinding()]
param(
    [string]$Worktree,
    [string]$InstallDir,
    [string]$LabHome = (Join-Path $env:LOCALAPPDATA 'cimmeria-lab'),
    [switch]$SkipBuild,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$I686 = 'i686-pc-windows-msvc'

function Say([string]$Text) { Write-Host "[lab-install] $Text" }

# --- the worktree and its commit -------------------------------------------------------
if (-not $Worktree) {
    $Worktree = git rev-parse --show-toplevel 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $Worktree) {
        $Worktree = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    }
}
$Worktree = (Resolve-Path $Worktree).Path
if (-not (Test-Path (Join-Path $Worktree 'tools\build-lane\lane.sh'))) {
    throw "$Worktree is not a Cimmeria checkout (no tools\build-lane\lane.sh)"
}
$Commit = (git -C $Worktree rev-parse HEAD).Trim()
$Subject = (git -C $Worktree log -1 --format=%s).Trim()
$Dirty = @(git -C $Worktree status --porcelain --untracked-files=no).Count
Say "source:  $Worktree"
Say "commit:  $Commit  $Subject"
if ($Dirty -gt 0) { Say "warning: $Dirty tracked file(s) modified; the build includes them" }

# --- the game install -------------------------------------------------------------------
if (-not $InstallDir) { $InstallDir = $env:CIMMERIA_LAB_INSTALL_DIR }
if (-not $InstallDir) {
    # The main checkout holds the (gitignored) .mcp.json; a worktree has none.
    $common = git -C $Worktree rev-parse --path-format=absolute --git-common-dir 2>$null
    $mcp = if ($common) { Join-Path (Split-Path $common -Parent) '.mcp.json' } else { $null }
    if ($mcp -and (Test-Path $mcp)) {
        $entry = (Get-Content $mcp -Raw | ConvertFrom-Json).mcpServers.'cimmeria-lab'
        if ($entry -and $entry.env) { $InstallDir = $entry.env.CIMMERIA_LAB_INSTALL_DIR }
    }
}
if (-not $InstallDir -or -not (Test-Path (Join-Path $InstallDir 'Binaries\SGW.exe'))) {
    throw "no SGW install found (got '$InstallDir'). Pass -InstallDir or set CIMMERIA_LAB_INSTALL_DIR to the folder holding Binaries\SGW.exe"
}
Say "game:    $InstallDir"
if ($env:CIMMERIA_LAB_DLL) {
    Say "warning: CIMMERIA_LAB_DLL is set ($env:CIMMERIA_LAB_DLL); a supervisor with it injects that file, not Binaries\cimmeria-client-telemetry.dll"
}

# --- refuse while a client runs ----------------------------------------------------------
function Get-ParentChain([int]$ProcessId) {
    $links = @()
    $p = Get-CimInstance Win32_Process -Filter "ProcessId=$ProcessId" -ErrorAction SilentlyContinue
    while ($p -and $links.Count -lt 12) {
        $links += "$($p.Name)[$($p.ProcessId)]"
        $parent = Get-CimInstance Win32_Process -Filter "ProcessId=$($p.ParentProcessId)" -ErrorAction SilentlyContinue
        # A parent created after its child is a reused PID, not the parent.
        if (-not $parent -or $parent.CreationDate -gt $p.CreationDate) { break }
        $p = $parent
    }
    $links -join ' <- '
}

$clients = @(Get-Process -Name SGW -ErrorAction SilentlyContinue)
if ($clients.Count -gt 0) {
    Say "REFUSED: $($clients.Count) SGW.exe running. Installing over a client's DLL fails or swaps it under a live session."
    foreach ($c in $clients) {
        Say "  SGW.exe pid $($c.Id)  parents: $(Get-ParentChain $c.Id)"
        # sgw-start32 exits after injecting, so the parent chain usually stops at
        # SGW.exe. The supervisor that owns it holds a connection to its bridge port.
        $ports = @(Get-NetTCPConnection -OwningProcess $c.Id -State Listen -ErrorAction SilentlyContinue |
                   Select-Object -ExpandProperty LocalPort -Unique)
        foreach ($port in $ports) {
            $owners = @(Get-NetTCPConnection -RemotePort $port -State Established -ErrorAction SilentlyContinue |
                        Where-Object { $_.OwningProcess -ne $c.Id } |
                        Select-Object -ExpandProperty OwningProcess -Unique)
            if ($owners.Count -eq 0) { Say "    bridge port $port : no supervisor connected" }
            foreach ($o in $owners) { Say "    bridge port $port : supervisor $(Get-ParentChain $o)" }
        }
        if ($ports.Count -eq 0) { Say "    no listening port: not a lab client (no bridge), or the bridge is down" }
    }
    $owner = Join-Path $LabHome 'live.lock\owner'
    if (Test-Path $owner) { Say "  live.lock owner: $((Get-Content $owner -Raw).Trim())" }
    Say "  Ask the session that owns the supervisor to run lab_client_stop; killing the client makes its watchdog relaunch it."
    if (-not $DryRun) { exit 1 }
    Say "  (dry run: continuing to show the plan)"
}

# --- build through the lane ---------------------------------------------------------------
$bash = Join-Path $env:ProgramFiles 'Git\bin\bash.exe'   # not WSL's bash.exe
if (-not (Test-Path $bash)) { $bash = 'bash' }
$builds = @(
    @('cargo', 'build', '-p', 'cimmeria-lab', '--release'),
    @('cargo', 'build', '-p', 'cimmeria-client-telemetry', '--features', 'lab-bridge', '--target', $I686, '--release'),
    @('cargo', 'build', '-p', 'cimmeria-start32', '-p', 'cimmeria-client-patches', '--target', $I686, '--release')
)
foreach ($b in $builds) {
    $line = "bash tools/build-lane/lane.sh $($b -join ' ')"
    if ($SkipBuild) { Say "skip:    $line"; continue }
    if ($DryRun) { Say "would build: $line"; continue }
    Say "build:   $line"
    Push-Location $Worktree
    try { & $bash tools/build-lane/lane.sh @b; $rc = $LASTEXITCODE } finally { Pop-Location }
    if ($rc -ne 0) { throw "build failed (exit $rc): $line. Read the failures file or log the lane printed." }
}

# --- find the outputs: the same target dir the lane picks --------------------------------
$targetRoot = $env:CIMMERIA_TARGET_ROOT
if (-not $targetRoot) { $targetRoot = [Environment]::GetEnvironmentVariable('CIMMERIA_TARGET_ROOT', 'User') }
$name = Split-Path $Worktree -Leaf
$local = Join-Path $Worktree 'target'
$targetDir = $local
if ($targetRoot -and (Test-Path $targetRoot)) {
    $devDrive = Join-Path $targetRoot $name
    if ((Test-Path $devDrive) -or -not (Test-Path $local)) { $targetDir = $devDrive }
}
Say "target:  $targetDir"

$telemetry = Join-Path $targetDir "$I686\release\cimmeria_client_telemetry.dll"
$bin = Join-Path $LabHome 'bin'
$plan = @(
    @{ From = (Join-Path $targetDir 'release\cimmeria-lab.exe');                 To = (Join-Path $bin 'cimmeria-lab.exe') },
    @{ From = (Join-Path $targetDir "$I686\release\sgw-start32.exe");            To = (Join-Path $bin 'sgw-start32.exe') },
    @{ From = (Join-Path $targetDir "$I686\release\cimmeria_client_patches.dll"); To = (Join-Path $bin 'cimmeria_client_patches.dll') },
    @{ From = $telemetry;                                                        To = (Join-Path $bin 'cimmeria_client_telemetry.dll') },
    @{ From = $telemetry;                                                        To = (Join-Path $InstallDir 'Binaries\cimmeria-client-telemetry.dll') }
)
$missing = @($plan | Where-Object { -not (Test-Path $_.From) } | ForEach-Object { $_.From } | Select-Object -Unique)
if ($missing.Count -gt 0) {
    $msg = "build output missing: $($missing -join ', ')"
    if ($DryRun) { Say "warning: $msg (dry run: nothing built yet)" } else { throw $msg }
}

# --- install -----------------------------------------------------------------------------
$stamp = Get-Date -Format 'yyyyMMdd'
if (-not $DryRun) { New-Item -ItemType Directory -Force -Path $bin | Out-Null }
foreach ($item in $plan) {
    $from = $item.From; $to = $item.To
    $built = if (Test-Path $from) { (Get-Item $from).LastWriteTime.ToString('yyyy-MM-dd HH:mm') } else { 'not built' }
    if ((Test-Path $to) -and (Test-Path $from) -and
        (Get-FileHash $from).Hash -eq (Get-FileHash $to).Hash) {
        Say "same:    $to"
        continue
    }
    $backup = $null
    if (Test-Path $to) {
        $backup = "$to.$stamp.old"
        if (Test-Path $backup) { $backup = "$to.$(Get-Date -Format 'yyyyMMdd-HHmmss').old" }
    }
    if ($DryRun) {
        if ($backup) { Say "would back up: $to -> $(Split-Path $backup -Leaf)" }
        Say "would copy: $from ($built) -> $to"
        continue
    }
    # Rename, then copy: a rename works on a running cimmeria-lab.exe, an overwrite does not.
    if ($backup) { Move-Item -LiteralPath $to -Destination $backup; Say "backup:  $backup" }
    Copy-Item -LiteralPath $from -Destination $to
    Say "install: $to ($built)"
}

if (-not $DryRun) {
    Set-Content -Path (Join-Path $bin 'installed-from.txt') -Value @(
        "commit=$Commit", "subject=$Subject", "worktree=$Worktree",
        "installed=$((Get-Date).ToString('s'))", "dirty_files=$Dirty")
}

# --- what to do next -----------------------------------------------------------------------
$running = @(Get-Process -Name cimmeria-lab -ErrorAction SilentlyContinue)
Say "source commit: $Commit"
if ($running.Count -gt 0) {
    Say "running supervisor(s), still the old build: $(($running | ForEach-Object { $_.Id }) -join ', ')"
}
Say "next: reconnect the MCP server (/mcp reconnect cimmeria-lab). If it stays on the old process,"
Say "      stop that cimmeria-lab.exe first, then reconnect. Verify with lab_uat_run { plan_only: true }."
if ($DryRun) { Say "dry run: nothing was built, moved or copied" }
