@echo off
setlocal

set PGROOT=%~dp0external\postgresql_server
set PGBIN=%PGROOT%\bin
set PGDATA=%PGROOT%\data
set PGPORT=5433
set PGLOG=%PGROOT%\pg.log

if "%1"=="" goto usage

if /i "%1"=="start" goto start
if /i "%1"=="stop" goto stop
if /i "%1"=="status" goto status
if /i "%1"=="init" goto init
if /i "%1"=="restart" goto restart
goto usage

:init
if exist "%PGDATA%\postgresql.conf" (
    echo Database already initialized at %PGDATA%
    exit /b 0
)
echo Initializing database...
"%PGBIN%\initdb.exe" -D "%PGDATA%" -U postgres -E UTF8 --locale=C -A trust
if errorlevel 1 (
    echo ERROR: initdb failed
    exit /b 1
)
echo Setting port to %PGPORT%...
echo port = %PGPORT% >> "%PGDATA%\postgresql.conf"
echo Database initialized. Run: db start
exit /b 0

:start
if not exist "%PGDATA%\postgresql.conf" (
    echo Database not initialized. Run: db init
    exit /b 1
)
echo Starting PostgreSQL on port %PGPORT%...
"%PGBIN%\pg_ctl.exe" -D "%PGDATA%" -l "%PGLOG%" -w start
if errorlevel 1 (
    echo ERROR: Failed to start. Check %PGLOG%
    exit /b 1
)
echo PostgreSQL started.
exit /b 0

:stop
echo Stopping PostgreSQL...
"%PGBIN%\pg_ctl.exe" -D "%PGDATA%" -w stop
if errorlevel 1 (
    echo ERROR: Failed to stop (maybe not running?)
    exit /b 1
)
echo PostgreSQL stopped.
exit /b 0

:restart
call :stop
call :start
exit /b 0

:status
"%PGBIN%\pg_ctl.exe" -D "%PGDATA%" status
exit /b 0

:usage
echo Usage: db [init^|start^|stop^|restart^|status]
echo.
echo   init     Initialize the database (first time only)
echo   start    Start PostgreSQL on port %PGPORT%
echo   stop     Stop PostgreSQL
echo   restart  Stop then start
echo   status   Check if running
exit /b 1
