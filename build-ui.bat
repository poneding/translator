@echo off
cd /d "%~dp0ui"
call npm run build
exit /b %ERRORLEVEL%
