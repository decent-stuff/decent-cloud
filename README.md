# Install dependencies

We use `pixi` as a dependency manager for Python, and `cargo` for Rust.

Install `pixi` by following https://pixi.sh/latest/ -- it should be something along the lines of:

```bash
curl -fsSL https://pixi.sh/install.sh | bash
```

Install `cargo` by following https://rustup.rs/ -- it should be something along the lines of:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

To run end-to-end tests, we use [cargo-make](https://github.com/sagiegurari/cargo-make), which you can install by running

```bash
cargo install --force cargo-make
```

# Usage

## Key generation

### Option 1 (recommended): key generation with the `dc` cli tool:

For node provider:

```bash
cargo run --bin dc -- keygen --generate --identity np
```

For user:

```bash
cargo run --bin dc -- keygen --generate --identity user
```

### Option 2 (alternative): key generation with openssl:

For node provider:

```bash
mkdir -p $HOME/.dcc/identities/np
openssl genpkey -algorithm ED25519 -out $HOME/.dcc/identities/np/private.pem
```

For user:

```bash
mkdir -p $HOME/.dcc/identities/user
openssl genpkey -algorithm ED25519 -out $HOME/.dcc/identities/user/private.pem
```

# Running tests

You can run unit tests with:

```bash
cargo test
```

Or you can run the complete suite of unit tests and the canister tests with PocketIC, with:

```bash
cargo make
```

# Build whitepaper

There is a Python build script that uses a docker images with latex and mermaid.js to build the whitepaper PDF.

You can invoke the build script with:

```bash
pixi run python3 ./docs/whitepaper/build.py
```

The result PDF document will be at `build/docs/whitepaper/whitepaper.pdf`.

# Update CI image

There is a CI workflow that you can run to refresh the CI build image: https://github.com/decent-stuff/decent-cloud/actions/workflows/build-container-image.yaml

If that fails, you can build the image locally and push it manually.

```
docker build .github/container/ --tag ghcr.io/decent-stuff/decent-cloud/ci-image:latest
docker push ghcr.io/decent-stuff/decent-cloud/ci-image:latest
```

If `docker push` fails with `denied: denied` or similar error, refresh the ghcr token at https://github.com/settings/tokens?page=1 and run

```
docker login ghcr.io
username: yanliu38
password: <generated token>
```
