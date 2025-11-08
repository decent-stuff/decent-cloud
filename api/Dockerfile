# API Dockerfile - expects binary to be built natively first and then COPYed into the image
# Cloudflare Containers require linux/amd64 architecture
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

# Copy pre-built binary from host (assumes cargo build --target x86_64-unknown-linux-gnu)
COPY target/x86_64-unknown-linux-gnu/release/api-server /usr/local/bin/api-server

# Copy migrations
COPY api/migrations /migrations

# Create data directory and set permissions
RUN mkdir -p /data && \
    chown -R appuser:appuser /usr/local/bin/api-server /migrations /data

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

# Run the application
CMD ["api-server"]
