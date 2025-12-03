# Multi-stage build for minimal container image
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig

WORKDIR /build

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src

# Build application
RUN cargo build --release --bin kizuna

# Runtime stage
FROM alpine:latest

# Metadata
LABEL maintainer="Kizuna Team"
LABEL version="0.1.0"
LABEL description="Kizuna - Cross-platform connectivity solution"

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc

# Create non-root user
RUN addgroup -g 1000 kizuna && \
    adduser -D -u 1000 -G kizuna kizuna

# Create application directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder --chown=kizuna:kizuna /build/target/release/kizuna /app/kizuna
RUN chmod +x /app/kizuna

# Switch to non-root user
USER kizuna

# Expose ports
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/app/kizuna", "health"] || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV KIZUNA_PORT=8080

# Entry point
ENTRYPOINT ["/app/kizuna"]
CMD ["serve"]
