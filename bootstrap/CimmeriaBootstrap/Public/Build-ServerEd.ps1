function Build-ServerEd {
    <#
    .SYNOPSIS
        Builds the ServerEd Qt admin tool via qmake + nmake.

    .DESCRIPTION
        Locates Qt and Visual Studio, then builds ServerEd.pro using qmake and nmake
        in the MSVC environment. The resulting ServerEd.exe is copied to bin64/.

        This is idempotent - if ServerEd.exe already exists in bin64/ and is newer
        than all source files, the build is skipped.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

    .EXAMPLE
        Build-ServerEd

    .EXAMPLE
        Build-ServerEd -Configuration Release
    #>
    [CmdletBinding()]
    param(
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Release"
    )

    $paths = Get-ProjectPaths
    $serverEdDir = Join-Path $paths.ProjectRoot "tools\ServerEd"
    $binDir = Join-Path $paths.ProjectRoot "bin64"

    if (-not ($IsWindows -or (-not (Test-Path variable:IsWindows)))) {
        throw "Build-ServerEd requires Windows with Visual Studio and Qt."
    }

    Write-Step "BUILDING SERVERED"

    # Verify ServerEd.pro exists
    $proFile = Join-Path $serverEdDir "ServerEd.pro"
    if (-not (Test-Path $proFile)) {
        throw "ServerEd.pro not found at: $proFile"
    }

    # Find Qt
    $qtInfo = Find-Qt
    if (-not $qtInfo) {
        throw "Qt not found. Run Install-CimmeriaDependencies to download Qt 5.15."
    }
    Write-Status "Qt: $($qtInfo.InstallPath) (v$($qtInfo.Version))" "DarkGray"

    # Find Visual Studio
    if (-not $script:VSInfo) {
        $script:VSInfo = Find-VisualStudio
    }
    if (-not $script:VSInfo) {
        throw "Visual Studio not found. Install VS with C++ tools first."
    }

    # Determine output exe location
    $destExe = Join-Path $binDir "ServerEd.exe"

    # Idempotency: skip if ServerEd.exe exists and is newer than all sources
    if (Test-Path $destExe) {
        $exeTime = (Get-Item $destExe).LastWriteTime
        $sourceFiles = Get-ChildItem $serverEdDir -Include "*.cpp","*.h","*.ui","*.pro","*.qrc" -Recurse
        $newerSource = $sourceFiles | Where-Object { $_.LastWriteTime -gt $exeTime } | Select-Object -First 1
        if (-not $newerSource) {
            Write-Status "ServerEd.exe is up-to-date, skipping build" "DarkGray"
            return
        }
        Write-Status "Source files changed, rebuilding..." "White"
    }

    # Find vcvarsall.bat
    $vcvarsall = Join-Path $script:VSInfo.InstallPath "VC\Auxiliary\Build\vcvarsall.bat"
    if (-not (Test-Path $vcvarsall)) {
        throw "vcvarsall.bat not found at: $vcvarsall"
    }

    # Build configuration for qmake
    $qmakeConfig = if ($Configuration -eq "Debug") { "debug" } else { "release" }
    $buildSubdir = if ($Configuration -eq "Debug") { "debug" } else { "release" }

    # Clean previous qmake-generated files for a fresh build
    $makefilePath = Join-Path $serverEdDir "Makefile"
    if (Test-Path $makefilePath) {
        Remove-Item $makefilePath -Force -ErrorAction SilentlyContinue
    }
    $makefileConfig = Join-Path $serverEdDir "Makefile.$buildSubdir"
    if (Test-Path $makefileConfig) {
        Remove-Item $makefileConfig -Force -ErrorAction SilentlyContinue
    }

    # Step 1: Run qmake to generate Makefiles
    $qmakePath = $qtInfo.QmakePath
    $qmakeCmd = @(
        "`"$vcvarsall`" x64",
        "cd /d `"$serverEdDir`"",
        "`"$qmakePath`" ServerEd.pro -spec win32-msvc CONFIG+=$qmakeConfig"
    ) -join " && "

    Write-Status "Running qmake ($Configuration)..." "White"
    cmd /c "$qmakeCmd" 2>&1 | ForEach-Object {
        $line = "$_"
        if ($line -match 'error|fatal|warning') {
            Write-Status "  $_" "DarkGray"
        }
    }
    if ($LASTEXITCODE -ne 0) {
        throw "qmake failed with exit code $LASTEXITCODE"
    }

    # Step 2: Patch qmake LIBS bug (Qt 5.15 qmake emits bare Qt lib dir in LIBS line)
    # qmake generates "...qtmaind.lib C:\...\qt\lib shell32.lib" — the bare directory path
    # is interpreted by the linker as a file, causing LNK1104 looking for "lib.obj".
    $makefileDebug = Join-Path $serverEdDir "Makefile.Debug"
    $makefileRelease = Join-Path $serverEdDir "Makefile.Release"
    foreach ($mf in @($makefileDebug, $makefileRelease)) {
        if (Test-Path $mf) {
            $content = Get-Content $mf -Raw
            $patched = $content -replace '[A-Za-z]:\\[^\s]+\\qt\\lib\s+(shell32\.lib)', '$1'
            if ($patched -ne $content) {
                Set-Content $mf $patched -NoNewline
                Write-Status "  Patched LIBS bug in $(Split-Path $mf -Leaf)" "DarkGray"
            }
        }
    }

    # Step 3: Run nmake to compile
    $nmakeCmd = @(
        "`"$vcvarsall`" x64",
        "cd /d `"$serverEdDir`"",
        "nmake $qmakeConfig"
    ) -join " && "

    Write-Status "Running nmake ($Configuration)..." "White"
    $lineCount = 0
    cmd /c "$nmakeCmd" 2>&1 | ForEach-Object {
        $lineCount++
        $line = "$_"
        if ($line -match 'error|fatal|LINK :|ServerEd\.exe') {
            Write-Status "  $_" "DarkGray"
        } elseif ($lineCount % 50 -eq 0) {
            Write-Status "  $lineCount lines processed..." "DarkGray"
        }
    }
    $buildExitCode = $LASTEXITCODE

    # Find the built exe
    $builtExe = Join-Path $serverEdDir "$buildSubdir\ServerEd.exe"
    if (-not (Test-Path $builtExe)) {
        # Some qmake setups put it directly in the project dir
        $builtExe = Join-Path $serverEdDir "ServerEd.exe"
    }

    if (-not (Test-Path $builtExe)) {
        throw "ServerEd.exe was not produced. Check build output above for errors (exit code: $buildExitCode)."
    }

    # Copy to bin64/
    New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    Copy-Item -Path $builtExe -Destination $destExe -Force
    Write-Status "Copied: ServerEd.exe -> bin64/" "Green"

    Write-Status "Build-ServerEd complete ($Configuration)." "Green"
}
