<#
.SYNOPSIS
    Build the legacy C++ NavBuilder as a standalone exe (no Boost, no SOCI,
    no unified_kernel) from deprecated/cpp/src/nav_builder + external/recast.

.DESCRIPTION
    deprecated/cpp-build/projects/NavBuilder.vcxproj cannot be built any more:
    its precompiled header (deprecated/cpp/src/stdafx.hpp) needs Boost
    (python/asio/thread), SOCI, TinyXML and unified_kernel.lib, none of which
    setup.ps1 provisions. NavBuilder itself uses none of them - only a logger
    and three Boost.uBLAS names - so this script compiles the five nav_builder
    sources plus Recast's sources straight into one exe with `cl`, putting
    deprecated/cpp/src/nav_builder/standalone/ first on the include path so
    its stdafx.hpp shim wins.

    The reference binary bin64\NavBuilder_d.exe is never written to: this
    script refuses any -Out whose file name is NavBuilder_d.exe.

.EXAMPLE
    tools\build-navbuilder.ps1 -Out $env:TEMP\NavBuilder.exe

.EXAMPLE
    # Worktree without external/: borrow the primary checkout's Recast.
    tools\build-navbuilder.ps1 -RecastRoot C:\src\Cimmeria\external\recast -Out C:\tmp\NavBuilder.exe
#>
[CmdletBinding()]
param(
    [string]$Out,
    [string]$RecastRoot,
    [ValidateSet('Release', 'Debug')][string]$Config = 'Release',
    [string]$VcVars
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$src = Join-Path $repo 'deprecated\cpp\src\nav_builder'

if (-not $RecastRoot) { $RecastRoot = Join-Path $repo 'external\recast' }
if (-not $Out) { $Out = Join-Path $repo 'bin64\NavBuilder.exe' }
$Out = [System.IO.Path]::GetFullPath($Out)

if ((Split-Path -Leaf $Out) -ieq 'NavBuilder_d.exe') {
    throw "Refusing to overwrite the reference binary NavBuilder_d.exe; pick another -Out."
}
if (-not (Test-Path (Join-Path $RecastRoot 'Recast\Include\Recast.h'))) {
    throw "Recast not found at '$RecastRoot' (external/ is populated by setup.ps1; pass -RecastRoot otherwise)."
}

if (-not $VcVars) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path $vswhere) {
        $vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($vs) { $VcVars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat' }
    }
}
if (-not $VcVars -or -not (Test-Path $VcVars)) {
    throw "vcvars64.bat not found; install the MSVC x64 toolset or pass -VcVars <path>."
}

$objDir = Join-Path ([System.IO.Path]::GetTempPath()) ("navbuilder-obj-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force $objDir | Out-Null
New-Item -ItemType Directory -Force (Split-Path -Parent $Out) | Out-Null

$sources = @(Get-ChildItem (Join-Path $src '*.cpp')) + @(Get-ChildItem (Join-Path $RecastRoot 'Recast\Source\*.cpp'))
# /fp:precise + no /arch override keeps float results identical to the
# original Debug x64 build, so default-parameter output is byte-comparable.
$flags = @('/nologo', '/EHsc', '/W3', '/fp:precise', '/DNAVBUILDER_STANDALONE', '/D_CRT_SECURE_NO_WARNINGS', '/DWIN32_LEAN_AND_MEAN', '/DNOMINMAX')
if ($Config -eq 'Release') { $flags += @('/O2', '/MT', '/DNDEBUG') } else { $flags += @('/Od', '/Zi', '/MTd') }
$includes = @(
    (Join-Path $src 'standalone'), $src,
    (Join-Path $RecastRoot 'Recast\Include'), (Join-Path $RecastRoot 'Detour\Include')
) | ForEach-Object { "/I`"$_`"" }

$rsp = Join-Path $objDir 'cl.rsp'
$lines = @($flags) + @($includes) + @($sources | ForEach-Object { "`"$($_.FullName)`"" }) +
    @("/Fo`"$objDir\\`"", "/Fd`"$objDir\\`"", "/Fe`"$Out`"")
Set-Content -Path $rsp -Value $lines -Encoding ascii

try {
    & cmd.exe /d /c "`"$VcVars`" >nul && cl @`"$rsp`""
    if ($LASTEXITCODE -ne 0) { throw "cl failed with exit code $LASTEXITCODE" }
}
finally {
    Remove-Item -Recurse -Force $objDir -ErrorAction SilentlyContinue
}

Write-Host "Built $Out"
