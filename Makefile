# Makefile for agent.nvim
# This provides a reliable cross-platform build method

.PHONY: all build clean install help

# Default target
all: build

build:
	@echo "Building nvim-spec-agent..."
	@if command -v cargo >/dev/null 2>&1; then \
		cargo build --release --bin nvim-spec-agent && \
		mkdir -p bin && \
		if [ -f target/release/nvim-spec-agent.exe ]; then \
			cp target/release/nvim-spec-agent.exe bin/; \
		else \
			cp target/release/nvim-spec-agent bin/ && chmod +x bin/nvim-spec-agent; \
		fi && \
		echo "Build completed successfully!" && \
		echo "Cleaning up build artifacts..." && \
		rm -rf target/ && \
		echo "Only keeping essential binary in bin/"; \
	else \
		echo "Error: Cargo not found. Please install Rust from https://rustup.rs/"; \
		exit 1; \
	fi

clean:
	@echo "Cleaning all build artifacts..."
	@rm -rf target/ bin/
	@echo "Clean completed!"

clean-target:
	@echo "Cleaning intermediate build files..."
	@rm -rf target/
	@echo "Target directory cleaned (keeping bin/)"

install: build
	@echo "Build completed - binary available in bin/ directory"

# Help target
help:
	@echo "Available targets:"
	@echo "  build   - Build the Rust binary"
	@echo "  clean   - Clean build artifacts"
	@echo "  install - Build and install"
	@echo "  help    - Show this help"