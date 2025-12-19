#!/bin/bash

# Build script for nvim-spec-agent
# This script is called by lazy.nvim during plugin installation

set -e

echo "Building nvim-spec-agent..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Build the binary in release mode
cargo build --release --bin nvim-spec-agent

# Create bin directory if it doesn't exist
mkdir -p bin

# Copy the binary to the bin directory
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    # Windows
    cp target/release/nvim-spec-agent.exe bin/
else
    # Unix-like systems (Linux, macOS)
    cp target/release/nvim-spec-agent bin/
fi

echo "Build completed successfully!"
echo "Binary location: $(pwd)/bin/nvim-spec-agent"