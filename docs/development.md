# Development Guide

This guide covers everything you need to know about developing and contributing to the Decent Cloud project.

Note that the Decent Cloud project is split across three repositories:

- **LedgerMap**: The backing ledger implementation

  - GitHub: [github.com/decent-stuff/ledger-map](https://github.com/decent-stuff/ledger-map/)
  - NPM Package: [@decent-stuff/ledger-map](https://www.npmjs.com/package/@decent-stuff/ledger-map)

- **Decent-Cloud**: This repository, Internet Computer canister and the client code

  - Console Client: Available in this repository under `/cli` and published on GitHub as release binaries
  - Browser Client: [@decent-stuff/dc-client](https://www.npmjs.com/package/@decent-stuff/dc-client) NPM package

- **Frontend Website**: The official Decent Cloud web interface

  - GitHub: [github.com/decent-stuff/website](https://github.com/decent-stuff/website/)

## Development Environment Setup

### Prerequisites

1. **Essential Build Tools**

```bash
sudo apt update -y
sudo apt install build-essential curl git
```

2. **Rust Setup**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

3. **Cargo Make**

```bash
cargo install --force cargo-make
```

4. **Python Environment**
   Install `pixi` for Python dependency management:

```bash
curl -fsSL https://pixi.sh/install.sh | bash
```

After installation, set up project dependencies:

```bash
pixi install
```

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
pixi run python3 ./docs/whitepaper/build.py
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
docker build .github/container/ --tag ghcr.io/decent-stuff/decent-cloud/ci-image:latest
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
├── np-offering/    # Node provider offering
├── np-profile/     # Node provider profile
├── np-json-search/ # JSON search functionality
├── docs/           # Documentation
└── examples/       # Example configurations
```

## Support

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
- 📚 [Read the Whitepaper](https://decent-cloud.org/)

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
