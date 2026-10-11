# Tests for Install-CimmeriaLauncher (setup.ps1 -WithLauncher) against a local
# HTTP server standing in for the launcher-current release: a good release
# unpacks into launcher\, a re-run replaces it, a digest mismatch, a .sha256
# naming another file and a zip without the exe at its root are refused and
# leave the existing launcher\ alone, and an unpublished release (404) returns
# without failing. Windows only, like the function. Needs python on PATH for
# the server. bootstrap-lint.yml runs it.
#
# Usage: pwsh -NoProfile -File bootstrap/tests/test-install-launcher.ps1
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { Write-Host 'skip: Install-CimmeriaLauncher is Windows only'; exit 0 }

$base = Join-Path ([IO.Path]::GetTempPath()) "cimmeria-install-launcher-$PID"
$srv = Join-Path $base 'srv'
$root = Join-Path $base 'root'
New-Item -ItemType Directory $srv, $root -Force | Out-Null
$zipName = 'StargateWorlds-Launcher-windows-x64.zip'
$failed = 0

function Check([string]$name, [bool]$ok, [string]$detail = '') {
    if ($ok) { Write-Host "ok   $name" } else { Write-Host "FAIL $name $detail"; $script:failed++ }
}

# One release directory: a zip of the launcher layout and its .sha256.
function New-Release([string]$name, [bool]$withExe = $true, [string]$shaName = $zipName, [bool]$wrongDigest = $false) {
    $dir = Join-Path $srv $name
    $pkg = Join-Path $base "pkg-$name"
    New-Item -ItemType Directory $dir, (Join-Path $pkg 'windows'), (Join-Path $pkg 'graphics') -Force | Out-Null
    if ($withExe) { Set-Content (Join-Path $pkg 'cimmeria-launcher-desktop.exe') 'MZ launcher' }
    Set-Content (Join-Path $pkg 'windows\cimmeria_client_patches.dll') 'MZ patches'
    Set-Content (Join-Path $pkg 'graphics\d3d9.dll') 'MZ graphics'
    $zip = Join-Path $dir $zipName
    Compress-Archive -Path (Join-Path $pkg '*') -DestinationPath $zip
    $hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($wrongDigest) { $hash = '0' * 64 }
    Set-Content "$zip.sha256" "$hash  $shaName" -NoNewline
}

New-Release 'good'
New-Release 'bad-digest' -wrongDigest $true
New-Release 'wrong-name' -shaName 'other.zip'
New-Release 'no-exe' -withExe $false

$listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
$listener.Start(); $port = $listener.LocalEndpoint.Port; $listener.Stop()
$server = Start-Process python -ArgumentList '-I', '-m', 'http.server', "$port", '--bind', '127.0.0.1', '--directory', $srv -PassThru -WindowStyle Hidden
try {
    $url = "http://127.0.0.1:$port"
    $deadline = (Get-Date).AddSeconds(15)
    while ($true) {
        try { Invoke-WebRequest "$url/" -UseBasicParsing | Out-Null; break } catch {
            if ((Get-Date) -gt $deadline) { throw "the test HTTP server did not start on $url" }
            Start-Sleep -Milliseconds 200
        }
    }

    $module = Import-Module (Join-Path $PSScriptRoot '..\CimmeriaBootstrap') -Force -PassThru
    & $module { param($r) $script:ProjectRoot = $r } $root
    $launcher = Join-Path $root 'launcher'

    Install-CimmeriaLauncher -BaseUrl "$url/good" *> $null
    Check 'a good release unpacks into launcher\' ((Test-Path "$launcher\cimmeria-launcher-desktop.exe") -and (Test-Path "$launcher\windows\cimmeria_client_patches.dll") -and (Test-Path "$launcher\graphics\d3d9.dll"))

    Set-Content "$launcher\stale.txt" 'old'
    Install-CimmeriaLauncher -BaseUrl "$url/good" *> $null
    Check 'a re-run replaces launcher\' (-not (Test-Path "$launcher\stale.txt"))

    foreach ($case in 'bad-digest', 'wrong-name', 'no-exe') {
        Set-Content "$launcher\marker.txt" 'keep'
        $threw = $false
        try { Install-CimmeriaLauncher -BaseUrl "$url/$case" *> $null } catch { $threw = $true }
        Check "$case is refused" $threw
        Check "$case leaves launcher\ alone" (Test-Path "$launcher\marker.txt")
    }

    $threw = $false
    try { Install-CimmeriaLauncher -BaseUrl "$url/missing" *> $null } catch { $threw = $true }
    Check 'an unpublished release (404) returns without failing' (-not $threw)
} finally {
    Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
    Remove-Item $base -Recurse -Force -ErrorAction SilentlyContinue
}

if ($failed -gt 0) { Write-Host "$failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
