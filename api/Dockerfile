# Build stage
# Cloudflare Containers require linux/amd64 architecture
FROM --platform=linux/amd64 rust:1.91 as builder
WORKDIR /app

# Install musl target for static binary
RUN rustup target add x86_64-unknown-linux-musl && \
    apt-get update && \
    apt-get install -y musl-tools musl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy workspace Cargo files
COPY Cargo.toml Cargo.lock ./
COPY api/Cargo.toml ./api/
COPY cli ./cli
COPY common ./common
COPY ic-canister ./ic-canister
COPY ledger-map ./ledger-map
COPY provider-json-search ./provider-json-search
COPY provider-offering ./provider-offering
COPY provider-profile ./provider-profile
COPY wasm ./wasm

# Create dummy main.rs to cache dependencies
RUN mkdir -p api/src && echo "fn main() {}" > api/src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl --bin api-server
RUN rm -rf api/src

# Copy actual source code
COPY api/src ./api/src
COPY api/migrations ./api/migrations

# Build the application
RUN cargo build --release --target x86_64-unknown-linux-musl --bin api-server

# Runtime stage
FROM --platform=linux/amd64 debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false appuser

# Copy binary from builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/api-server /usr/local/bin/api-server

# Set permissions
RUN chown appuser:appuser /usr/local/bin/api-server

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

# Run the application
CMD ["api-server"]
