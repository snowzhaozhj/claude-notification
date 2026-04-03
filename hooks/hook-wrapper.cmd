@echo off
rem hook-wrapper.cmd — Claude Notification Plugin hook dispatcher (Windows)
rem Checks for the binary and runs it, forwarding all arguments.

setlocal EnableDelayedExpansion

rem ---------------------------------------------------------------------------
rem Resolve CLAUDE_PLUGIN_ROOT
rem ---------------------------------------------------------------------------
set "SCRIPT_DIR=%~dp0"
rem Remove trailing backslash from SCRIPT_DIR
if "%SCRIPT_DIR:~-1%"=="\" set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

rem Parent of hooks\ is the plugin root
for %%I in ("%SCRIPT_DIR%\..") do set "PLUGIN_ROOT=%%~fI"

if not defined CLAUDE_PLUGIN_ROOT (
  set "CLAUDE_PLUGIN_ROOT=%PLUGIN_ROOT%"
)

rem ---------------------------------------------------------------------------
rem Detect ARCH
rem ---------------------------------------------------------------------------
set "ARCH=x86_64"
if /I "%PROCESSOR_ARCHITECTURE%"=="ARM64" set "ARCH=aarch64"
if /I "%PROCESSOR_ARCHITEW6432%"=="ARM64" set "ARCH=aarch64"

set "BINARY_NAME=claude-notify-windows-%ARCH%.exe"
set "BINARY_PATH=%CLAUDE_PLUGIN_ROOT%\bin\%BINARY_NAME%"

rem ---------------------------------------------------------------------------
rem Check binary exists
rem ---------------------------------------------------------------------------
if not exist "%BINARY_PATH%" (
  echo [claude-notification] Binary not found at: %BINARY_PATH% 1>&2
  echo [claude-notification] Please build from source or download from GitHub Releases. 1>&2
  echo [claude-notification] See: https://github.com/zhaohejie/claude-notification-plugin/releases 1>&2
  exit /b 0
)

rem ---------------------------------------------------------------------------
rem Run binary, forwarding all args (stdin is passed through automatically)
rem ---------------------------------------------------------------------------
"%BINARY_PATH%" %*
