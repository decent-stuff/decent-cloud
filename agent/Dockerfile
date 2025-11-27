# Dockerfile for Decent Cloud + Containerized development environment
# Based on .github/container/Dockerfile with additions for Claude Code and Happy Coder

FROM rust:latest

# Environment variables (from CI Dockerfile)
ENV HOME=/home/ubuntu \
    XDG_DATA_HOME=/home/ubuntu/.cache/data \
    PATH=/home/ubuntu/.cache/data/dfx/bin:/home/ubuntu/.cargo/bin:/home/ubuntu/.local/bin:/home/ubuntu/bin:/home/ubuntu/.npm-global/bin:$PATH \
    POCKET_IC_BIN=/home/ubuntu/bin/pocket-ic \
    RUST_BACKTRACE=1

# Create working directory and non-root user
RUN useradd -m -u 1000 ubuntu && mkdir -p $HOME/.git/hooks $HOME/bin $XDG_DATA_HOME
WORKDIR $HOME

# Add tini for proper signal handling (from CI Dockerfile)
ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini

# Install dfx (exact from CI Dockerfile)
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)" \
    && dfxvm default 0.29.2

# Install deps (exact from CI Dockerfile)
RUN apt update && \
    apt install -y libunwind-dev curl libssl-dev pkg-config gzip build-essential ca-certificates less jq

# Install Node.js directly from NodeSource repository (simpler than NVM for Docker)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs

# Install UV (exact from CI Dockerfile)
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

RUN chown -R ubuntu:ubuntu $HOME /usr/local/cargo /usr/local/rustup

# Switch to non-root user
USER ubuntu

# Install pocket-ic-server (exact from CI Dockerfile)
RUN curl -L https://github.com/dfinity/pocketic/releases/download/10.0.0/pocket-ic-x86_64-linux.gz -o $HOME/bin/pocket-ic.gz \
    && gunzip $HOME/bin/pocket-ic.gz && chmod +x $HOME/bin/pocket-ic

# Install Rust (exact from CI Dockerfile)
RUN rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown \
    && rustup toolchain install nightly --profile=complete \
    && curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install cargo tools using binstall for faster installation
RUN cargo binstall cargo-make cargo-nextest

# Configure npm to use user-specific global directory and install Claude Code and Happy Coder
RUN mkdir -p ~/.npm-global \
    && npm config set prefix '~/.npm-global' \
    && echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.bashrc \
    && echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.profile \
    && export PATH=~/.npm-global/bin:$PATH \
    && npm install -g @anthropic-ai/claude-code happy-coder

# Configure git hooks
COPY --chown=ubuntu:ubuntu .git-hooks/ $HOME/.git-hooks/
RUN chmod +x $HOME/.git-hooks/commit-msg \
    && git config --global core.hooksPath $HOME/.git-hooks \
    && git config --global init.defaultBranch main

# Set entrypoint to use tini for proper signal handling
ENTRYPOINT ["/tini", "--"]
