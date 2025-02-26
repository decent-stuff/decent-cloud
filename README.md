# Decent Cloud

A peer-to-peer platform for decentralized cloud resource sharing and management.

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](https://opensource.org/licenses/Apache-2.0)
![Version](https://img.shields.io/badge/version-0.2.0-blue)
[![Documentation](https://img.shields.io/badge/docs-latest-green.svg)](docs/)
[![Discussions](https://img.shields.io/github/discussions/decent-stuff/decent-cloud)](https://github.com/orgs/decent-stuff/discussions)

## 🌟 Overview

Decent Cloud revolutionizes cloud resource sharing by enabling anyone to rent out or lease cloud resources—from GPUs to web servers—through a unified interface. It eliminates vendor lock-in and simplifies cloud resource management through a decentralized approach.

### ✨ Key Features

- 🌐 **Peer-to-Peer Platform**: Rent or lease cloud resources directly from providers
- 🔄 **Unified Interface**: Single login for all cloud resources
- 🌿 **Green Technology**: No Proof of Work, environmentally friendly
- 🪙 **Fair Token Model**: DC Tokens minted every 10 minutes, similar to Bitcoin
- ⛏️ **Accessible Mining**: Easy-to-join validation system with regular rewards
- 🤝 **Community Driven**: Decisions made by the community, not VCs

## 📚 Documentation

- [Installation Guide](docs/installation.md) - Platform installation instructions
- [Getting Started](docs/getting-started.md) - Quick start guide
- [User Guide](docs/user-guide.md) - Comprehensive guide for users
- [Provider Guide](docs/provider-guide.md) - Complete guide for providers
- [Mining & Validation](docs/mining-and-validation.md) - How to participate and earn rewards
- [Token Distribution](docs/token-distribution.md) - Token system explanation
- [Development Guide](docs/development.md) - Contributing and development setup

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

- **Client Libraries**:
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
