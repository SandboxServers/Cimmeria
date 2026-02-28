@echo off
setlocal enabledelayedexpansion
cls

echo ============================================
echo  OpenSSL 1.0.1e Build Script for Cimmeria
echo ============================================
echo.

REM --- Find Visual Studio ---
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
    echo ERROR: vswhere.exe not found. Is Visual Studio installed?
    goto :fail
)

for /f "usebackq tokens=*" %%i in (`"%VSWHERE%" -latest -property installationPath`) do set "VSDIR=%%i"
if not defined VSDIR (
    echo ERROR: No Visual Studio installation found.
    goto :fail
)
echo Found Visual Studio at: %VSDIR%

REM --- Set up x64 build environment ---
set "VCVARS=%VSDIR%\VC\Auxiliary\Build\vcvarsall.bat"
if not exist "%VCVARS%" (
    echo ERROR: vcvarsall.bat not found at %VCVARS%
    goto :fail
)
echo Setting up x64 build environment...
call "%VCVARS%" x64 >nul 2>&1

REM --- Verify MSVC tools ---
where cl >nul 2>&1
if errorlevel 1 (
    echo ERROR: cl.exe not found after setting up environment
    goto :fail
)
where nmake >nul 2>&1
if errorlevel 1 (
    echo ERROR: nmake.exe not found
    goto :fail
)

REM --- Find Perl (use full path, NOT a wrapper) ---
REM Git for Windows bundles Perl in usr\bin, but we must NOT add that
REM directory to PATH because it contains Unix tools (link, sort, find)
REM that shadow MSVC tools. We use the full path to perl.exe AND create
REM a safe wrapper directory on PATH so nmake's generated commands can
REM find "perl" without pulling in the rest of Git's usr\bin.
set "PERL="
set "PERL_WRAPPER_DIR="

REM Check for standalone Perl first (e.g. Strawberry Perl)
for /f "tokens=*" %%i in ('where perl 2^>nul') do (
    if not defined PERL (
        echo %%i | findstr /i /c:"\usr\bin" >nul
        if errorlevel 1 (
            set "PERL=%%i"
        )
    )
)

REM Fall back to Git's bundled Perl
if not defined PERL (
    for /f "tokens=*" %%i in ('where git 2^>nul') do (
        if not defined PERL (
            for %%G in ("%%i") do set "PERL=%%~dpG..\usr\bin\perl.exe"
        )
    )
    if defined PERL (
        if not exist "!PERL!" (
            set "PERL="
        )
    )
)

if not defined PERL (
    echo ERROR: Perl not found. Install Strawberry Perl or ensure Git for Windows is on PATH.
    goto :fail
)
echo Using Perl: %PERL%

REM Create a safe wrapper directory with ONLY a perl forwarder on PATH.
REM This lets nmake's generated makefile find "perl" without exposing
REM Git's usr\bin (which contains link.exe, sort.exe, etc. that shadow MSVC).
set "PERL_WRAPPER_DIR=%TEMP%\openssl_perl_%RANDOM%"
mkdir "!PERL_WRAPPER_DIR!" 2>nul
> "!PERL_WRAPPER_DIR!\perl.bat" echo @"%PERL%" %%*
set "PATH=!PERL_WRAPPER_DIR!;!PATH!"
echo Created safe perl wrapper for nmake

REM --- Navigate to OpenSSL source ---
set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."
set "OPENSSL_SRC=%PROJECT_ROOT%\external\openssl_src"
set "OPENSSL_OUT=%PROJECT_ROOT%\external\openssl"

if not exist "%OPENSSL_SRC%\Configure" (
    echo ERROR: OpenSSL source not found at %OPENSSL_SRC%
    goto :fail
)

cd /d "%OPENSSL_SRC%"
echo Working in: %CD%
echo.

REM --- Clean any previous build ---
if exist ms\nt.mak (
    echo Cleaning previous build...
    nmake -f ms\nt.mak clean >nul 2>&1
)
if exist out32 rd /s /q out32
if exist out32.dbg rd /s /q out32.dbg
if exist tmp32 rd /s /q tmp32
if exist tmp32.dbg rd /s /q tmp32.dbg
if exist inc32 rd /s /q inc32

