<#
.SYNOPSIS
    One-shot setup for a lab against a local server: i686 target, tokens, lab account file, login row.

.DESCRIPTION
    Idempotent: every step keeps what is already there and works, and no
    secret value is ever printed.

      1. Adds the i686-pc-windows-msvc Rust target to the checkout's pinned
         toolchain when it is missing (tools/lab/install.ps1 builds the
         client DLLs for it).
      2. Sets user environment variables, keeping any that already work:
           CIMMERIA_LAB_MCP_TOKEN          32 random bytes (hex), when unset or short
           CIMMERIA_TELEMETRY_HMAC_SECRET  64 random bytes (hex), when unset or short
           CIMMERIA_LAB_MCP_BIND           127.0.0.1:8451
           CIMMERIA_LAB_MCP_URL            http://127.0.0.1:8451/mcp
           CIMMERIA_LAB_INSTALL_DIR        the install found (see -InstallDir)
         A server started from a NEW terminal (setup.ps1, or the exe) inherits
         them, which turns on its lab endpoint and the lab's telemetry mint;
         .mcp.json's lab-server entry expands the URL and token. Restart Claude
         Code (or Codex) and open a new terminal after the first run.
      3. Writes <install>\Binaries\sessions\lab-account.json when it is absent:
         login row Local, the seeded account lab / test, character Labone.
         Labone is not seeded: create it once with lab_create_character, or
         lab_ensure_in_world with a create block.
      4. Checks the client's LoginInternal.lua for a Local row pointing at
         http://127.0.0.1:8081. With -PatchLoginInternal it adds the row
         (after a backup); without it, it prints the line to add.
      5. Prints what is left to do.

.PARAMETER InstallDir
    The SGW install (the folder holding Binaries\SGW.exe). Default:
    CIMMERIA_LAB_INSTALL_DIR (process, then user), else labd.env's.
    Passed explicitly, it is also recorded as the user variable CIMMERIA_LAB_INSTALL_DIR, replacing a
    different value there (the row names both paths). Found any other way,
    a different stored value is kept with a WARN row.

.PARAMETER Worktree
    The checkout whose pinned toolchain gets the i686 target. Default: the
    repository the current directory is in when it is a Cimmeria checkout,
    else the one this script is in. A missing git, rustup or a failed
    `rustup target add` becomes a to-do item; the later steps still run.

.PARAMETER PatchLoginInternal
    Add the Local row to LoginInternal.lua, keeping a .bak copy. Never done
    without this switch.

.PARAMETER EnvScope
    Where the variables go: User (the default) or Process. Process changes
    nothing outside this run; it is for testing the script.

.EXAMPLE
    pwsh tools/lab/cli/local-setup.ps1 -InstallDir 'D:\Games\Stargate Worlds'
.EXAMPLE
    lab local-setup -PatchLoginInternal
#>
[CmdletBinding()]
param(
    [string]$InstallDir,
    [string]$Worktree,
    [switch]$PatchLoginInternal,
    [ValidateSet('User', 'Process')]
    [string]$EnvScope = 'User'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'local-setup-lib.ps1')

$I686 = 'i686-pc-windows-msvc'
$todo = [System.Collections.Generic.List[string]]::new()
function Say([string]$Text) { Write-Host "[local-setup] $Text" }

# --- 1. the i686 target ------------------------------------------------------------------
$checkout = Resolve-LabCheckout $Worktree $PSScriptRoot
if (-not $checkout) {
    Say "target:  no Cimmeria checkout found$(if ($Worktree) { " at $Worktree" }); skipped. Run it from a checkout, or pass -Worktree."
    $todo.Add("rustup target add $I686 from the checkout (it uses the pinned toolchain)")
} elseif (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Say 'target:  rustup not found; install Rust from https://rustup.rs'
    $todo.Add("install rustup, then rerun local-setup (it adds $I686)")
} else {
    # From the checkout, so rustup acts on the toolchain rust-toolchain.toml pins.
    # A failure here must not stop the steps below.
    Push-Location $checkout
    try {
        $installed = @(rustup target list --installed 2>$null)
        if ($installed -contains $I686) {
            Say "target:  $I686 already installed"
        } else {
            Say "target:  adding $I686 (the 32-bit client's DLLs need it)"
            rustup target add $I686
            if ($LASTEXITCODE -ne 0) { throw "rustup target add $I686 failed (exit $LASTEXITCODE)" }
        }
    } catch {
        Say "target:  WARN $($_.Exception.Message)"
        $todo.Add("from $checkout, run: rustup target add $I686")
    } finally { Pop-Location }
}

# --- 2. user environment -------------------------------------------------------------------
$store = New-LabEnvStore $EnvScope
foreach ($row in (Set-LabLocalEnv $store)) {
    Say ('env:     {0,-31} {1,-9} {2}' -f $row.name, $row.action, $row.note)
    if ($row.action -eq 'WARN') { $todo.Add("check $($row.name): $($row.note)") }
    # This process too, for anything it starts.
    [Environment]::SetEnvironmentVariable($row.name, (& $store.Get $row.name), 'Process')
}

