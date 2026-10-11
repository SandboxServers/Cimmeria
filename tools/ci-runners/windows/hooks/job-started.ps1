# Runs before every job (ACTIONS_RUNNER_HOOK_JOB_STARTED). Logs the disk picture, and
# picks this job's compile cache by writing SCCACHE_CONF: the shared Redis on the
# cache host when it answers within two seconds, else nothing, which leaves sccache on
# the runner's own SCCACHE_DIR. sccache 0.18 doesn't fail open on a Redis that drops
# packets: its startup storage check outlasts the client's 10-second wait, so the
# first compile of every job would fail or stall. The job log is public, so this
# never prints the cache's address or password.
$ErrorActionPreference = 'Continue'
$root = $env:CIMMERIA_RUNNER_ROOT
$sccache = $env:RUSTC_WRAPPER

# A server left over from an earlier job would keep its old backend.
if ($sccache) { & $sccache --stop-server *> $null }

$backend = 'local disk'
$conf = ''
try {
    $c = Get-Content -Raw -LiteralPath $env:CIMMERIA_CACHE_FILE | ConvertFrom-Json
    if ($c.endpoint) {
        $u = [uri]$c.endpoint
        $t = New-Object Net.Sockets.TcpClient
        try {
            if ($t.ConnectAsync($u.Host, $u.Port).Wait(2000) -and $t.Connected) {
                $esc = { param($s) ([string]$s).Replace('\', '\\').Replace('"', '\"') }
                $conf = "[cache.redis]`nendpoint = `"$(& $esc $c.endpoint)`"`npassword = `"$(& $esc $c.password)`"`n"
                $backend = 'shared Redis'
            } else {
                $backend = 'local disk (the shared Redis did not answer)'
            }
        } catch {
            $backend = 'local disk (the shared Redis did not answer)'
        } finally {
            $t.Dispose()
        }
    }
} catch {
    $backend = 'local disk (no cache settings)'
}
if ($env:SCCACHE_CONF) { [IO.File]::WriteAllText($env:SCCACHE_CONF, $conf) }

function Get-SizeGB([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return 0 }
    $sum = (Get-ChildItem -LiteralPath $Path -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
    [math]::Round([double]$sum / 1GB, 1)
}
$drive = New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($root))
Write-Host ("runner {0}: {1} {2:N0} GB free, sccache dir {3} GB, compile cache: {4}" -f `
        $env:RUNNER_NAME, $drive.Name, ($drive.AvailableFreeSpace / 1GB), (Get-SizeGB "$root\sccache"), $backend)
exit 0
