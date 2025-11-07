# Development Guide

This guide covers everything you need to know about developing and contributing to the Decent Cloud project.

Note that the Decent Cloud project is split across three repositories:

- **LedgerMap**: The backing ledger implementation

  - GitHub: [github.com/decent-stuff/ledger-map](https://github.com/decent-stuff/ledger-map/)
  - NPM Package: [@decent-stuff/ledger-map](https://www.npmjs.com/package/@decent-stuff/ledger-map)

- **Decent-Cloud**: This repository, Internet Computer canister and the client code

  - Console Client: Available in this repository under `/cli` and published on GitHub as release binaries
  - Browser Client: [@decent-stuff/dc-client](https://www.npmjs.com/package/@decent-stuff/dc-client) NPM package

- **Frontend Website**: The official Decent Cloud web interface (located in `/website` directory)

## Development Environment Setup

### Prerequisites

1. **Essential Build Tools**

```bash
sudo apt update -y
sudo apt install build-essential curl git libssl-dev pkg-config
```

These packages are required for building the project:
- `build-essential`: Contains essential build tools like gcc, g++, and make
- `curl`: Used for downloading dependencies and tools
- `git`: Required for version control
- `libssl-dev`: OpenSSL development headers required by some Rust crates
- `pkg-config`: Tool for finding installed libraries and their compile flags

## Troubleshooting

### Build Failures

If you encounter build failures, especially related to OpenSSL or other system libraries, ensure you have installed all the required system dependencies listed above.

### Missing Tools

If the build process fails due to missing tools like `dfx` or `pocket-ic`, the build script will attempt to automatically install them. If this fails, you can install them manually using the instructions below in the appropriate sections.

2. **Internet Computer SDK (dfx)**

```bash
DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"
```

3. **Node.js Setup**

```bash
# Install NVM (Node Version Manager)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
source ~/.bashrc  # or restart your terminal

# Install and use Node.js 22
nvm install 22
nvm use 22

# Install required global npm packages
npm install --global jest rimraf
```

4. **Rust Setup**

Visit [rustup.rs](https://rustup.rs) or use:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install wasm-pack for WebAssembly support
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

5. **Cargo Make**

```bash
cargo install --force cargo-make
```

6. **Python Environment**
   Set up Python environment for the project:

```bash
python3 scripts/setup-python-env.py
```

This will create a virtual environment and install all necessary dependencies.

### Docker Setup for Volume Permissions

The API service uses Docker volumes to persist the SQLite database. To ensure proper file permissions, the container's `appuser` UID/GID can be configured at build time.

**Default Configuration:**
By default, the API container uses UID/GID 1000:1000, which matches most single-user Linux systems.

**Custom UID/GID:**
If your host data directory has different ownership (e.g., UID 1001), set environment variables before building:

```bash
# Check your data directory ownership
stat -c "UID: %u, GID: %g" data/api-data

# Set matching UID/GID for Docker build
export USER_ID=1001
export GROUP_ID=1001

# Rebuild the API container
docker compose -f cf/docker-compose.yml -f cf/docker-compose.prod.yml build api
```

The build arguments are defined in `cf/docker-compose.yml` and default to 1000 if not specified.

## Building the Project

### Building Binaries

#### Local Development Build

```bash
cargo build --release --bin dc
```

The binary will be available at `target/release/dc`

#### Cross-Platform Builds

For Windows builds, first install [cross](https://github.com/cross-rs/cross), then:

```bash
cross build --release --target x86_64-pc-windows-gnu
```

Example output binaries:

```
target/release/dc                               # Linux/MacOS binary
target/x86_64-pc-windows-gnu/release/dc.exe    # Windows binary
```

### Building the Website

The website is located in the `/website` directory and uses Next.js. The website depends on the WebAssembly (WASM) client package located in `/wasm`.

#### Prerequisites for Website Development

Ensure you have completed the Node.js setup from the prerequisites section above.

#### Website Development Workflow

The website build process has been optimized to automatically build the WASM package when needed, eliminating the need to publish to npmjs.com during development.

**For Production Build:**

```bash
cd website
npm install  # Install dependencies (first time only)
npm run build  # Automatically builds WASM package first, then website
```

**For Development (standard):**

```bash
cd website
npm install  # Install dependencies (first time only)
npm run dev  # Automatically builds WASM package first, then starts dev server
```

**For Development (with auto-rebuild on WASM changes):**

```bash
cd website
npm install  # Install dependencies (first time only)
npm run dev:watch  # Watches WASM files and rebuilds automatically
```

#### How the Build Process Works

1. **Automatic WASM Building**: The website's `prebuild` and `predev` scripts automatically build the WASM package before starting the website build or dev server.

2. **TypeScript Path Mapping**: The website's `tsconfig.json` is configured to import `@decent-stuff/dc-client` from the built WASM distribution (`../wasm/dist`) rather than the source files.

3. **Watch Mode**: The `dev:watch` script uses `nodemon` and `concurrently` to:
   - Watch for changes in WASM source files (`.rs`, `.ts`, `.js`)
   - Automatically rebuild the WASM package when changes are detected
   - Run the Next.js development server simultaneously

#### WASM Package Structure

The WASM package (`/wasm`) contains:

- **Source files**: TypeScript and Rust source code
- **Build script**: `build.js` that compiles Rust to WASM and TypeScript to JavaScript
- **Distribution**: `dist/` directory containing the built package ready for consumption

#### Troubleshooting Website Development

**Issue: Import errors for `@decent-stuff/dc-client`**

- Solution: Ensure the WASM package is built by running `cd wasm && npm run build`

**Issue: Changes to WASM code not reflected in website**

- Solution: Use `npm run dev:watch` instead of `npm run dev` for automatic rebuilding

**Issue: Build fails with missing dependencies**

- Solution: Run `npm install` in both `/wasm` and `/website` directories

## Testing

### Running Tests

1. **Unit Tests**

```bash
cargo test
```

2. **Complete Test Suite**
   Includes unit tests and canister tests using PocketIC:

```bash
cargo make
```

### Writing Tests

1. **Unit Test Guidelines**

- Place tests in a `tests` module
- Use meaningful test names
- Cover edge cases
- Add documentation

2. **Integration Test Guidelines**

- Test real-world scenarios
- Cover main workflows
- Test error conditions
- Document test setup

## Documentation

### Building Documentation

#### Whitepaper

```bash
# Activate virtual environment first (if not already activated)
source .venv/bin/activate  # On Linux/Mac
# or .venv\Scripts\activate  # On Windows

python3 ./docs/whitepaper/build.py
```

The PDF will be generated at `build/docs/whitepaper/whitepaper.pdf`

#### API Documentation

```bash
cargo doc --no-deps --open
```

### Documentation Guidelines

1. **Code Documentation**

- Document public interfaces
- Explain complex algorithms
- Include usage examples
- Note any limitations

2. **README Updates**

- Keep installation instructions current
- Document new features
- Update troubleshooting guides
- Maintain changelog

## CI/CD

### Local CI Image

Build the CI image locally:

```bash
docker build -f .github/container/Dockerfile --tag ghcr.io/decent-stuff/decent-cloud/ci-image:latest .
```

Push to registry:

```bash
docker push ghcr.io/decent-stuff/decent-cloud/ci-image:latest
```

If push fails with "denied":

1. Get new token from [GitHub settings/tokens](https://github.com/settings/tokens?page=1)
2. Login:

```bash
docker login ghcr.io
username: <your-username>
password: <generated token>
```

### GitHub Actions

- CI workflow runs on all PRs
- Release workflow triggers on tags
- Container image workflow can be manually triggered
- Cloudflare Pages deployment workflow runs on pushes to main branch (after successful tests)

### Cloudflare Pages Deployment

The website is automatically deployed to Cloudflare Pages on successful pushes to the main branch.
The deployment workflow:

1. Runs all tests (website tests and Rust tests)
2. Builds the WASM package
3. Builds the Next.js website using `@cloudflare/next-on-pages`
4. Deploys to Cloudflare Pages

See [Cloudflare Deployment Setup](cloudflare-deployment.md) for detailed instructions on setting up
the required secrets and Cloudflare Pages project.

## Contributing

### Getting Started

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

### Contribution Guidelines

1. **Code Style**

- Follow Rust style guidelines
- Use meaningful variable names
- Keep functions focused
- Add appropriate comments

2. **Commit Messages**

- Use clear, descriptive messages
- Reference issues when applicable
- Follow conventional commits

3. **Pull Requests**

- Describe changes thoroughly
- Include test coverage
- Update documentation
- Add to changelog

### Review Process

1. **Before Submitting**

- Run all tests
- Update documentation
- Check formatting
- Verify CI passes

2. **During Review**

- Respond to feedback
- Make requested changes
- Keep discussion professional
- Update as needed

## Project Structure

```
decent-cloud/
├── cli/            # Command-line interface
├── common/         # Shared utilities
├── ic-canister/    # Internet Computer canister
├── ledger-map/     # A secure, persistent key-value storage
├── wasm/           # WebAssembly client package
│   ├── src/        # Rust source code
│   ├── dist/       # Built package (auto-generated)
│   ├── build.js    # Build script
│   └── *.ts        # TypeScript source files
├── website/        # Next.js frontend website
│   ├── app/        # Next.js app directory
│   ├── components/ # React components
│   ├── lib/        # Utility libraries
│   └── public/     # Static assets
├── provider-offering/    # Node provider offering
├── provider-profile/     # Node provider profile
├── provider-json-search/ # JSON search functionality
├── docs/           # Documentation
└── examples/       # Example configurations
```

## Support

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
- 📚 [Read the Whitepaper](https://decent-cloud.org/)

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](../LICENSE) file for details.
