@echo off
setlocal enabledelayedexpansion
cls

echo ============================================
echo  SOCI 3.2.1 Build Script for Cimmeria
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

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."
set "SOCI_SRC=%PROJECT_ROOT%\external\soci\src"
set "PG_INC=%PROJECT_ROOT%\external\postgresql\include"

REM Generate soci_backends_config.h if missing (normally CMake-generated).
REM Also defines SOCI_LIB_PREFIX/SUFFIX for the dynamic backend loader
REM (unused in our static-link build, but must compile).
if not exist "%SOCI_SRC%\core\soci_backends_config.h" (
    echo Generating soci_backends_config.h...
    (
        echo #ifndef SOCI_BACKENDS_CONFIG_H
        echo #define SOCI_BACKENDS_CONFIG_H
        echo #define DEFAULT_BACKENDS_PATH ""
        echo #define SOCI_LIB_PREFIX ""
        echo #define SOCI_LIB_SUFFIX ".dll"
        echo #endif
    ) > "%SOCI_SRC%\core\soci_backends_config.h"
)

REM Common compiler flags
set "COMMON_FLAGS=/nologo /c /EHsc /W3 /DSOCI_LIB /D_CRT_SECURE_NO_WARNINGS /D_CRT_SECURE_NO_DEPRECATE"
set "INCLUDE_CORE=/I"%SOCI_SRC%\core""
set "INCLUDE_PG=/I"%SOCI_SRC%\backends\postgresql" /I"%PG_INC%""

REM ============================================
REM  Build Debug libraries
REM ============================================
echo.
echo === Building DEBUG SOCI libraries (x64) ===
echo.

set "TMP_DIR=%PROJECT_ROOT%\external\soci\tmp_debug"
set "OUT_DIR=%PROJECT_ROOT%\lib64\debug"
if not exist "%TMP_DIR%" mkdir "%TMP_DIR%"
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

set "DBG_FLAGS=/MDd /Od /Zi /D_DEBUG /DDEBUG"

echo Compiling SOCI core...
for %%f in ("%SOCI_SRC%\core\*.cpp") do (
    echo   %%~nxf
    cl %COMMON_FLAGS% %DBG_FLAGS% %INCLUDE_CORE% /Fo"%TMP_DIR%\%%~nf.obj" "%%f"
    if errorlevel 1 goto :fail
)

echo.
echo Creating libsoci_core_3_2.lib...
lib /nologo /out:"%OUT_DIR%\libsoci_core_3_2.lib" "%TMP_DIR%\backend-loader.obj" "%TMP_DIR%\blob.obj" "%TMP_DIR%\connection-parameters.obj" "%TMP_DIR%\connection-pool.obj" "%TMP_DIR%\error.obj" "%TMP_DIR%\into-type.obj" "%TMP_DIR%\once-temp-type.obj" "%TMP_DIR%\prepare-temp-type.obj" "%TMP_DIR%\procedure.obj" "%TMP_DIR%\ref-counted-prepare-info.obj" "%TMP_DIR%\ref-counted-statement.obj" "%TMP_DIR%\row.obj" "%TMP_DIR%\rowid.obj" "%TMP_DIR%\session.obj" "%TMP_DIR%\statement.obj" "%TMP_DIR%\transaction.obj" "%TMP_DIR%\use-type.obj" "%TMP_DIR%\values.obj"
if errorlevel 1 goto :fail

echo.
echo Compiling SOCI PostgreSQL backend...
for %%f in ("%SOCI_SRC%\backends\postgresql\*.cpp") do (
    echo   %%~nxf
    cl %COMMON_FLAGS% %DBG_FLAGS% %INCLUDE_CORE% %INCLUDE_PG% /Fo"%TMP_DIR%\pg_%%~nf.obj" "%%f"
    if errorlevel 1 goto :fail
)

