# Decent Cloud

Decent Cloud is a peer-to-peer platform for decentralized cloud resource sharing and management.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](https://opensource.org/licenses/Apache-2.0)
![Version](https://img.shields.io/badge/version-0.4.0-blue)
[![Documentation](https://img.shields.io/badge/docs-latest-green.svg)](docs/)
[![Discussions](https://img.shields.io/github/discussions/decent-stuff/decent-cloud)](https://github.com/orgs/decent-stuff/discussions)

## Overview

Decent Cloud transforms cloud resource sharing by enabling anyone to rent or lease computing assets—from GPUs to web services—through a single, unified interface. With no vendor lock-in and streamlined cloud management, our decentralized approach empowers both providers and users.

Our unique reputation system ties credibility to transaction fees. Paying these fees is essential for building a trustworthy history, while poor service quickly impacts reputation. This tamper-proof ledger ensures transparency and accountability across the platform.

### Key Features

- **Peer-to-Peer Platform:** Rent or lease cloud resources directly.
- **Unified Interface:** Single login for all cloud resources.
- **Eco-Friendly:** No Proof of Work; environmentally responsible.
- **Fair Token Model:** DC Tokens minted every 10 minutes, similar to Bitcoin.
- **Accessible Mining:** Easy-to-join validation system with regular rewards.
- **Community-Driven:** Governance by community, not venture capital.

## Documentation

- [Getting Started](docs/getting-started.md) – A quick introduction for new users, outlining basic setup.
- [Installation](docs/installation.md) – Detailed installation instructions with OS specifics.
- [User Guide](docs/user-guide.md) – Comprehensive usage guide for consumers.
- [Provider Guide](docs/provider-guide.md) – Detailed information for resource providers.
- [Reputation Guide](docs/reputation.md) – Explanation of our decentralized trust mechanism.
- [Mining & Validation](docs/mining-and-validation.md) – Steps to participate in network validation.
- [Token Distribution](docs/token-distribution.md) – Overview of token creation and rewards.
- [Development Guide](docs/development.md) – Guidelines for contributors, including setup, testing, and best practices.

## Quick Start

### Installation

<details>
<summary>Linux (Ubuntu 24.04+)</summary>

```bash
mkdir $HOME/bin
curl -L https://github.com/decent-stuff/decent-cloud/releases/latest/download/decent-cloud-linux-amd64 -o $HOME/bin/dc
chmod +x $HOME/bin/dc
```

Add to PATH in `~/.bashrc`:

```bash
if [ -d "$HOME/bin" ]; then
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

See the [Installation](docs/installation.md) guide for detailed instructions.

### Basic Usage

1. Generate your identity:

```bash
dc keygen --generate --identity my-id
```

2. Register according to role:

```bash
# For users
dc user register --identity my-user

# For providers/validators
dc provider register --identity my-provider
```

Continue with [Getting Started](docs/getting-started.md) or [Mining & Validation](docs/mining-and-validation.md) for next steps.

## Related Projects

- **LedgerMap:** Backing ledger implementation  
  - GitHub: [github.com/decent-stuff/ledger-map](https://github.com/decent-stuff/ledger-map/)  
  - NPM: [@decent-stuff/ledger-map](https://www.npmjs.com/package/@decent-stuff/ledger-map)
- **Frontend Website:** Official Decent Cloud web interface  
  - GitHub: [github.com/decent-stuff/website](https://github.com/decent-stuff/website/)
- **Decent-Cloud:** Internet Computer canister and client code  
  - CLI Client in `/cli`, and published as binaries.  
  - Browser Client: [@decent-stuff/dc-client](https://www.npmjs.com/package/@decent-stuff/dc-client)

## Project Status

Active development is underway. Primary development occurs on Linux (Ubuntu 24.04), with support for MacOS and Windows.

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