REM ============================================
REM  Build Debug (libeay32MTd.lib)
REM ============================================
echo.
echo === Building DEBUG x64 static library ===
echo.

"%PERL%" Configure debug-VC-WIN64A no-asm
if errorlevel 1 (
    echo ERROR: Configure failed for debug build
    goto :fail
)

REM Generate makefile directly with debug- prefix
REM (do_win64a.bat hardcodes non-debug platform, so we call scripts directly)
"%PERL%" util\mkfiles.pl >MINFO
if errorlevel 1 (
    echo ERROR: mkfiles.pl failed
    goto :fail
)

"%PERL%" util\mk1mf.pl debug VC-WIN64A >ms\nt.mak
if errorlevel 1 (
    echo ERROR: mk1mf.pl failed for debug build
    goto :fail
)

echo.
echo Running nmake for debug build...
nmake -f ms\nt.mak
if errorlevel 1 (
    echo.
    echo ERROR: Debug build failed. See errors above.
    goto :fail
)

REM --- Copy debug output ---
if not exist "%OPENSSL_OUT%\lib64" mkdir "%OPENSSL_OUT%\lib64"

set "DEBUG_OUT=out32.dbg"
if not exist "%DEBUG_OUT%\libeay32.lib" set "DEBUG_OUT=out32"
if not exist "%DEBUG_OUT%\libeay32.lib" (
    echo ERROR: libeay32.lib not found in output directory
    dir out32* /b 2>nul
    goto :fail
)

copy /y "%DEBUG_OUT%\libeay32.lib" "%OPENSSL_OUT%\lib64\libeay32MTd.lib"
echo.
echo Debug library copied to: external\openssl\lib64\libeay32MTd.lib

REM --- Clean for release build ---
echo.
echo Cleaning for release build...
nmake -f ms\nt.mak clean >nul 2>&1
if exist out32 rd /s /q out32
if exist out32.dbg rd /s /q out32.dbg
if exist tmp32 rd /s /q tmp32
if exist tmp32.dbg rd /s /q tmp32.dbg

REM ============================================
REM  Build Release (libeay32MT.lib)
REM ============================================
echo.
echo === Building RELEASE x64 static library ===
echo.

"%PERL%" Configure VC-WIN64A no-asm
if errorlevel 1 (
    echo ERROR: Configure failed for release build
    goto :fail
)

"%PERL%" util\mkfiles.pl >MINFO
"%PERL%" util\mk1mf.pl VC-WIN64A >ms\nt.mak
if errorlevel 1 (
    echo ERROR: mk1mf.pl failed for release build
    goto :fail
)

echo.
echo Running nmake for release build...
nmake -f ms\nt.mak
if errorlevel 1 (
    echo.
    echo ERROR: Release build failed. See errors above.
    goto :fail
)

REM --- Copy release output ---
set "RELEASE_OUT=out32"
if not exist "%RELEASE_OUT%\libeay32.lib" (
    echo ERROR: libeay32.lib not found in release output directory
    goto :fail
)

copy /y "%RELEASE_OUT%\libeay32.lib" "%OPENSSL_OUT%\lib64\libeay32MT.lib"
echo.
echo Release library copied to: external\openssl\lib64\libeay32MT.lib

REM --- Done ---
echo.
echo ============================================
echo  OpenSSL build complete!
echo  Output: external\openssl\lib64\
echo    - libeay32MTd.lib (debug)
echo    - libeay32MT.lib  (release)
echo ============================================
goto :end

:fail
echo.
echo BUILD FAILED
echo.

:end
REM Clean up temp wrapper directory
if defined PERL_WRAPPER_DIR (
    if exist "!PERL_WRAPPER_DIR!" rd /s /q "!PERL_WRAPPER_DIR!"
)
cd /d "%PROJECT_ROOT%"
pause
endlocal