echo.
echo Creating libsoci_postgresql_3_2.lib...
lib /nologo /out:"%OUT_DIR%\libsoci_postgresql_3_2.lib" "%TMP_DIR%\pg_blob.obj" "%TMP_DIR%\pg_common.obj" "%TMP_DIR%\pg_error.obj" "%TMP_DIR%\pg_factory.obj" "%TMP_DIR%\pg_row-id.obj" "%TMP_DIR%\pg_session.obj" "%TMP_DIR%\pg_standard-into-type.obj" "%TMP_DIR%\pg_standard-use-type.obj" "%TMP_DIR%\pg_statement.obj" "%TMP_DIR%\pg_vector-into-type.obj" "%TMP_DIR%\pg_vector-use-type.obj"
if errorlevel 1 goto :fail

echo.
echo Debug libraries built successfully.

REM Clean debug temp
rd /s /q "%TMP_DIR%"

REM ============================================
REM  Build Release libraries
REM ============================================
echo.
echo === Building RELEASE SOCI libraries (x64) ===
echo.

set "TMP_DIR=%PROJECT_ROOT%\external\soci\tmp_release"
set "OUT_DIR=%PROJECT_ROOT%\lib64\release"
if not exist "%TMP_DIR%" mkdir "%TMP_DIR%"
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

set "REL_FLAGS=/MD /O2 /DNDEBUG"

echo Compiling SOCI core...
for %%f in ("%SOCI_SRC%\core\*.cpp") do (
    echo   %%~nxf
    cl %COMMON_FLAGS% %REL_FLAGS% %INCLUDE_CORE% /Fo"%TMP_DIR%\%%~nf.obj" "%%f"
    if errorlevel 1 goto :fail
)

echo.
echo Creating libsoci_core_3_2.lib...
lib /nologo /out:"%OUT_DIR%\libsoci_core_3_2.lib" "%TMP_DIR%\backend-loader.obj" "%TMP_DIR%\blob.obj" "%TMP_DIR%\connection-parameters.obj" "%TMP_DIR%\connection-pool.obj" "%TMP_DIR%\error.obj" "%TMP_DIR%\into-type.obj" "%TMP_DIR%\once-temp-type.obj" "%TMP_DIR%\prepare-temp-type.obj" "%TMP_DIR%\procedure.obj" "%TMP_DIR%\ref-counted-prepare-info.obj" "%TMP_DIR%\ref-counted-statement.obj" "%TMP_DIR%\row.obj" "%TMP_DIR%\rowid.obj" "%TMP_DIR%\session.obj" "%TMP_DIR%\statement.obj" "%TMP_DIR%\transaction.obj" "%TMP_DIR%\use-type.obj" "%TMP_DIR%\values.obj"
if errorlevel 1 goto :fail

echo.
echo Compiling SOCI PostgreSQL backend...
for %%f in ("%SOCI_SRC%\backends\postgresql\*.cpp") do (
    echo   %%~nxf
    cl %COMMON_FLAGS% %REL_FLAGS% %INCLUDE_CORE% %INCLUDE_PG% /Fo"%TMP_DIR%\pg_%%~nf.obj" "%%f"
    if errorlevel 1 goto :fail
)

echo.
echo Creating libsoci_postgresql_3_2.lib...
lib /nologo /out:"%OUT_DIR%\libsoci_postgresql_3_2.lib" "%TMP_DIR%\pg_blob.obj" "%TMP_DIR%\pg_common.obj" "%TMP_DIR%\pg_error.obj" "%TMP_DIR%\pg_factory.obj" "%TMP_DIR%\pg_row-id.obj" "%TMP_DIR%\pg_session.obj" "%TMP_DIR%\pg_standard-into-type.obj" "%TMP_DIR%\pg_standard-use-type.obj" "%TMP_DIR%\pg_statement.obj" "%TMP_DIR%\pg_vector-into-type.obj" "%TMP_DIR%\pg_vector-use-type.obj"
if errorlevel 1 goto :fail

echo.
echo Release libraries built successfully.

REM Clean release temp
rd /s /q "%TMP_DIR%"

REM --- Done ---
echo.
echo ============================================
echo  SOCI build complete!
echo  Output:
echo    lib64\debug\libsoci_core_3_2.lib
echo    lib64\debug\libsoci_postgresql_3_2.lib
echo    lib64\release\libsoci_core_3_2.lib
echo    lib64\release\libsoci_postgresql_3_2.lib
echo ============================================
goto :end

:fail
echo.
echo BUILD FAILED
echo.

:end
cd /d "%PROJECT_ROOT%"
pause
endlocal
