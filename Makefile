# Makefile for agent.nvim
# This provides an alternative build method

.PHONY: build clean install

# Default target
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
		echo "Build completed successfully!"; \
	else \
		echo "Error: Cargo not found. Please install Rust from https://rustup.rs/"; \
		exit 1; \
	fi

clean:
	@echo "Cleaning build artifacts..."
	@rm -rf target/ bin/
	@echo "Clean completed!"

install: build
	@echo "Build completed - binary available in bin/ directory"

# Help target
help:
	@echo "Available targets:"
	@echo "  build   - Build the Rust binary"
	@echo "  clean   - Clean build artifacts"
	@echo "  install - Build and install"
	@echo "  help    - Show this help"