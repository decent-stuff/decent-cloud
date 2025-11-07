# Build stage
# Cloudflare Containers require linux/amd64 architecture
FROM --platform=linux/amd64 rust:1.91 as builder
WORKDIR /app

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
RUN cargo build --release --bin api-server

# Copy actual source code (this invalidates the dummy binary)
COPY api/src ./api/src
COPY api/migrations ./api/migrations

# Build the application (force rebuild by touching files)
RUN touch api/src/main.rs && cargo build --release --bin api-server

# Runtime stage
FROM --platform=linux/amd64 debian:bookworm-slim

# Build arguments for user/group IDs (defaults to 1000)
ARG USER_ID=1000
ARG GROUP_ID=1000

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user with configurable UID/GID for volume compatibility
RUN groupadd -g ${GROUP_ID} appuser && \
    useradd -r -u ${USER_ID} -g appuser -s /bin/false appuser

# Copy binary from builder stage
COPY --from=builder /app/target/release/api-server /usr/local/bin/api-server

# Create data directory and set permissions
RUN mkdir -p /data && \
    chown -R appuser:appuser /usr/local/bin/api-server /data

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

# Run the application
CMD ["api-server"]
