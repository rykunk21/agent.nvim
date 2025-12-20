# Multi-stage build for Rust-based MCP orchestration layer
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /build

# Copy Cargo files
COPY container/Cargo.toml container/Cargo.lock* ./

# Copy source code
COPY container/src ./src

# Build the application
RUN cargo build --release

# Production stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/mcp-orchestration /app/mcp-orchestration

# Copy configuration files
COPY container/config/ /app/config/

# Create non-root user for security
RUN useradd --create-home --shell /bin/bash agent
RUN chown -R agent:agent /app
USER agent

# Expose gRPC port
EXPOSE 50051

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:50051/health || exit 1

# Set environment variables
ENV GRPC_PORT=50051
ENV LOG_LEVEL=info
ENV RUST_LOG=info

# Start the MCP orchestration layer
CMD ["/app/mcp-orchestration"]