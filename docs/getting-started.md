# Getting Started with Decent Cloud

This guide will help you get up and running with Decent Cloud quickly.

## Choose Your Interface

Decent Cloud offers two ways to interact with the platform:

### Web Interface (Easiest)

**Best for:**
- First-time users exploring the platform
- Quick marketplace browsing
- Visual resource management
- Non-technical users

**Get started:**
1. Visit [decent-cloud.org/dashboard](https://decent-cloud.org/dashboard)
2. Browse available offerings without creating an account
3. Create an account when ready to rent resources
4. See the [Web Interface Guide](web-interface-guide.md) for details

### Command Line Interface (Advanced)

**Best for:**
- Automation and scripting
- Advanced provider management
- Integration with existing tools
- Power users and developers

**Get started:**
- Follow the [CLI installation guide](installation.md) below

## Quick Start (Web Interface) — No crypto required

The web interface supports signup with Google OAuth or email, and lets you pay
with a credit card (Stripe) for fiat-priced offerings. **You do not need DCT
tokens or a crypto wallet for this path.**

1. **Browse the Marketplace**
   - Visit [decent-cloud.org/dashboard/marketplace](https://decent-cloud.org/dashboard/marketplace)
   - No account required to browse

2. **Find a Resource**
   - Use search and filters
   - Check provider reputation
   - Compare pricing

3. **Create Account**
   - Click "Rent Resource" (or sign in from the header)
   - Sign in with Google, or create an account with email
   - Verify your email when the confirmation link arrives — this is required
     before your first rental request

4. **Pay and Rent**
   - In the rental dialog, pick a payment method:
     - **Credit card (Stripe)** — default for fiat-priced offerings (USD, EUR, ...)
     - **ICPay** — required for offerings priced in ICP
   - Enter SSH key and duration, confirm, complete payment on Stripe
   - Provider provisions your resource and the contract moves to `active`

> Self-rental note: if you rent from your own offering, the payment amount is
> zero and no card is charged.

## Quick Start (CLI) — Advanced, requires DCT

The CLI path uses on-chain identity and a DCT-denominated registration fee.
Pick this only if you want full CLI automation, are registering as a provider,
or otherwise need direct on-chain interaction. Most users should use the web
interface above.

### Prerequisites

- Decent Cloud CLI installed (see [Installation Guide](installation.md))
- Basic understanding of cloud resources and command-line interfaces

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

The CLI path requires DCT to cover the on-chain registration fee. (The web
interface does not — see "Quick Start (Web Interface)" above.)

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
dc provider register --identity my-provider
```

## Next Steps

After completing the basic setup, you can:

### For Users

- [Learn how to find and contract providers](user-guide.md)
- [Understand token distribution](token-distribution.md)
- [Participate in the community](https://github.com/orgs/decent-stuff/discussions)

### For Providers

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
