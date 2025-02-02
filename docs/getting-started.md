# Getting Started with Decent Cloud

This guide will help you get up and running with Decent Cloud quickly.

## Prerequisites

- Decent Cloud CLI installed (see [Installation Guide](installation.md))
- Basic understanding of cloud resources and command-line interfaces

## Quick Start Steps

### 1. Generate Your Identity

First, you'll need to create your identity on the platform:

```bash
dc keygen --generate --identity <id-slug>
```

Example:

```bash
dc keygen --generate --identity my-id
```

> **Important**: Save the generated mnemonic in a secure location. It can be used to recreate your identity if needed.

#### Alternative: Using OpenSSL

You can also generate your identity using OpenSSL:

```bash
mkdir -p $HOME/.dcc/identity/my-id
openssl genpkey -algorithm ED25519 -out $HOME/.dcc/identity/my-id/private.pem
```

### 2. Get Initial Tokens

Before registering, you'll need some DC tokens:

1. Visit [kongswap](https://www.kongswap.io/swap?from=cngnf-vqaaa-aaaar-qag4q-cai&to=ggi4a-wyaaa-aaaai-actqq-cai) or [icpswap](https://app.icpswap.com/swap?input=ryjl3-tyaaa-aaaaa-aaaba-cai&output=ggi4a-wyaaa-aaaai-actqq-cai)
2. Exchange for DC tokens and send them to the Principal Id that you get from `dc account --identity <my-id>`
3. Check the current registration fee in DCT:

```bash
dc ledger-remote get-registration-fee
```

### 3. Register Your Account

Choose your account type:

#### For Users

```bash
dc user register --identity my-user
```

#### For Providers

```bash
dc np register --identity my-provider
```

## Next Steps

After completing the basic setup, you can:

### For Users

- [Learn how to find and contract providers](user-guide.md)
- [Understand token distribution](token-distribution.md)
- [Participate in the community](https://github.com/orgs/decent-stuff/discussions)

### For Providers

- [Set up your provider profile](provider-guide.md)
- [Create and manage offerings](provider-guide.md#offerings)
- [Participate in token distribution](token-distribution.md)

## Common Operations

### Check Ledger Status

```bash
dc ledger-remote fetch
```

### View Available Offerings

```bash
dc offering list
```

### Search Specific Offerings

```bash
dc offering query 'memory >= 512MB AND storage.size > 1gb'
```

## Getting Help

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
- 📚 [Read the Whitepaper](https://decent-cloud.org/)

## Best Practices

1. **Security**

   - Always backup your mnemonic phrase
   - Keep your private keys secure
   - Use strong passwords and secure communication channels

2. **Resource Management**

   - Regularly check your token balance
   - Monitor your active contracts
   - Keep your local ledger synchronized

3. **Community Participation**
   - Engage in discussions
   - Report issues and bugs
   - Share your experience and feedback

Remember: The platform is community-driven, and your participation helps make it better for everyone!
