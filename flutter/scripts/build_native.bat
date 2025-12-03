@echo off
REM Build script for Kizuna native libraries on Windows

setlocal enabledelayedexpansion

echo [INFO] Starting native library build process...

REM Check if Rust is installed
where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Rust is not installed. Please install from https://rustup.rs/
    exit /b 1
)

REM Check if flutter_rust_bridge_codegen is installed
where flutter_rust_bridge_codegen >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [WARN] flutter_rust_bridge_codegen not found. Installing...
    cargo install flutter_rust_bridge_codegen
)

REM Navigate to project root
cd /d "%~dp0..\.."

REM Generate Flutter Rust Bridge bindings
echo [INFO] Generating Flutter Rust Bridge bindings...
flutter_rust_bridge_codegen ^
    --rust-input src/developer_api/bindings/flutter.rs ^
    --dart-output flutter/lib/src/bridge_generated.dart ^
    --dart-decl-output flutter/lib/src/bridge_definitions.dart

REM Add Windows target if not already added
rustup target add x86_64-pc-windows-msvc 2>nul

REM Build for Windows
echo [INFO] Building for Windows x86_64...
cargo build --release --target x86_64-pc-windows-msvc --features flutter

REM Copy library to Flutter plugin
echo [INFO] Copying Windows library...
if not exist "flutter\windows" mkdir "flutter\windows"
copy /Y "target\x86_64-pc-windows-msvc\release\kizuna.dll" "flutter\windows\"

echo [INFO] Windows build complete!
echo [INFO] Native library is ready in the flutter\windows directory

endlocal
