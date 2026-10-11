#Requires -Version 7
#Requires -RunAsAdministrator
# Sets up (or updates) the Windows runner pool on node3, run ON node3 as an admin by
# deploy.ps1: the toolchain, the egress proxy, the runner account and three runner
# services. Every step is idempotent: it checks what is there and does only what is
# missing or changed. See the "Windows pool" section of tools/ci-runners/README.md.
#
#   pwsh -File F:\gha\deploy\setup.ps1 -RustVersion 1.98.1
#
# Secrets arrive in -Secrets (deploy.ps1 writes it into an admin-only directory):
# JSON with `token` (a one-hour registration token, needed only by a runner that is
# not registered yet), `cache_endpoint`, `cache_password` and `egress_exit`. The last
# three are kept in $Root\deploy\node.json (admin-only) so a later run can omit them;
# the file named by -Secrets is deleted when this script ends.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$RustVersion,
    [string]$Root = 'F:\gha',
    [int]$Runners = 3,
    # Parallel compiles per job: 32 hardware threads shared by three runners.
    [int]$Jobs = 10,
    [string]$Node = 'node3',
    [string]$Repo = 'SandboxServers/Cimmeria',
    [string]$Account = 'gha-runner',
    [string]$Secrets = ''
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# Pinned downloads. SHA-256 values are the publishers' (actions/runner release notes,
# the .sha256 files next to rustup-init and sccache, Node's SHASUMS256.txt), except
# Python's: python.org publishes only MD5 and a signature for 3.12.10 (the last 3.12
# with a Windows installer), so the installer is checked against that MD5 and its
# Authenticode signer, and the SHA-256 is pinned here. VS Build Tools and WebView2 come
# from unversioned Microsoft links and are checked by Authenticode signer only.
$Pins = @{
    Runner  = @{ Version = '2.338.0'; Sha = 'f48e0750a21812bca5f82de5f7f5aeae71abee647fab5a582f1742d07eba455f' }
    Rustup  = @{ Version = '1.29.1'; Sha = '6f4bef66261261fcb43131be8720bab817d403a09edec7455c371974b90bdb7e' }
    Sccache = @{ Version = '0.18.0'; Sha = '8965c74d5e8a225244f741e18ad2f3f504f48228dc1bac948fc22761a348363d' }
    Python  = @{ Version = '3.12.10'; Sha = '67b5635e80ea51072b87941312d00ec8927c4db9ba18938f7ad2d27b328b95fb'; Md5 = '5eddb0b6f12c852725de071ae681dde4' }
    Node    = @{ Version = '24.21.0'; Sha = 'bb0eaee134f9357f22aea915ee793343e627aefc1e66488164bac6915bce2cac' }
}
# MSVC x64/x86 build tools and a Windows 11 SDK, as on windows-latest.
$VsComponents = @(
    'Microsoft.VisualStudio.Workload.VCTools',
    'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
    'Microsoft.VisualStudio.Component.Windows11SDK.26100'
)
$PythonDir = 'C:\Program Files\Python312'
$NodeDir = 'C:\Program Files\nodejs'
$GitDir = 'C:\Program Files\Git'
$Sccache = "$Root\tools\sccache\sccache.exe"
$EgressListen = '127.0.0.1:3128'
$EgressTask = 'cimmeria-ci-egress'
# Well-known SIDs, so the ACLs don't depend on the display language.
$System = '*S-1-5-18'; $Admins = '*S-1-5-32-544'; $LocalService = '*S-1-5-19'

$notes = [Collections.Generic.List[string]]::new()
function Step([string]$Text) { Write-Host "== $Text" }

function Set-Dacl([string]$Path, [string[]]$Grants, [switch]$Protect) {
    $a = @($Path)
    if ($Protect) { $a += '/inheritance:r' }
    foreach ($g in $Grants) { $a += @('/grant:r', $g) }
    $out = icacls @a /Q 2>&1
    if ($LASTEXITCODE -ne 0) { throw "icacls $Path failed: $out" }
}

