# Multi-stage Dockerfile for opencode-share application

# Stage 1: Builder
FROM rust:latest as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer is cached unless Cargo.toml changes)
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src/
COPY migrations ./migrations/
COPY static ./static/

# Build the application
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/opencode-share ./opencode-share

# Copy migrations and static files
COPY --from=builder /app/migrations ./migrations/
COPY --from=builder /app/static ./static/

# Create database file with proper permissions
RUN touch opencode-share.db && chown appuser:appuser opencode-share.db

# Change ownership to appuser
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 3006

# Set environment variables
ENV RUST_LOG=opencode_share=info,tower_http=info
ENV DATABASE_URL=sqlite:/app/opencode-share.db

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3006/ || exit 1

# Run the application
CMD ["./opencode-share"]