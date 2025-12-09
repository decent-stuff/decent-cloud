# API Dockerfile - expects binary to be built natively first and then COPYed into the image
# Cloudflare Containers require linux/amd64 architecture
FROM --platform=linux/amd64 debian:trixie-slim

# Build arguments for user/group IDs (defaults to 1000)
ARG USER_ID=1000
ARG GROUP_ID=1000

# Install runtime dependencies and Typst CLI (for invoice PDF generation)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    sqlite3 \
    xz-utils \
    && curl -L https://github.com/typst/typst/releases/latest/download/typst-x86_64-unknown-linux-musl.tar.xz | tar -xJ -C /usr/local/bin --strip-components=1 typst-x86_64-unknown-linux-musl/typst \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user with configurable UID/GID for volume compatibility
RUN groupadd -g ${GROUP_ID} appuser && \
    useradd -r -u ${USER_ID} -g appuser -s /bin/false appuser

# Cache invalidation: binary hash changes when binary changes for ANY reason
# This ensures Docker rebuilds when the binary changes due to:
# - Migration changes (embedded at compile time via sqlx::migrate!)
# - Code changes (bug fixes, features, refactoring)
# - Dependency updates
ARG BINARY_HASH=unknown
RUN echo "API binary hash: ${BINARY_HASH}"

# Copy pre-built binaries from host (built with --profile release)
COPY target/x86_64-unknown-linux-gnu/release/api-server /usr/local/bin/api-server
COPY target/x86_64-unknown-linux-gnu/release/dc /usr/local/bin/dc

# Copy migrations and validation script
COPY api/migrations /migrations
COPY cf/validator-entrypoint.sh /usr/local/bin/validator-entrypoint.sh

# Create data directory and set permissions
RUN mkdir -p /data /identity && \
    chown -R appuser:appuser /usr/local/bin/api-server /usr/local/bin/dc /usr/local/bin/validator-entrypoint.sh /migrations /data /identity && \
    chmod +x /usr/local/bin/validator-entrypoint.sh

# Set Typst cache directory for package downloads (uses mounted /data volume)
ENV XDG_CACHE_HOME=/data

# Switch to non-root user
USER appuser

# Run the application
CMD ["api-server"]
