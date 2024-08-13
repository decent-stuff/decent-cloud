# Use the official Rust image as a base
FROM rust:latest

ENV HOME=/code \
    XDG_DATA_HOME=/usr/local
RUN mkdir $HOME
WORKDIR $HOME

# Install dfx
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

ENV PATH=/usr/local/dfx/bin:$PATH

# Install deps
RUN apt update && apt install -y libunwind-dev

# Install Rust
RUN rustup toolchain install nightly \
    && rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown \
    && rustup component add rustfmt

# Install cargo-make
RUN cargo install cargo-make

# Install pocket-ic-server
RUN curl -L https://github.com/dfinity/pocketic/releases/download/5.0.0/pocket-ic-x86_64-linux.gz -o - | gzip -d - > /usr/local/bin/pocket-ic && chmod +x /usr/local/bin/pocket-ic

ENV POCKET_IC_BIN=/usr/local/bin/pocket-ic

ENV TINI_VERSION v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]
