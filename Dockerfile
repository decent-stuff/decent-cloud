# Dockerfile for Decent Cloud + Claude Code development environment
# Based on .github/container/Dockerfile with additions for Claude Code

FROM rust:latest

# Environment variables (from CI Dockerfile) - HOME must be /code for dfx
ENV HOME=/code \
    XDG_DATA_HOME=/usr/local \
    PATH=/usr/local/dfx/bin:/home/developer/.cargo/bin:$PATH \
    POCKET_IC_BIN=/usr/local/bin/pocket-ic \
    RUST_BACKTRACE=1

# Create working directory
RUN mkdir $HOME
WORKDIR $HOME

# Install dfx (exact from CI Dockerfile)
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

# Install deps (exact from CI Dockerfile)
RUN apt update && apt install -y libunwind-dev curl libssl-dev pkg-config

# Install Rust (exact from CI Dockerfile)
RUN rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown \
    && rustup toolchain install nightly --profile=complete

# Install cargo-make (exact from CI Dockerfile)
RUN cargo install cargo-make cargo-nextest wasm-pack

# Install UV (exact from CI Dockerfile)
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Install pocket-ic-server (exact from CI Dockerfile)
RUN curl -L https://github.com/dfinity/pocketic/releases/download/10.0.0/pocket-ic-x86_64-linux.gz -o - | gzip -d - > /usr/local/bin/pocket-ic && chmod +x /usr/local/bin/pocket-ic

# Install Node.js and npm (NEW addition)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt install -y nodejs

# Install Claude Code globally (NEW addition)
RUN npm install -g @anthropic-ai/claude-code

# Add tini for proper signal handling (from CI Dockerfile)
ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini

# Create non-root user for security (NEW addition)
RUN useradd -m -u 1000 developer

# Set ownership of directories for the developer user
RUN chown -R developer:developer $HOME /usr/local/cargo /usr/local/rustup /home/developer

# Switch to non-root user (NEW addition)
USER developer

# Set entrypoint to use tini for proper signal handling
ENTRYPOINT ["/tini", "--"]
