# Use the official Rust image as a base
FROM rust:slim-bookworm

ENV PATH=/usr/local/dfx/bin:$PATH \
    POCKET_IC_BIN=/usr/local/bin/pocket-ic \
    HOME=/code \
    XDG_DATA_HOME=/usr/local
RUN mkdir $HOME
WORKDIR $HOME

# Install dfx
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

# Install system deps and Node.js
RUN apt update && apt install -y \
    libunwind-dev \
    ca-certificates \
    curl \
    gnupg \
    git \
    && mkdir -p /etc/apt/keyrings \
    && curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg \
    && echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_22.x nodistro main" | tee /etc/apt/sources.list.d/nodesource.list \
    && apt update \
    && apt install -y nodejs \
    && npm install -g npm@latest \
    && apt clean \
    && rm -rf /var/lib/apt/lists/*

# Install Rust and other tools
RUN rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown \
    && rustup toolchain install nightly --profile=complete \
    && cargo install cargo-make cargo-nextest wasm-pack

# Install pocket-ic-server
RUN curl -L https://github.com/dfinity/pocketic/releases/download/9.0.3/pocket-ic-x86_64-linux.gz -o - | gzip -d - > /usr/local/bin/pocket-ic && chmod +x /usr/local/bin/pocket-ic

# -------- source + npm deps layer ----------
# Note: this is not great because we add a lot of files to the image. But considering that we add all node_modules as well, and the repo is not yet so big, it's acceptable.
COPY . .

# We could also copy Cargo.toml / Cargo.lock but it's tricky because of dfx and the canister build

# Initialize git repository and configure git
# The custom build step in wasm/ expects to be in a git repo
RUN git config --global user.email "ci@example.com" && \
    git config --global user.name "CI" && \
    git init && \
    git add . && \
    git commit -m "Initial commit"

# NPM + Cargo caches persist across builds
RUN --mount=type=cache,target=/root/.npm \
    cd wasm && npm ci \
    && cd ../website && npm ci

# Optional: pre-fetch Rust crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo fetch

ENV TINI_VERSION=v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini
ENTRYPOINT ["/tini", "--"]