# --- 3. the game install and the lab account -----------------------------------------------
$explicitInstall = $PSBoundParameters.ContainsKey('InstallDir')
if (-not $InstallDir) { $InstallDir = $env:CIMMERIA_LAB_INSTALL_DIR }
if (-not $InstallDir) { $InstallDir = & $store.Get 'CIMMERIA_LAB_INSTALL_DIR' }
if (-not $InstallDir) {
    $labdEnv = Get-LabdEnv
    if ($labdEnv.Contains('CIMMERIA_LAB_INSTALL_DIR')) { $InstallDir = $labdEnv['CIMMERIA_LAB_INSTALL_DIR'] }
}
$haveInstall = [bool]($InstallDir -and (Test-Path (Join-Path $InstallDir 'Binaries\SGW.exe')))
if (-not $haveInstall) {
    Say "game:    no SGW install found (got '$InstallDir'); skipped the account file and login row"
    $todo.Add('rerun: lab local-setup -InstallDir <folder holding Binaries\SGW.exe>')
} else {
    Say "game:    $InstallDir"
    $row = Set-LabInstallDirEnv $store $InstallDir $explicitInstall
    Say ('env:     {0,-31} {1,-9} {2}' -f $row.name, $row.action, $row.note)
    $account = Write-LabAccountFile $InstallDir
    $accountPath = Join-Path $InstallDir 'Binaries\sessions\lab-account.json'
    if ($account -eq 'written') {
        Say "account: wrote $accountPath (row $script:LabLocalServerRow, user $script:LabLocalUsername, character $script:LabLocalCharacter)"
    } else {
        $warnings = @(Get-LabAccountWarnings $InstallDir)
        if ($warnings.Count) {
            foreach ($w in $warnings) { Say "account: WARN kept $accountPath, but its $w" }
            $todo.Add("edit $accountPath for the local server (server $($script:LabLocalServerRow), username $($script:LabLocalUsername)), or delete it and rerun")
        } else {
            Say "account: kept $accountPath"
        }
    }
    $todo.Add("create the character once: lab_create_character (or lab_ensure_in_world with create) named $script:LabLocalCharacter, unless the account has it")

    # --- 4. the client's login row ------------------------------------------------------------
    $lua = Get-LoginInternalPath $InstallDir
    $row = Format-LoginRow $script:LabLocalServerRow $script:LabLocalLoginUrl
    $how = "add this line inside LoginMod.loadServerSystems(), after LoginMod.servers = {}:`n        $($row.Trim())"
    if (-not (Test-Path -LiteralPath $lua)) {
        Say "login:   $lua is missing"
        $todo.Add("create $lua the way the launcher writes it, and $how")
    } else {
        $text = [System.IO.File]::ReadAllText($lua)
        $plan = Add-LoginInternalRow $text $script:LabLocalServerRow $script:LabLocalLoginUrl
        switch ($plan.status) {
            'present' { Say "login:   $($script:LabLocalServerRow) row present ($($script:LabLocalLoginUrl))" }
            'conflict' {
                Say "login:   the $($script:LabLocalServerRow) row points at $($plan.url), not $($script:LabLocalLoginUrl); left alone"
                $todo.Add("point the $($script:LabLocalServerRow) row in $lua at $($script:LabLocalLoginUrl) if the server runs locally")
            }
            'unrecognised' {
                Say "login:   $lua has no LoginMod.servers table this script recognises; left alone"
                $todo.Add("in $lua, $how")
            }
            'added' {
                if ($PatchLoginInternal) {
                    $backup = "$lua.$(Get-Date -Format 'yyyyMMdd-HHmmss').bak"
                    Copy-Item -LiteralPath $lua -Destination $backup
                    [System.IO.File]::WriteAllText($lua, $plan.text, (New-Object System.Text.UTF8Encoding $false))
                    Say "login:   added the $($script:LabLocalServerRow) row; backup $backup"
                } else {
                    Say "login:   no $($script:LabLocalServerRow) row; not changed (pass -PatchLoginInternal to add it)"
                    $todo.Add("in $lua, $how")
                }
                Say '         the desktop launcher rewrites this file before each of its launches; lab launches do not'
            }
        }
    }
}

# --- 5. what is left -------------------------------------------------------------------------
$todo.Add('open a new terminal (it picks up the variables), start the server there, e.g. pwsh setup.ps1')
$todo.Add('pwsh tools/lab/install.ps1              build and install the lab binaries')
$todo.Add('pwsh tools/lab/daemon.ps1 install       the shared daemon (generates CIMMERIA_LAB_DAEMON_TOKEN)')
$todo.Add('MCP: copy .mcp.json.example to .mcp.json (Claude Code) or .codex/config.toml.example to .codex/config.toml (Codex); restart the client')
$todo.Add('lab doctor                              (checks the install, the daemon and each instance)')
Write-Host ''
Write-Host 'Left to do:'
$n = 0
foreach ($t in $todo) { $n++; Write-Host ("  {0}. {1}" -f $n, $t) }
