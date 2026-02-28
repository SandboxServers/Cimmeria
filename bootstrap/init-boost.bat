@echo off
setlocal
cls

echo ============================================
echo  Boost Build Engine (b2) Bootstrap
echo ============================================
echo.

REM --- Find Visual Studio ---
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
    echo ERROR: vswhere.exe not found.
    goto :fail
)
for /f "usebackq tokens=*" %%i in (`"%VSWHERE%" -latest -property installationPath`) do set "VSDIR=%%i"
echo Found Visual Studio at: %VSDIR%

set "VCVARS=%VSDIR%\VC\Auxiliary\Build\vcvarsall.bat"
call "%VCVARS%" x64 >nul 2>&1

cd /d "%~dp0..\external\boost\tools\build\v2"

REM Use vc12 toolset profile - it uses /MTd /RTC1 flags compatible with modern MSVC.
REM The generic 'msvc' profile uses /MLd which was removed in VS2005.
call bootstrap.bat vc12

if not exist "b2.exe" goto :fail

echo.
echo ============================================
echo  b2.exe bootstrapped successfully
echo ============================================
goto :end

:fail
echo.
echo BOOTSTRAP FAILED
echo.

:end
pause
endlocal
