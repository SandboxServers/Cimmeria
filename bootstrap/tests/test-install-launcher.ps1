# Tests for Install-CimmeriaLauncher (setup.ps1 -WithLauncher) against a local
# HTTP server standing in for the launcher-current release:
#   - a good release unpacks into launcher\, and a re-run replaces it and
#     leaves no launcher.old behind;
#   - a digest mismatch, a .sha256 naming another file, a zip without the exe
#     at its root and a zip with a `..\` entry are refused, each by its own
#     check, and leave the existing launcher\ alone;
#   - only one of the two files present is reported as a mid-refresh release;
#   - an unpublished release (both 404) returns without failing;
#   - a plain-http URL to anything but loopback is refused before any request.
# Windows only, like the function. Needs python on PATH for the server.
# bootstrap-lint.yml runs it.
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
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

function Check([string]$name, [bool]$ok, [string]$detail = '') {
    if ($ok) { Write-Host "ok   $name" } else { Write-Host "FAIL $name $detail"; $script:failed++ }
}

# A zip from (entry name -> text) pairs, written entry by entry so a test can
# name an entry `..\x`, which Compress-Archive cannot.
function New-Zip([string]$path, [System.Collections.Specialized.OrderedDictionary]$entries) {
    $zip = [IO.Compression.ZipFile]::Open($path, 'Create')
    try {
        foreach ($name in $entries.Keys) {
            $writer = [IO.StreamWriter]::new($zip.CreateEntry($name).Open())
            $writer.Write($entries[$name]); $writer.Dispose()
        }
    } finally { $zip.Dispose() }
}

# One release directory: the launcher layout zipped, and its .sha256.
function New-Release([string]$name, [switch]$NoExe, [string]$ShaName = $zipName, [switch]$WrongDigest,
                     [switch]$Escape, [switch]$NoZip, [switch]$NoSha) {
    $dir = Join-Path $srv $name
    New-Item -ItemType Directory $dir -Force | Out-Null
    $entries = [ordered]@{}
    if (-not $NoExe) { $entries['cimmeria-launcher-desktop.exe'] = 'MZ launcher' }
    $entries['windows/cimmeria_client_patches.dll'] = 'MZ patches'
    $entries['graphics/d3d9.dll'] = 'MZ graphics'
    if ($Escape) { $entries['..\escaped.txt'] = 'outside' }
    $zip = Join-Path $dir $zipName
    New-Zip $zip $entries
    $hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($WrongDigest) { $hash = '0' * 64 }
    if (-not $NoSha) { Set-Content "$zip.sha256" "$hash  $ShaName" -NoNewline }
    if ($NoZip) { Remove-Item $zip }
}

New-Release 'good'
New-Release 'bad-digest' -WrongDigest
New-Release 'wrong-name' -ShaName 'other.zip'
New-Release 'no-exe' -NoExe
New-Release 'zip-slip' -Escape
New-Release 'sha-only' -NoZip
New-Release 'zip-only' -NoSha

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

    # The message a refused install throws, or $null when it did not throw.
    function Install-Message([string]$from) {
        try { Install-CimmeriaLauncher -BaseUrl $from *> $null; return $null } catch { return "$_" }
    }

    $msg = Install-Message "$url/good"
    Check 'a good release unpacks into launcher\' ($null -eq $msg -and (Test-Path "$launcher\cimmeria-launcher-desktop.exe") -and (Test-Path "$launcher\windows\cimmeria_client_patches.dll") -and (Test-Path "$launcher\graphics\d3d9.dll")) $msg

    Set-Content "$launcher\stale.txt" 'old'
    $msg = Install-Message "$url/good"
    Check 'a re-run replaces launcher\' ($null -eq $msg -and -not (Test-Path "$launcher\stale.txt")) $msg
    Check 'a re-run leaves no launcher.old' (-not (Test-Path "$launcher.old"))

    $cases = [ordered]@{
        'bad-digest' = 'retry in a minute'
        'wrong-name' = "names 'other.zip'"
        'no-exe'     = 'has no cimmeria-launcher-desktop.exe'
        'zip-slip'   = 'has an entry outside the folder'
        'sha-only'   = 'retry in a minute'
        'zip-only'   = 'retry in a minute'
    }
    foreach ($case in $cases.Keys) {
        Set-Content "$launcher\marker.txt" 'keep'
        $msg = Install-Message "$url/$case"
        Check "$case is refused by its own check" ($null -ne $msg -and $msg.Contains($cases[$case])) "(got: $msg)"
        Check "$case leaves launcher\ alone" (Test-Path "$launcher\marker.txt")
    }
    Check 'zip-slip wrote nothing outside the staging folder' (-not (Get-ChildItem -Path $base -Recurse -Filter 'escaped.txt'))

    $msg = Install-Message "$url/missing"
    Check 'an unpublished release (404) returns without failing' ($null -eq $msg) $msg

    $msg = Install-Message 'http://example.invalid/launcher-current'
    Check 'plain http to a non-loopback host is refused' ($null -ne $msg -and $msg.Contains('must be https')) "(got: $msg)"
} finally {
    Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
    Remove-Item $base -Recurse -Force -ErrorAction SilentlyContinue
}

if ($failed -gt 0) { Write-Host "$failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