function Get-Download([string]$Url, [string]$Name, [string]$Sha256 = '') {
    $dl = "$Root\deploy\downloads"
    New-Item -ItemType Directory -Force $dl | Out-Null
    $file = Join-Path $dl $Name
    if ($Sha256 -and (Test-Path $file) -and (Get-FileHash $file).Hash -eq $Sha256) { return $file }
    Invoke-WebRequest $Url -OutFile "$file.part" -UseBasicParsing
    if ($Sha256 -and (Get-FileHash "$file.part").Hash -ne $Sha256) {
        Remove-Item "$file.part"
        throw "$Name does not match its pinned SHA-256"
    }
    Move-Item -Force "$file.part" $file
    $file
}

function Assert-Signed([string]$File, [string]$Org) {
    $sig = Get-AuthenticodeSignature $File
    if ($sig.Status -ne 'Valid' -or $sig.SignerCertificate.Subject -notmatch "O=$([regex]::Escape($Org))(,|$)") {
        throw "$File is not validly signed by $Org (status $($sig.Status), signer $($sig.SignerCertificate.Subject))"
    }
}

function New-Password {
    $b = [byte[]]::new(24); [Security.Cryptography.RandomNumberGenerator]::Fill($b)
    # Complexity rules want upper, lower, digit and symbol; the random part is the secret.
    'Aa1!' + [Convert]::ToBase64String($b)
}

# --- secrets ---------------------------------------------------------------------------
$nodeFile = "$Root\deploy\node.json"
$cfg = @{ cache_endpoint = ''; cache_password = ''; egress_exit = '' }
if (Test-Path $nodeFile) {
    $saved = Get-Content -Raw $nodeFile | ConvertFrom-Json
    foreach ($k in @($cfg.Keys)) { if ($saved.PSObject.Properties[$k]) { $cfg[$k] = [string]$saved.$k } }
}
$token = ''
if ($Secrets -and (Test-Path $Secrets)) {
    $in = Get-Content -Raw $Secrets | ConvertFrom-Json
    foreach ($k in @($cfg.Keys)) { if ($in.PSObject.Properties[$k] -and $in.$k) { $cfg[$k] = [string]$in.$k } }
    if ($in.PSObject.Properties['token']) { $token = [string]$in.token }
}

