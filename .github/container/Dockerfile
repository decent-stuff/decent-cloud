# Use the official Rust image as a base
FROM rust:1.91

ENV HOME=/code \
    XDG_DATA_HOME=/usr/local \
    PATH=/usr/local/dfx/bin:$PATH \
    POCKET_IC_BIN=/usr/local/bin/pocket-ic \
    RUST_BACKTRACE=1

RUN mkdir $HOME
WORKDIR $HOME

# Install dfx
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

# Install deps
RUN apt update && \
    apt install -y libunwind-dev curl libssl-dev pkg-config gzip build-essential ca-certificates less jq clang mold sqlite3

# Install Rust
RUN rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown \
    && rustup toolchain install nightly-2025-11-27 --profile=complete \
    && curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install cargo tools using binstall for faster installation
RUN cargo binstall cargo-make cargo-nextest sqlx-cli

# Install UV
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Install pocket-ic-server
RUN curl -L https://github.com/dfinity/pocketic/releases/download/10.0.0/pocket-ic-x86_64-linux.gz -o - | gzip -d - > /usr/local/bin/pocket-ic && chmod +x /usr/local/bin/pocket-ic

ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]
