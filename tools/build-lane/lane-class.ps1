# The build lane's job classes, for lane.ps1 (dot-source this file). Read the design in
# docs/analysis/lane-classes/design.md (§ 4.1-4.6) for the reasons.
#
# The lane has three classes. Each is its own counting semaphore in the same lock dir, and
# each uses the slot protocol of lane-slots.ps1:
#
#   heavy   slot.N         compiling cargo, and anything the classifier can't name.
#                          Count: LANE_SLOTS, else lane\SLOTS, else 2.
#   light   slot.light.N   rustfmt, cargo fmt/metadata/..., cargo check -p X.
#                          Count: LANE_LIGHT_SLOTS, else lane\LIGHT_SLOTS, else 4.
#   db      slot.db.N      the live-DB run phase; only by an explicit --class db.
#                          Count: LANE_DB_SLOTS, else lane\DB_SLOTS, else 3.
#
# A light or db count of 0 sends that class's jobs to heavy (the kill switch). A count
# that doesn't parse counts as unset. lane-class.sh is the bash twin; the rules are held
# equal by fixtures/lane-classes.tsv, which both lanes' tests read.

Set-StrictMode -Version Latest

# A count from a text value: a non-negative (heavy: positive) integer, else $null.
function ConvertTo-LaneCount([string]$Text, [int]$Min) {
    if ($Text -notmatch '^\s*(\d{1,4})\s*$') { return $null }
    $n = [int]$Matches[1]
    if ($n -lt $Min) { return $null }
    $n
}

# The class a command runs in, from its words alone: 'heavy' or 'light' (never 'db').
# $Cmd is the command as lane.ps1 received it, the program first.
function Get-LaneClass([string[]]$Cmd) {
    if (-not $Cmd -or $Cmd.Count -eq 0) { return 'heavy' }
    $exe = (($Cmd[0] -split '[\\/]')[-1]).ToLowerInvariant() -replace '\.exe$', ''
    if ($exe -eq 'rustfmt') { return 'light' }
    if ($exe -ne 'cargo') { return 'heavy' }

    $words = @($Cmd | Select-Object -Skip 1)
    if ($words.Count -gt 0 -and $words[0].StartsWith('+')) { $words = @($words | Select-Object -Skip 1) }
    # Global options: --color, --config and -Z, -C take a value; any other dash word is one word.
    $i = 0
    while ($i -lt $words.Count -and $words[$i].StartsWith('-')) {
        if ($words[$i] -in '--color', '--config', '-Z', '-C') { $i += 2 } else { $i++ }
    }
    if ($i -ge $words.Count) { return 'light' }   # cargo --version, and bare cargo
    $sub = $words[$i]
    $rest = @($words | Select-Object -Skip ($i + 1))

    if ($sub -in 'fmt', 'metadata', 'tree', 'version', 'locate-project', 'pkgid', 'verify-project',
                  'search', 'help', 'update', 'generate-lockfile', 'fetch', 'hakari', 'machete') { return 'light' }
    if ($sub -notin 'check', 'c') { return 'heavy' }

    # cargo check is light only for one named package: -p X, --package X or --package=X,
    # and no --workspace or --all.
    $pkg = $false; $all = $false
    for ($j = 0; $j -lt $rest.Count; $j++) {
        $w = $rest[$j]
        if (($w -eq '-p' -or $w -eq '--package') -and $j + 1 -lt $rest.Count -and -not $rest[$j + 1].StartsWith('-')) { $pkg = $true }
        elseif ($w -match '^--package=.+') { $pkg = $true }
        elseif ($w -eq '--workspace' -or $w -eq '--all') { $all = $true }
    }
    if ($pkg -and -not $all) { 'light' } else { 'heavy' }
}

# The slot count for $Class: LANE_<X>_SLOTS, then <LockDir>\<X>SLOTS file, then the default.
function Get-LaneClassCount([string]$LockDir, [string]$Class) {
    $spec = switch ($Class) {
        'light' { @('LANE_LIGHT_SLOTS', 'LIGHT_SLOTS', 4) }
        'db'    { @('LANE_DB_SLOTS', 'DB_SLOTS', 3) }
        default { @('LANE_SLOTS', 'SLOTS', 2) }
    }
    $min = if ($Class -eq 'heavy') { 1 } else { 0 }
    $n = ConvertTo-LaneCount ([Environment]::GetEnvironmentVariable($spec[0])) $min
    if ($null -eq $n) {
        $text = ''
        try { $text = [System.IO.File]::ReadAllText((Join-Path $LockDir $spec[1])) } catch { }
        $n = ConvertTo-LaneCount $text $min
    }
    if ($null -eq $n) { $n = $spec[2] }
    $n
}

# The path of slot $I of $Class: slot.I, slot.light.I or slot.db.I.
function Get-LaneSlotPath([string]$LockDir, [string]$Class, [int]$I) {
    switch ($Class) {
        'light' { Join-Path $LockDir "slot.light.$I" }
        'db'    { Join-Path $LockDir "slot.db.$I" }
        default { Join-Path $LockDir "slot.$I" }
    }
}

# The slot dirs of $Class only (Get-SlotDirs in lane-slots.ps1 returns every class).
function Get-LaneClassDirs([string]$LockDir, [string]$Class) {
    $pattern = switch ($Class) {
        'light' { '^slot\.light\.\d+$' }
        'db'    { '^slot\.db\.\d+$' }
        default { '^slot\.\d+$' }
    }
    @(Get-ChildItem -LiteralPath $LockDir -Directory -Filter 'slot.*' -ErrorAction SilentlyContinue |
        Where-Object Name -Match $pattern)
}

# The `-j` for cargo when the caller didn't set CARGO_BUILD_JOBS. Heavy shares the cores
# among the $Heavy heavy slots (floor 4); light and db use LANE_LIGHT_JOBS (default 4).
function Get-LaneClassJobs([string]$Class, [int]$Heavy) {
    if ($Class -eq 'heavy') { return [int][math]::Max(4, [math]::Floor([Environment]::ProcessorCount / $Heavy)) }
    $n = ConvertTo-LaneCount $env:LANE_LIGHT_JOBS 1
    if ($null -eq $n) { 4 } else { $n }
}

# The slot dir of the job that this process runs inside, or $null. A lane call is nested
# when CIMMERIA_LANE_SLOT names a dir whose `token` file holds CIMMERIA_LANE_TOKEN: the
# token, not the path, stops a detached grandchild of a finished job from riding a slot
# that someone else now holds.
function Test-LaneNested {
    $slot = $env:CIMMERIA_LANE_SLOT
    $token = $env:CIMMERIA_LANE_TOKEN
    if (-not $slot -or -not $token) { return $null }
    if (-not (Test-Path -LiteralPath $slot -PathType Container)) { return $null }
    $held = ''
    try { $held = ([System.IO.File]::ReadAllText((Join-Path $slot 'token'))).Trim() } catch { }
    if ($held -and $held -ceq $token) { return $slot }
    $null
}