try {
    # --- directories and ACLs -----------------------------------------------------------
    # $Root is admin-only; the runner account gets exactly the directories it needs.
    Step "directories under $Root"
    foreach ($d in 'deploy', 'tools', 'tools\sccache', 'hooks', 'shared', 'egress', 'egress\log') {
        New-Item -ItemType Directory -Force "$Root\$d" | Out-Null
    }
    Set-Dacl $Root @("${System}:(OI)(CI)F", "${Admins}:(OI)(CI)F") -Protect
    [IO.File]::WriteAllText($nodeFile, ($cfg | ConvertTo-Json))
    Set-Dacl $nodeFile @("${System}:F", "${Admins}:F") -Protect

    # --- runner account -----------------------------------------------------------------
    # A plain local user: no admin, no interactive use. Its password is random, handed to
    # config.cmd and to the services, and never stored. It is reset only when a runner
    # needs registering, and then every existing runner service gets the new one.
    $computer = [ADSI]"WinNT://$env:COMPUTERNAME,computer"
    $userExists = [bool](Get-CimInstance Win32_UserAccount -Filter "LocalAccount=True AND Name='$Account'")
    $needsConfig = @(1..$Runners | Where-Object { -not (Test-Path "$Root\r$_\runner\.runner") })
    $password = ''
    if (-not $userExists -or $needsConfig) {
        $password = New-Password
        if (-not $userExists) {
            Step "creating local user $Account"
            $u = $computer.Create('User', $Account)
            $u.SetPassword($password)
            $u.SetInfo()
            # SCRIPT | NORMAL_ACCOUNT (the defaults) | DONT_EXPIRE_PASSWD | PASSWD_CANT_CHANGE
            $u.Put('UserFlags', 0x1 -bor 0x200 -bor 0x10000 -bor 0x40)
            $u.Put('Description', 'GitHub Actions runner services (tools/ci-runners/windows)')
            $u.SetInfo()
        } else {
            ([ADSI]"WinNT://$env:COMPUTERNAME/$Account,user").SetPassword($password)
            Get-CimInstance Win32_Service | Where-Object { $_.Name -like 'actions.runner.*' -and $_.StartName -match "\\$([regex]::Escape($Account))$" } | ForEach-Object {
                $r = Invoke-CimMethod -InputObject $_ -MethodName Change -Arguments @{ StartName = ".\$Account"; StartPassword = $password }
                if ($r.ReturnValue -ne 0) { throw "updating $($_.Name)'s password failed ($($r.ReturnValue))" }
            }
        }
    }

    # --- toolchain ----------------------------------------------------------------------
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    function Test-Msvc {
        if (-not (Test-Path $vswhere)) { return $false }
        [bool](& $vswhere -products * -requires @($VsComponents | Select-Object -Skip 1) -property installationPath)
    }
    if (-not (Test-Msvc)) {
        Step 'installing VS 2022 Build Tools (C++ x64/x86, Windows 11 SDK); this takes a while'
        $vs = Get-Download 'https://aka.ms/vs/17/release/vs_buildtools.exe' 'vs_buildtools.exe'
        Assert-Signed $vs 'Microsoft Corporation'
        $a = @('--quiet', '--wait', '--norestart', '--nocache') + ($VsComponents | ForEach-Object { '--add', $_ })
        $p = Start-Process $vs -ArgumentList $a -Wait -PassThru
        if ($p.ExitCode -eq 3010) { $notes.Add('VS Build Tools asked for a reboot (exit 3010); builds normally work before it') }
        elseif ($p.ExitCode -ne 0) { throw "VS Build Tools installer exited $($p.ExitCode) (logs: $env:TEMP\dd_*.log)" }
        if (-not (Test-Msvc)) { throw 'VS Build Tools installed but vswhere does not find the C++ tools' }
    }

    $pyExe = "$PythonDir\python.exe"
    if (-not ((Test-Path $pyExe) -and ((& $pyExe --version) -eq "Python $($Pins.Python.Version)"))) {
        Step "installing Python $($Pins.Python.Version) for all users"
        $f = Get-Download "https://www.python.org/ftp/python/$($Pins.Python.Version)/python-$($Pins.Python.Version)-amd64.exe" "python-$($Pins.Python.Version)-amd64.exe" $Pins.Python.Sha
        if ((Get-FileHash $f -Algorithm MD5).Hash -ne $Pins.Python.Md5) { throw 'Python installer does not match python.org MD5' }
        Assert-Signed $f 'Python Software Foundation'
        $p = Start-Process $f -ArgumentList '/quiet', 'InstallAllUsers=1', 'PrependPath=1', 'Include_test=0', 'Include_launcher=1', 'InstallLauncherAllUsers=1', "TargetDir=`"$PythonDir`"" -Wait -PassThru
        if ($p.ExitCode -ne 0) { throw "Python installer exited $($p.ExitCode)" }
    }

    $nodeExe = "$NodeDir\node.exe"
    if (-not ((Test-Path $nodeExe) -and ((& $nodeExe --version) -eq "v$($Pins.Node.Version)"))) {
        Step "installing Node $($Pins.Node.Version)"
        $f = Get-Download "https://nodejs.org/dist/v$($Pins.Node.Version)/node-v$($Pins.Node.Version)-x64.msi" "node-v$($Pins.Node.Version)-x64.msi" $Pins.Node.Sha
        $p = Start-Process msiexec.exe -ArgumentList '/i', "`"$f`"", '/qn', '/norestart' -Wait -PassThru
        if ($p.ExitCode -notin 0, 3010) { throw "Node installer exited $($p.ExitCode)" }
    }

    $wv2Key = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    if (-not ((Test-Path $wv2Key) -and (Get-ItemProperty $wv2Key).PSObject.Properties['pv'])) {
        Step 'installing the WebView2 Runtime (evergreen)'
        $f = Get-Download 'https://go.microsoft.com/fwlink/?linkid=2124701' 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe'
        Assert-Signed $f 'Microsoft Corporation'
        $p = Start-Process $f -ArgumentList '/silent', '/install' -Wait -PassThru
        if ($p.ExitCode -ne 0) { throw "WebView2 installer exited $($p.ExitCode)" }
    }

    if (-not ((Test-Path $Sccache) -and ((& $Sccache --version) -eq "sccache $($Pins.Sccache.Version)"))) {
        Step "installing sccache $($Pins.Sccache.Version)"
        $n = "sccache-v$($Pins.Sccache.Version)-x86_64-pc-windows-msvc"
        $f = Get-Download "https://github.com/mozilla/sccache/releases/download/v$($Pins.Sccache.Version)/$n.zip" "$n.zip" $Pins.Sccache.Sha
        $x = "$Root\deploy\downloads\$n"
        Remove-Item -Recurse -Force $x -ErrorAction SilentlyContinue
        Expand-Archive $f "$Root\deploy\downloads"
        Copy-Item -Force "$x\sccache.exe" $Sccache
    }
    Set-Dacl "$Root\tools" @("${Account}:(OI)(CI)RX")

    # --- egress proxy -------------------------------------------------------------------
    # Log uploads go to Azure Storage, which the colo can't reach directly (README, "Colo
    # egress"). A scheduled task runs the node-role proxy as LOCAL SERVICE, at boot and
    # every five minutes (a no-op while it is running), so a crash is back within minutes.
    Step "egress proxy on $EgressListen"
    Copy-Item -Force "$PSScriptRoot\egress_proxy.py" "$Root\egress\egress_proxy.py"
    Set-Dacl "$Root\egress" @("${LocalService}:(OI)(CI)RX")
    Set-Dacl "$Root\egress\log" @("${LocalService}:(OI)(CI)M")
    $pa = "-I `"$Root\egress\egress_proxy.py`" --listen $EgressListen --allow-src 127.0.0.1/32 --log-file `"$Root\egress\log\egress.log`""
    if ($cfg.egress_exit) { $pa += " --via $($cfg.egress_exit) --via-dst 57.0.0.0/8 52.239.0.0/16" }
    $action = New-ScheduledTaskAction -Execute "$PythonDir\pythonw.exe" -Argument $pa
    $existing = Get-ScheduledTask -TaskName $EgressTask -ErrorAction SilentlyContinue
    if (-not $existing -or $existing.Actions[0].Arguments -ne $pa -or $existing.Actions[0].Execute -ne $action.Execute) {
        if ($existing) { Stop-ScheduledTask -TaskName $EgressTask -ErrorAction SilentlyContinue }
        $triggers = @(
            (New-ScheduledTaskTrigger -AtStartup),
            (New-ScheduledTaskTrigger -Once -At (Get-Date) -RepetitionInterval (New-TimeSpan -Minutes 5))
        )
        $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew `
            -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
        $principal = New-ScheduledTaskPrincipal -UserId 'NT AUTHORITY\LOCAL SERVICE' -LogonType ServiceAccount
        Register-ScheduledTask -TaskName $EgressTask -Action $action -Trigger $triggers -Settings $settings -Principal $principal -Force | Out-Null
    }
    if ((Get-ScheduledTask -TaskName $EgressTask).State -ne 'Running') { Start-ScheduledTask -TaskName $EgressTask }
    $up = $false
    foreach ($i in 1..20) {
        $t = [Net.Sockets.TcpClient]::new()
        try { if ($t.ConnectAsync('127.0.0.1', 3128).Wait(500) -and $t.Connected) { $up = $true; break } } catch { } finally { $t.Dispose() }
        Start-Sleep -Milliseconds 500
    }
    if (-not $up) { throw "the egress proxy is not listening on $EgressListen (see $Root\egress\log\egress.log)" }

    # node3 hosts other people's VMs, and its Users group can read their storage folders.
    # Deny the runner account read and list there (inherited by every file below), and
    # change nothing else: only an ACE is added, the VMs themselves are never touched.
    # The folders come from Hyper-V's own settings, read-only: the host defaults and
    # each VM's configuration, checkpoint, smart-paging and disk locations.
    $vmDirs = [Collections.Generic.List[string]]::new()
    if (Get-Command Get-VMHost -ErrorAction SilentlyContinue) {
        $h = Get-VMHost
        $vmDirs.Add($h.VirtualHardDiskPath); $vmDirs.Add($h.VirtualMachinePath)
        foreach ($vm in Get-VM) {
            foreach ($p in $vm.ConfigurationLocation, $vm.SnapshotFileLocation, $vm.SmartPagingFilePath) { $vmDirs.Add($p) }
            foreach ($d in $vm.HardDrives) { if ($d.Path) { $vmDirs.Add((Split-Path -Parent $d.Path)) } }
        }
    }
    $vmDirs = @($vmDirs | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Container) } |
            ForEach-Object { (Resolve-Path -LiteralPath $_).ProviderPath.TrimEnd('\') } | Sort-Object -Unique)
    foreach ($d in $vmDirs) {
        # /C: carry on past a file that refuses (an open VHD); icacls merges a repeat deny.
        $out = icacls $d /deny "${Account}:(OI)(CI)(RX)" /C /Q 2>&1
        if ($LASTEXITCODE -ne 0) { $notes.Add("denying $Account on ${d}: $out") }
    }
    Step "runner account denied on VM storage: $($vmDirs -join ', ')"

    # The listener refuses to start unless it can list every directory above its own, so
    # the account may list $Root itself (this folder only, nothing inherited).
    Set-Dacl $Root @("${Account}:(RX)")

    # Below-normal CPU priority for the runner processes; jobs inherit it (a child of a
    # below-normal process starts below-normal), so builds yield to the host's VMs.
    foreach ($exe in 'RunnerService.exe', 'Runner.Listener.exe', 'Runner.Worker.exe') {
        $k = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\$exe\PerfOptions"
        if (-not (Test-Path $k)) { New-Item -Path $k -Force | Out-Null }
        Set-ItemProperty -Path $k -Name CpuPriorityClass -Value 5 -Type DWord
    }

    # --- hooks and the shared cache settings --------------------------------------------
    Copy-Item -Force "$PSScriptRoot\hooks\job-started.ps1", "$PSScriptRoot\hooks\job-completed.ps1" "$Root\hooks\"
    Set-Dacl "$Root\hooks" @("${Account}:(OI)(CI)RX")
    # Read by job-started.ps1, which points sccache at Redis only when it answers.
    $cacheFile = "$Root\shared\cache.json"
    [IO.File]::WriteAllText($cacheFile, (@{ endpoint = $cfg.cache_endpoint; password = $cfg.cache_password } | ConvertTo-Json))
    Set-Dacl "$Root\shared" @("${Account}:(OI)(CI)RX")

    # --- runners ------------------------------------------------------------------------
    $rustup = Get-Download "https://static.rust-lang.org/rustup/archive/$($Pins.Rustup.Version)/x86_64-pc-windows-msvc/rustup-init.exe" "rustup-init-$($Pins.Rustup.Version).exe" $Pins.Rustup.Sha
    $runnerZip = Get-Download "https://github.com/actions/runner/releases/download/v$($Pins.Runner.Version)/actions-runner-win-x64-$($Pins.Runner.Version).zip" "actions-runner-win-x64-$($Pins.Runner.Version).zip" $Pins.Runner.Sha
    $cacheHost = if ($cfg.cache_endpoint) { ([uri]$cfg.cache_endpoint).Host } else { '' }
    $noProxy = (@('localhost', '127.0.0.1', '::1', $cacheHost) | Where-Object { $_ }) -join ','
    # Job steps reach github.com (and its subdomains) directly: through any CONNECT
    # proxy git falls back to HTTP/1.1, and on node3 a git fetch over HTTP/1.1 stalls
    # after the pack's response headers (direct HTTP/2 works; plain curl over HTTP/1.1
    # works too). Only the Azure Storage uploads need the detour.
    $stepNoProxy = "$noProxy,github.com"
    $machinePath = [Environment]::GetEnvironmentVariable('PATH', 'Machine')

    foreach ($n in 1..$Runners) {
        $name = "$Node-win-$n"
        $rd = "$Root\r$n"
        $runner = "$rd\runner"
        Step "runner $name in $rd"
        foreach ($d in 'runner', 'work', 'rust', 'state', 'sccache', 'tmp', 'lane') { New-Item -ItemType Directory -Force "$rd\$d" | Out-Null }
        # Modify, not FullControl: without delete-child on the directory, the account
        # can't replace the read-only .env below.
        Set-Dacl $rd @("${Account}:(OI)(CI)M")

        if (-not (Test-Path "$runner\config.cmd")) {
            Expand-Archive $runnerZip $runner
        }

        # A toolchain per runner: dtolnay/rust-toolchain runs `rustup toolchain install`
        # and `rustup default` in every job, and rustup doesn't lock against itself.
        $env:RUSTUP_HOME = "$rd\rust\rustup"; $env:CARGO_HOME = "$rd\rust\cargo"
        $env:RUSTUP_INIT_SKIP_PATH_CHECK = 'yes'
        $rustupExe = "$env:CARGO_HOME\bin\rustup.exe"
        if (-not (Test-Path $rustupExe)) {
            & $rustup -y --no-modify-path --profile minimal --default-toolchain $RustVersion -c clippy -c rustfmt -t i686-pc-windows-msvc
            if ($LASTEXITCODE -ne 0) { throw "rustup-init failed for $name" }
        }
        & $rustupExe toolchain install $RustVersion --profile minimal -c clippy -c rustfmt -t i686-pc-windows-msvc -t x86_64-pc-windows-msvc --no-self-update
        if ($LASTEXITCODE -ne 0) { throw "rustup toolchain install failed for $name" }
        & $rustupExe default $RustVersion | Out-Null
        Remove-Item Env:RUSTUP_HOME, Env:CARGO_HOME, Env:RUSTUP_INIT_SKIP_PATH_CHECK

        if (-not (Test-Path "$runner\.runner")) {
            if (-not $token) { throw "$name is not registered and no registration token was given; run deploy.ps1" }
            Step "registering $name"
            & "$runner\config.cmd" --unattended --replace --url "https://github.com/$Repo" --token $token `
                --name $name --labels "cimmeria-windows,$Node" --work "$rd\work" `
                --runasservice --windowslogonaccount ".\$Account" --windowslogonpassword $password
            if ($LASTEXITCODE -ne 0) { throw "config.cmd failed for $name" }
        }
        $svc = (Get-Content -Raw "$runner\.service").Trim()
        # config.cmd gives a group of its own (GITHUB_ActionsRunner_*, the account its only
        # member) full control of the runner directory; full control includes deleting
        # children, which would let a job replace the read-only .env. Modify is enough.
        foreach ($ace in @((Get-Acl $runner).Access | Where-Object { -not $_.IsInherited -and $_.IdentityReference -like '*\GITHUB_ActionsRunner_*' -and ($_.FileSystemRights -band [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles) })) {
            Set-Dacl $runner @("$($ace.IdentityReference):(OI)(CI)M")
        }

        # The environment every job step sees (the runner reads .env when it starts).
        # Git's bin first: System32 holds WSL's bash.exe, and `shell: bash` takes the
        # first bash on PATH. SCCACHE_CONF is written per job by job-started.ps1, which
        # adds the Redis backend only when it answers; SCCACHE_DIR is the fallback.
        $path = @("$rd\rust\cargo\bin", "$GitDir\bin", "$GitDir\cmd", "$PythonDir\Scripts", $PythonDir, $NodeDir, "$Root\tools\sccache") +
            @($machinePath -split ';' | Where-Object { $_ })
        $path = @($path | Select-Object -Unique) -join ';'
        $envText = (@(
                "PATH=$path"
                "RUSTUP_HOME=$rd\rust\rustup"
                "CARGO_HOME=$rd\rust\cargo"
                "RUSTC_WRAPPER=$Sccache"
                "SCCACHE_DIR=$rd\sccache"
                'SCCACHE_CACHE_SIZE=20G'
                'SCCACHE_IDLE_TIMEOUT=0'
                # One sccache server per runner; the default port would share one server
                # (and its config) between the three, and one job's stop would kill another's.
                "SCCACHE_SERVER_PORT=$(4226 + $n)"
                "SCCACHE_CONF=$rd\sccache.conf"
                "CARGO_BUILD_JOBS=$Jobs"
                # The lane's slot semaphore is per LANE_ROOT; one per runner, or the three
                # runners would queue on one machine-wide set of slots.
                "LANE_ROOT=$rd\lane"
                "TEMP=$rd\tmp"
                "TMP=$rd\tmp"
                "HTTPS_PROXY=http://$EgressListen"
                "https_proxy=http://$EgressListen"
                "NO_PROXY=$stepNoProxy"
                "no_proxy=$stepNoProxy"
                "CIMMERIA_RUNNER_ROOT=$rd"
                "CIMMERIA_CACHE_FILE=$cacheFile"
                "ACTIONS_RUNNER_HOOK_JOB_STARTED=$Root\hooks\job-started.ps1"
                "ACTIONS_RUNNER_HOOK_JOB_COMPLETED=$Root\hooks\job-completed.ps1"
            ) -join "`n") + "`n"
        $envFile = "$runner\.env"
        $changed = -not (Test-Path $envFile) -or ([IO.File]::ReadAllText($envFile) -ne $envText)
        if ($changed) { [IO.File]::WriteAllText($envFile, $envText) }
        Set-Dacl $envFile @("${System}:F", "${Admins}:F", "${Account}:R") -Protect

        # The listener itself uploads logs, so it needs the proxy before .env is read.
        $svcKey = "HKLM:\SYSTEM\CurrentControlSet\Services\$svc"
        $svcEnv = [string[]]@("https_proxy=http://$EgressListen", "HTTPS_PROXY=http://$EgressListen", "no_proxy=$noProxy", "NO_PROXY=$noProxy")
        $old = (Get-ItemProperty $svcKey).PSObject.Properties['Environment']
        if (-not $old -or (($old.Value -join "`n") -ne ($svcEnv -join "`n"))) {
            Set-ItemProperty -Path $svcKey -Name Environment -Value $svcEnv -Type MultiString
            $changed = $true
        }
        sc.exe failure $svc reset= 86400 actions= restart/60000/restart/60000/restart/60000 | Out-Null

        $s = Get-Service $svc
        if ($s.Status -ne 'Running') { Start-Service $svc }
        elseif ($changed) {
            $notes.Add("$name restarted for a changed environment (a job it was running, if any, was cut off)")
            Restart-Service $svc
        }
    }
} finally {
    if ($Secrets -and (Test-Path $Secrets)) { Remove-Item -Force $Secrets }
    $token = ''; $password = ''
}

Step 'installed'
"VS Build Tools: $((& $vswhere -products * -latest -property catalog_productDisplayVersion) -join '')"
"$(& "$PythonDir\python.exe" --version); node $(& "$NodeDir\node.exe" --version); $(& $Sccache --version)"
"WebView2 $((Get-ItemProperty $wv2Key).pv); rust $RustVersion; actions-runner $($Pins.Runner.Version)"
Get-Service 'actions.runner.*' | ForEach-Object { "$($_.Name): $($_.Status)" }
foreach ($m in $notes) { Write-Warning $m }
