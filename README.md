# Install dependencies

We use `pixi` for Python, and `cargo` for Rust.

Install `pixi` by following https://pixi.sh/latest/  -- it should be something along the lines of:

```bash
curl -fsSL https://pixi.sh/install.sh | bash
```

Install `cargo` by following https://rustup.rs/ -- it should be something along the lines of:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

# Running tests

Cargo tests

```bash
cargo test
```

Pytest

```bash
pixi run pytest
```

# Build whitepaper

Whitepaper is still built with buck2.
This may change since buck2 is a lot of work to maintain in contrast with plain cargo.

```bash
buck2 build //docs/whitepaper:whitepaper
```

The generated pdf can be found by running

```bash
buck2 targets --show-output //docs/whitepaper:whitepaper 2>/dev/null | grep out/whitepaper.pdf | awk '{print $2}'
```

# Usage

## Key generation

### Option 1: key generation with the `dc` cli tool:

For node provider:
```bash
cargo run --bin dc -- keygen --generate --identity np
```

For user:
```bash
cargo run --bin dc -- keygen --generate --identity user
```

### Option 2: key generation with openssl:

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
