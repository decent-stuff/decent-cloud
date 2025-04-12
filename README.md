# Decent Cloud

A peer-to-peer platform for decentralized cloud resource sharing and management.

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](https://opensource.org/licenses/Apache-2.0)
![Version](https://img.shields.io/badge/version-0.3.0-blue)
[![Documentation](https://img.shields.io/badge/docs-latest-green.svg)](docs/)
[![Discussions](https://img.shields.io/github/discussions/decent-stuff/decent-cloud)](https://github.com/orgs/decent-stuff/discussions)

## 🌟 Overview

Decent Cloud revolutionizes cloud resource sharing by enabling anyone to rent or lease computing assets—from GPUs to web servers and web services—all through a single, unified interface. It eliminates vendor lock-in and streamlines cloud management with a decentralized approach.

What makes Decent Cloud truly stand out is its unique reputation system, which ties credibility directly to transaction fees. Paying these fees is the only way to build a strong reputation, and delivering poor service can quickly damage it—just as in real life. This tamper-proof ledger provides a transparent history of participant reputations, making it simple to identify reliable providers and maintain accountability across the platform.

### ✨ Key Features

- 🌐 **Peer-to-Peer Platform**: Rent or lease cloud resources directly from providers
- 🔄 **Unified Interface**: Single login for all cloud resources
- 🌿 **Green Technology**: No Proof of Work, environmentally friendly
- 🪙 **Fair Token Model**: DC Tokens minted every 10 minutes, similar to Bitcoin
- ⛏️ **Accessible Mining**: Easy-to-join validation system with regular rewards
- 🤝 **Community Driven**: Decisions made by the community, not VCs

## 📚 Documentation

- [Getting Started](docs/getting-started.md) - A quick introduction to Decent Cloud for new users, including basic setup steps.
- [Installation Guide](docs/installation.md) - Detailed platform installation instructions (OS specifics, requirements, etc.).
- [User Guide](docs/user-guide.md) - A comprehensive guide for end-users (developers, operators) on how to interact with the platform.
- [Provider Guide](docs/provider-guide.md) - In-depth documentation for providers on how to offer resources (e.g., servers, GPUs).
- [Reputation Guide](docs/reputation.md) - Explanation of the unique, blockchain-based reputation system.
- [Mining & Validation](docs/mining-and-validation.md) - Instructions on how to participate in securing the network and earning rewards.
- [Token Distribution](docs/token-distribution.md) - An outline of how tokens are created, rewarded, and circulated within the ecosystem.
- [Development Guide](docs/development.md) - Steps for contributors, including environment setup, testing, and best practices.

## 🚀 Quick Start

### Installation

<details>
<summary>Linux (Ubuntu 20.04+)</summary>

```bash
mkdir $HOME/bin
curl -L https://github.com/decent-stuff/decent-cloud/releases/latest/download/decent-cloud-linux-amd64 -o $HOME/bin/dc
chmod +x $HOME/bin/dc
```

Add to PATH in `~/.bashrc`:

```bash
if [ -d "$HOME/bin" ] ; then
   export PATH="$HOME/bin:$PATH"
fi
```

</details>

<details>
<summary>MacOS ARM64 (M1, M2, M3)</summary>

```bash
curl -L https://github.com/decent-stuff/decent-cloud/releases/latest/download/decent-cloud-darwin-arm64 -o /usr/local/bin/dc
chmod +x /usr/local/bin/dc
```

</details>

<details>
<summary>Windows</summary>

```powershell
$download_url = "https://github.com/decent-stuff/decent-cloud/releases/latest/download/decent-cloud-windows-amd64.exe"
Invoke-WebRequest "$download_url" -OutFile "dc.exe"
```

</details>

See the [Installation Guide](docs/installation.md) for detailed instructions.

### Basic Usage

1. Generate your identity:

```bash
dc keygen --generate --identity my-id
```

2. Register as a user, provider, or validator:

```bash
# For users
dc user register --identity my-user

# For providers/validators
dc np register --identity my-provider
```

See the [Getting Started Guide](docs/getting-started.md) for next steps, or check the [Mining & Validation Guide](docs/mining-and-validation.md) to learn how to earn rewards.

## 🔗 Related Projects

- **LedgerMap**: The backing ledger implementation

  - GitHub: [github.com/decent-stuff/ledger-map](https://github.com/decent-stuff/ledger-map/)
  - NPM Package: [@decent-stuff/ledger-map](https://www.npmjs.com/package/@decent-stuff/ledger-map)

- **Frontend Website**: The official Decent Cloud web interface

  - GitHub: [github.com/decent-stuff/website](https://github.com/decent-stuff/website/)

- **Decent-Cloud**: This repository, Internet Computer canister and the client code
  - Console Client: Available in this repository under `/cli` and published on GitHub as release binaries
  - Browser Client: [@decent-stuff/dc-client](https://www.npmjs.com/package/@decent-stuff/dc-client) NPM package

## 🌐 Project Status

The project is in active development. Main development and testing happens on Linux (Ubuntu 24.04), but MacOS and Windows versions work without issues.

If you encounter problems:

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
- 📚 [Read the Whitepaper](https://decent-cloud.org/)

## 🤝 Contributing

We welcome contributions! See our [Development Guide](docs/development.md) for:

- Setting up the development environment
- Building from source
- Running tests
- Contributing guidelines

## 📄 License

This project is licensed under the APACHE 2 License - see the [LICENSE](LICENSE) file for details.

---

⭐ If you find this project useful, please give it a star! ⭐

![Star the project](https://github.com/decent-stuff/decent-cloud/blob/main/images/star_img.png "Please star the project!")
