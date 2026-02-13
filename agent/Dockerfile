# Dockerfile for Decent Cloud + Containerized development environment
# Based on .github/container/Dockerfile with additions for Claude Code, Happy Coder, and OpenCode

FROM rust:1.91

# Environment variables (from CI Dockerfile)
ENV HOME=/home/ubuntu \
    XDG_DATA_HOME=/home/ubuntu/.cache/data \
    PATH=/usr/lib/postgresql/17/bin:/home/ubuntu/.cache/data/dfx/bin:/home/ubuntu/.cargo/bin:/home/ubuntu/.local/bin:/home/ubuntu/.opencode/bin:/home/ubuntu/bin:/home/ubuntu/.npm-global/bin:/home/ubuntu/.venv/bin:$PATH \
    POCKET_IC_BIN=/home/ubuntu/bin/pocket-ic \
    RUST_BACKTRACE=1 \
    VIRTUAL_ENV=/home/ubuntu/.venv

# Create working directory and non-root user
RUN useradd -m -u 1000 ubuntu && mkdir -p $HOME/.git/hooks $HOME/bin $XDG_DATA_HOME
WORKDIR $HOME

# Add tini for proper signal handling
ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini

# Install dfx
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)" \
    && dfxvm default 0.29.2

# Install deps
RUN apt update && \
    apt install -y libunwind-dev curl libssl-dev pkg-config gzip build-essential ca-certificates less jq clang mold sqlite3 curl gosu postgresql \
    # Docker CLI for running docker commands inside the container
    && install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc \
    && chmod a+r /etc/apt/keyrings/docker.asc \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian bookworm stable" > /etc/apt/sources-docker.list \
    && cat /etc/apt/sources-docker.list >> /etc/apt/sources.list.d/docker.list \
    && apt update && apt install -y docker-ce-cli docker-compose-plugin

# Install Node.js directly from NodeSource repository (simpler than NVM for Docker)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs

# Install pip
RUN apt install -y python3-pip python3-venv

RUN chown -R ubuntu:ubuntu $HOME /usr/local/cargo /usr/local/rustup

# Install Playwright system dependencies (requires root)
RUN npx playwright install-deps chromium && chown -R ubuntu:ubuntu /home/ubuntu

# Switch to non-root user
USER ubuntu

# Install pocket-ic-server
RUN curl -L https://github.com/dfinity/pocketic/releases/download/10.0.0/pocket-ic-x86_64-linux.gz -o $HOME/bin/pocket-ic.gz \
    && gunzip $HOME/bin/pocket-ic.gz && chmod +x $HOME/bin/pocket-ic

# Install Rust
RUN rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown \
    && rustup toolchain install nightly-2025-11-27 --profile=complete \
    && curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install cargo tools using binstall for faster installation
RUN cargo binstall cargo-make cargo-nextest sqlx-cli ripgrep cargo-sweep typst-cli

# Install UV to .local/bin (not .cargo/bin which gets shadowed by cargo-cache volume)
RUN curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR="$HOME/.local/bin" sh

# Create Python virtual environment and install common packages
# This allows pip install to work without --break-system-packages
RUN $HOME/.local/bin/uv venv $HOME/.venv \
    && $HOME/.local/bin/uv pip install python-dotenv pytest black ruff

# Configure npm to use user-specific global directory and install Claude Code and Happy Coder
RUN mkdir -p ~/.npm-global \
    && npm config set prefix '~/.npm-global' \
    && echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.bashrc \
    && echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.profile \
    && export PATH=~/.npm-global/bin:$PATH \
    && npm install -g  @anthropic-ai/claude-code happy-coder playwright \
    && npx playwright install chromium

# Install OpenCode
RUN curl -fsSL https://opencode.ai/install | bash

# Configure git hooks
COPY --chown=ubuntu:ubuntu .git-hooks/ $HOME/.git-hooks/
RUN chmod +x $HOME/.git-hooks/commit-msg \
    && git config --global core.hooksPath $HOME/.git-hooks \
    && git config --global init.defaultBranch main

# Copy entrypoint script (runs as root, fixes perms, drops to ubuntu)
USER root
COPY agent/entrypoint.sh $HOME/entrypoint.sh
RUN chmod +x $HOME/entrypoint.sh

# Default entrypoint uses tini for signal handling (compose overrides with entrypoint.sh)
ENTRYPOINT ["/tini", "--"]
