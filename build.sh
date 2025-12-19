#!/bin/bash

# Build script for nvim-spec-agent
# This script is called by lazy.nvim during plugin installation

set -e

echo "=== Building nvim-spec-agent ==="

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust/Cargo not found. Please install Rust from https://rustup.rs/"
    echo "   On most systems: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✅ Rust/Cargo found: $(cargo --version)"

# Build the binary in release mode
echo "🔨 Building Rust binary..."
if ! cargo build --release --bin nvim-spec-agent; then
    echo "❌ Build failed! Check the error messages above."
    exit 1
fi

# Create bin directory if it doesn't exist
mkdir -p bin

# Copy the binary to the bin directory
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    # Windows
    if [ -f "target/release/nvim-spec-agent.exe" ]; then
        cp target/release/nvim-spec-agent.exe bin/
        echo "✅ Binary copied to: $(pwd)/bin/nvim-spec-agent.exe"
    else
        echo "❌ Binary not found at target/release/nvim-spec-agent.exe"
        exit 1
    fi
else
    # Unix-like systems (Linux, macOS)
    if [ -f "target/release/nvim-spec-agent" ]; then
        cp target/release/nvim-spec-agent bin/
        chmod +x bin/nvim-spec-agent
        echo "✅ Binary copied to: $(pwd)/bin/nvim-spec-agent"
    else
        echo "❌ Binary not found at target/release/nvim-spec-agent"
        exit 1
    fi
fi

echo "🎉 Build completed successfully!"

# Clean up build artifacts to save space
echo "🧹 Cleaning up build artifacts..."
rm -rf target/
echo "✅ Cleanup completed - only keeping essential binary"