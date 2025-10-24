# Build stage
FROM rust:1.90-slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
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

# Run the application
ENTRYPOINT ["/usr/local/bin/gong-mcp"]
