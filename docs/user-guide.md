# User Guide

This guide covers everything you need to know about using the Decent Cloud platform as a user.

## Getting Started

### Choose Your Interface

Decent Cloud offers two interfaces:

**Web Interface** (Recommended for most users)
- Browse marketplace without an account
- Visual resource management
- Easy account creation
- Visit [decent-cloud.org/dashboard](https://decent-cloud.org/dashboard)

**CLI** (For advanced users and automation)
- Complete [installation](installation.md)
- Generate identity and register account
- Initial DC tokens for operations

## Using the Web Interface

### Browsing Without an Account

You can explore the platform without creating an account:

1. Visit the [Dashboard](https://decent-cloud.org/dashboard)
2. Browse available offerings in the marketplace
3. View provider reputation and validator information
4. Explore public user profiles

When you're ready to rent resources, you'll be prompted to create an account or login.

### Creating an Account (Web)

When you find a resource to rent:

1. Click "Rent Resource" on any offering
2. Choose "Create Account" from the authentication prompt
3. Select authentication method:
   - **Seed Phrase** (recommended): Auto-generated, secure
   - **Existing Seed Phrase**: Import from CLI or another device
4. Follow the setup wizard
5. You'll return to complete your rental request

### Managing Your Account (Web)

Access account settings from the sidebar after login:
- **Security** - Manage devices and authentication keys
- **Public Profile** - Edit your public profile information
- **Rentals** - View and manage your rental contracts

## Using the CLI

### Prerequisites

- Completed [installation](installation.md)
- Generated identity and registered account
- Initial DC tokens for operations

### Registration (CLI)

```bash
dc user register --identity my-user
```

## Finding Resources

### Listing All Offerings

```bash
dc offering list
```

### Searching Specific Offerings

Use the query command with specific criteria:

```bash
dc offering query 'memory >= 512MB AND storage.size > 1gb'
```

Common search criteria:

- Memory: `memory >= <size>`
- Storage: `storage.size > <size>`
- CPU: `cpu.cores >= <number>`
- Location: `location == "<region>"`

### Understanding Offerings

Key aspects to consider:

1. Resource specifications
2. Provider reputation
3. Pricing
4. Location
5. Terms of service

## Contracting Resources

### Preparing for Contract

1. Review provider's profile and history:

```bash
dc provider list --balances
```

2. Ensure sufficient token balance
3. Prepare SSH public key
4. Have contact information ready

### Creating a Contract Request

Basic contract request:

```bash
dc contract sign-request \
  --offering-id <offering-id> \
  --identity my-user \
  --requester-ssh-pubkey "ssh-ed25519 AAAAC3..." \
  --requester-contact "https://github.com/username" \
  --memo "Project deployment" \
  --provider-pubkey-pem <provider-key> \
  --interactive
```

> The `--interactive` flag will prompt for any missing required information.

### Monitoring Contract Status

Check open contract requests:

```bash
dc contract list-open
```

## Managing Resources

### Resource Management in Decent Cloud

1. **Ledger Synchronization**

   - Regularly sync your local ledger: `dc ledger-remote fetch`
   - This ensures you have the latest contract and provider information
   - Essential for accurate balance and status checks

2. **Token Balance Management**

   - Monitor your DC token balance before making contract requests
   - Check registration fees: `dc ledger-remote get-registration-fee`
   - Maintain sufficient tokens for contract operations

3. **Contract Monitoring**

   - Track active contracts with `dc contract list-open`
   - Verify provider reputation before contracting: `dc provider list --balances`
   - Keep records of contract terms and communication

## Token Management

### Checking Balance

```bash
dc ledger-remote fetch
dc user balance --identity my-user
```

### Token Best Practices

- Maintain sufficient balance for operations
- Monitor transaction history
- Keep private keys secure
- Regular ledger synchronization

## Troubleshooting

### Platform-Specific Issues

1. **Ledger Sync Problems**

   - Force ledger sync: `dc ledger-remote fetch --force`
   - Check your internet connection to the Internet Computer network
   - Verify your identity is correctly configured

2. **Contract Request Issues**

   - Ensure you have sufficient DC tokens for registration fees
   - Verify your SSH public key format is correct
   - Check that provider offerings are active: `dc offering list`

3. **Identity and Authentication**

   - Confirm identity exists: `dc keygen --list`
   - Verify account registration: `dc user status --identity <your-id>`
   - Check Principal ID matches your expectations

### Getting Support

For platform-specific support:
- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
