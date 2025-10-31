# Build stage
FROM rust:1.91-slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests and source
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/gong-mcp /usr/local/bin/gong-mcp

# Set environment variables (these can be overridden at runtime)
ENV GONG_BASE_URL=""
ENV GONG_ACCESS_KEY=""
ENV GONG_ACCESS_KEY_SECRET=""
ENV DOCKER_ENV="1"

# Expose HTTP port (only used in HTTP mode)
EXPOSE 8080

# Run the application
# Default to stdio mode for backward compatibility
# Override with: docker run ... gong-mcp --mode http --host 0.0.0.0 --port 8080
ENTRYPOINT ["/usr/local/bin/gong-mcp"]
CMD []
