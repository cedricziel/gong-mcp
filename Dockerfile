# Build stage
FROM rust:1.90-slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml ./
COPY Cargo.lock ./

# Create a dummy source to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release && \
    rm -rf src target/release/gong-mcp* target/release/deps/gong_mcp*

# Copy actual source code
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
