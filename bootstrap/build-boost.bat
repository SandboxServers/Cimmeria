@echo off
setlocal
cls

echo ============================================
echo  Boost 1.55.0 Build Script for Cimmeria
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

cd /d "%~dp0..\external\boost"

echo.
echo Building Boost libraries (date_time, math, python, system, thread)...
echo This may take several minutes.
echo.

tools\build\v2\b2.exe --user-config=user-config.jam -j 4 --build-dir=../boost-lib64 --stagedir=./lib64 toolset=msvc-14.5 link=static threading=multi runtime-link=shared --build-type=minimal --with-date_time --with-math --with-python --with-system --with-thread address-model=64 stage

if errorlevel 1 goto :fail

echo.
echo Cleaning build intermediates...
rd /q /s ..\boost-lib64 2>nul

echo.
echo ============================================
echo  Boost build complete!
echo  Output: external\boost\lib64\lib\
echo ============================================
goto :end

:fail
echo.
echo BUILD FAILED
echo.

:end
pause
endlocal
