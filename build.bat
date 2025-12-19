@echo off
REM Build script for nvim-spec-agent on Windows
REM This script is called by lazy.nvim during plugin installation

echo === Building nvim-spec-agent ===

REM Check if Rust is installed
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Error: Rust/Cargo not found. Please install Rust from https://rustup.rs/
    echo    Download and run: https://win.rustup.rs/x86_64
    exit /b 1
)

echo ✅ Rust/Cargo found
cargo --version

REM Build the binary in release mode
echo 🔨 Building Rust binary...
cargo build --release --bin nvim-spec-agent
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Build failed! Check the error messages above.
    exit /b 1
)

REM Create bin directory if it doesn't exist
if not exist bin mkdir bin

REM Copy the binary to the bin directory
if exist target\release\nvim-spec-agent.exe (
    copy target\release\nvim-spec-agent.exe bin\
    if %ERRORLEVEL% NEQ 0 (
        echo ❌ Failed to copy binary!
        exit /b 1
    )
    echo ✅ Binary copied to: %CD%\bin\nvim-spec-agent.exe
) else (
    echo ❌ Binary not found at target\release\nvim-spec-agent.exe
    exit /b 1
)

echo 🎉 Build completed successfully!