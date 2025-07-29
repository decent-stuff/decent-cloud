# Use the official Rust image as a base
FROM rust:latest

ENV HOME=/code \
    XDG_DATA_HOME=/usr/local \
    PATH=/usr/local/dfx/bin:$PATH \
    POCKET_IC_BIN=/usr/local/bin/pocket-ic
RUN mkdir $HOME
WORKDIR $HOME

# Install dfx
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

# Install deps
RUN apt update && apt install -y libunwind-dev libssl-dev pkg-config

# Install Rust
RUN rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown \
    && rustup toolchain install nightly --profile=complete

# Install cargo-make
RUN cargo install cargo-make cargo-nextest wasm-pack

# Install pocket-ic-server
RUN curl -L https://github.com/dfinity/pocketic/releases/download/9.0.3/pocket-ic-x86_64-linux.gz -o - | gzip -d - > /usr/local/bin/pocket-ic && chmod +x /usr/local/bin/pocket-ic

ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]
