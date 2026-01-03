# Decent Cloud

Decent Cloud is a peer-to-peer platform for decentralized cloud resource sharing and management.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](https://opensource.org/licenses/Apache-2.0)
![Version](https://img.shields.io/badge/version-0.4.0-blue)
[![Documentation](https://img.shields.io/badge/docs-latest-green.svg)](docs/)
[![Discussions](https://img.shields.io/github/discussions/decent-stuff/decent-cloud)](https://github.com/orgs/decent-stuff/discussions)

## Overview

Decent Cloud is a peer-to-peer marketplace where you can shop, compare, and order cloud resources—from GPUs to web services—through one unified platform. With no vendor lock-in and streamlined cloud management, our decentralized approach empowers both providers and users.

Trust verified through transparent track records, regular health checks, and provider security deposits. Our unique reputation system ties credibility to on-chain transaction history. Poor service quickly impacts reputation, while security deposits ensure providers have real financial accountability. This tamper-proof ledger guarantees transparency across the platform.

### Key Features

- **Peer-to-Peer Platform:** Rent or lease cloud resources directly.
- **Unified Interface:** Single login for all cloud resources.
- **Eco-Friendly:** No Proof of Work; environmentally responsible.
- **Fair Token Model:** DC Tokens minted every 10 minutes, similar to Bitcoin.
- **Accessible Mining:** Easy-to-join validation system with regular rewards.
- **Community-Driven:** Governance by community, not venture capital.

## Documentation

- [Getting Started](docs/getting-started.md) – A quick introduction for new users, outlining basic setup.
- [Web Interface Guide](docs/web-interface-guide.md) – Complete guide to using the web interface.
- [Installation](docs/installation.md) – Detailed installation instructions with OS specifics (CLI).
- [User Guide](docs/user-guide.md) – Comprehensive usage guide for consumers.
- [Reputation Guide](docs/reputation.md) – Explanation of our decentralized trust mechanism.
- [Mining & Validation](docs/mining-and-validation.md) – Steps to participate in network validation.
- [Token Distribution](docs/token-distribution.md) – Overview of token creation and rewards.
- [Development Guide](docs/development.md) – Guidelines for contributors, including setup, testing, and best practices.

## Requirements

### For Users

- Modern web browser (Chrome, Firefox, Safari, Edge)
- Stable internet connection

### For Contributors

- **PostgreSQL 14+** - Required for local development (see [Development Guide](docs/development.md))
- Rust toolchain (1.70+)
- Node.js 18+ (for frontend development)
- Docker and Docker Compose (recommended for local PostgreSQL)

## Quick Start

### Web Interface (Easiest)

Browse and rent resources without installing anything:

1. **Visit** [decent-cloud.org/dashboard](https://decent-cloud.org/dashboard)
2. **Browse** the marketplace - no account required
3. **Create account** when ready to rent resources

See [Web Interface Guide](docs/web-interface-guide.md) for details.

### CLI (Advanced Users)

For automation, scripting, and advanced features:

1. **Install** - See [Installation Guide](docs/installation.md) for detailed setup instructions
2. **Generate identity** - `dc keygen --generate --identity my-id`
3. **Register** - `dc user register --identity my-user` or `dc provider register --identity my-provider`

Continue with [Getting Started](docs/getting-started.md) for complete setup.

## Project Status

The project is in active development. Primary development occurs on Linux (Ubuntu 24.04), with best-effort support for MacOS and Windows.

If you encounter issues:
- [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
- [Read the Whitepaper](https://decent-cloud.org/)

## Contributing

Contributions are welcome! Refer to the [Development Guide](docs/development.md) for setup, building, testing, and contribution guidelines.

## License

This project is licensed under the APACHE 2 License. See the [LICENSE](LICENSE) file for details.

---

⭐ If you find this project useful, consider giving it a star!
