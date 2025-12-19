@echo off
REM Build script for nvim-spec-agent on Windows
REM This script is called by lazy.nvim during plugin installation

echo Building nvim-spec-agent...

REM Check if Rust is installed
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo Error: Rust/Cargo not found. Please install Rust from https://rustup.rs/
    exit /b 1
)

REM Build the binary in release mode
cargo build --release --bin nvim-spec-agent
if %ERRORLEVEL% NEQ 0 (
    echo Build failed!
    exit /b 1
)

REM Create bin directory if it doesn't exist
if not exist bin mkdir bin

REM Copy the binary to the bin directory
copy target\release\nvim-spec-agent.exe bin\
if %ERRORLEVEL% NEQ 0 (
    echo Failed to copy binary!
    exit /b 1
)

echo Build completed successfully!
echo Binary location: %CD%\bin\nvim-spec-agent.exe