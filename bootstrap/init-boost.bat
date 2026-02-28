@echo off
cls
cd /d "%~dp0..\external\boost"
cd tools\build\v2
bootstrap
pause
