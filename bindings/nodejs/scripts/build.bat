@echo off
REM Build script for Kizuna Node.js bindings on Windows

setlocal enabledelayedexpansion

echo Building Kizuna Node.js bindings...
echo.

REM Check if napi-rs CLI is installed
where napi >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo napi CLI not found. Installing...
    call npm install -g @napi-rs/cli
)

REM Determine build mode
set BUILD_MODE=%1
if "%BUILD_MODE%"=="" set BUILD_MODE=release

if "%BUILD_MODE%"=="debug" (
    set BUILD_FLAG=
    echo Building in debug mode
) else (
    set BUILD_FLAG=--release
    echo Building in release mode
)

REM Navigate to project root
cd /d "%~dp0..\..\..\"

REM Build the native module
echo.
echo Compiling Rust code...
cargo build --features nodejs %BUILD_FLAG%
if %ERRORLEVEL% NEQ 0 (
    echo Build failed!
    exit /b 1
)

REM Copy the built library
echo.
echo Copying native module...
if "%BUILD_MODE%"=="debug" (
    set BUILD_DIR=target\debug
) else (
    set BUILD_DIR=target\release
)

if exist "%BUILD_DIR%\kizuna.dll" (
    copy /Y "%BUILD_DIR%\kizuna.dll" "bindings\nodejs\kizuna.node"
    echo Native module copied to bindings\nodejs\kizuna.node
) else (
    echo Native module not found at %BUILD_DIR%\kizuna.dll
    exit /b 1
)

echo.
echo Build complete!
echo You can now test the bindings with: npm test

endlocal
