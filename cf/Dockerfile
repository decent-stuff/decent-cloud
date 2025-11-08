# Multi-stage build for Decent Cloud website
# Builds Next.js static website and serves with nginx

# Stage 1a: Prepare cargo dependencies
FROM --platform=linux/amd64 rust:1.91-slim as cargo-planner
WORKDIR /app
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev curl && \
    rm -rf /var/lib/apt/lists/* && \
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash && \
    cargo binstall cargo-chef

# Copy workspace structure (only manifests, not source code)
# This layer is cached unless Cargo.toml files change
COPY Cargo.toml Cargo.lock ./
COPY api/Cargo.toml ./api/
COPY cli/Cargo.toml ./cli/
COPY common/Cargo.toml ./common/
COPY ic-canister/Cargo.toml ./ic-canister/
COPY ledger-map/Cargo.toml ./ledger-map/
COPY provider-json-search/Cargo.toml ./provider-json-search/
COPY provider-offering/Cargo.toml ./provider-offering/
COPY provider-profile/Cargo.toml ./provider-profile/
COPY wasm/Cargo.toml ./wasm/

# Copy setup script and create dummy source files (future-proof)
# Reads workspace members from Cargo.toml and creates appropriate dummy files
COPY cf/setup-cargo-workspace.sh ./
RUN chmod +x setup-cargo-workspace.sh && ./setup-cargo-workspace.sh

RUN cargo chef prepare --recipe-path recipe.json

# Stage 1b: Build Next.js website with Rust toolchain for WASM
FROM --platform=linux/amd64 rust:1.91-slim as web-builder
WORKDIR /app

# Install Node.js and build tools
RUN apt-get update && \
    apt-get install -y curl ca-certificates git pkg-config libssl-dev && \
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Install cargo-chef and wasm-pack
RUN cargo binstall cargo-chef wasm-pack

# Build Rust dependencies first (cached layer)
COPY --from=cargo-planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy Cargo workspace files
COPY Cargo.toml Cargo.lock ./
COPY api ./api
COPY cli ./cli
COPY common ./common
COPY ic-canister ./ic-canister
COPY ledger-map ./ledger-map
COPY provider-json-search ./provider-json-search
COPY provider-offering ./provider-offering
COPY provider-profile ./provider-profile

# Copy wasm package.json first for npm caching
COPY wasm/package*.json ./wasm/
COPY website/package*.json ./website/

# Install npm dependencies (cached if package.json unchanged)
RUN cd wasm && npm ci --ignore-scripts && \
    cd ../website && npm ci --ignore-scripts

# Copy WASM source in stages for better caching
# First copy Rust source files (most likely to change)
COPY wasm/src ./wasm/src
COPY wasm/*.ts ./wasm/
COPY wasm/*.js ./wasm/
COPY wasm/package.json ./wasm/

# Initialize git repo for build.js (needed for incremental builds)
RUN git init && git config user.email "docker@build" && git config user.name "Docker Build"

# Build WASM first (cached layer)
RUN cd wasm && npm run build

# Copy website source after WASM is built (separate layer)
COPY website ./website

# Build website (cached layer - only rebuilds if website source changes)
RUN cd website && npm run build

# Stage 2: Runtime with nginx
FROM --platform=linux/amd64 nginx:alpine

# Install runtime dependencies
RUN apk add --no-cache curl ca-certificates

# Copy nginx config
COPY <<EOF /etc/nginx/conf.d/default.conf
server {
    listen 3000;
    server_name _;

    # Serve static website
    root /usr/share/nginx/html;
    index index.html;

    # SPA routing - try files, fallback to index.html
    location / {
        try_files \$uri \$uri/ /index.html;
    }

    # Health check endpoint
    location /health {
        access_log off;
        return 200 "healthy";
        add_header Content-Type text/plain;
    }
}
EOF

# Copy website build
COPY --from=web-builder /app/website/out /usr/share/nginx/html

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
