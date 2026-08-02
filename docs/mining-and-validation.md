# Mining and Validation Guide

This guide explains how to participate in the Decent Cloud network as a miner/validator, why it matters for our tamper-proof ledger, and how you can benefit from doing so.

Because the [reputation ledger](reputation.md) is central to Decent Cloud, validators maintain a complete copy of the blockchain, including reputation data and all other transactions. By cryptographically signing the hash of this entire ledger, you prove that you possess the most up-to-date and unaltered record. This process safeguards the network against tampering and ensures reliable reputation tracking.

## Overview

Decent Cloud uses an environmentally friendly validation process instead of traditional Proof of Work. Validators confirm that the shared ledger has not been tampered with, create a local copy, and receive an equal share of block rewards. This design helps to:

- Improve security by increasing the number of full ledger copies
- Remain energy-efficient
- Offer easy onboarding for participants

## Benefits

### For Validators

- 🪙 Earn DCT tokens through regular participation
- 💰 Share in block rewards (50 DCT every 10 minutes, halving every 210,000 blocks)
- 🌱 Participate in an eco-friendly system
- 🏗️ Help build a decentralized cloud infrastructure
- 🎯 Low barrier to entry compared to traditional mining
- 🤝 Be part of the governance system

### For the Network

- 🔒 Enhanced security through distributed validation
- ⚖️ Better decentralization
- 🌿 Environmental sustainability
- 📈 Network growth and stability
- 🤝 Community-driven development
- 🛡️ Protection against attacks

## Getting Started

### Prerequisites

1. Install the Decent Cloud CLI tool

   - Follow the [installation guide](installation.md)
   - Or build from source following the [development guide](development.md)

2. Generate Your Identity

```bash
dc keygen --generate --identity my-validator
```

3. Verify Your Account and get the Principal Id for the Identity

```bash
dc account --identity my-validator
```

This Principal ID is required for token transfers and network registration. Keep it secure and share it only when necessary for receiving tokens or registration purposes.

### Initial Setup

1. **Get Initial Tokens**

   - Receive initial transfer from community members, or
   - Purchase tokens e.g. on [KongSwap](https://www.kongswap.io/swap?from=cngnf-vqaaa-aaaar-qag4q-cai&to=ggi4a-wyaaa-aaaai-actqq-cai)
   - Required: minimum 0.5 DCT for registration, plus 0.5 DCT per validation

2. **Register as Provider**

```bash
dc provider register --identity my-validator
```

Registration fee: 0.5 DCT (one-time)

## Participation

### Validation (mining)

By running the validation command, you automatically fetch the latest ledger and sign its hash, proving you have a full, unaltered copy. The more validators that run this process, the safer the network—and the reputation system—becomes.

#### Automated Docker Deployment (retired)

The automated validator (`api-validate`) ran under the now-retired
`docker-compose.prod.yml` stack, which has been removed — production now runs on
k8s (see [`cf/DEPLOYMENT_CONFIG.md`](../cf/DEPLOYMENT_CONFIG.md#prod)), and the k8s manifests
currently have no validator Deployment. Re-introducing automated validation means
adding a validator to the k8s manifests, not to a compose file.

**Benefits (when it ran):**
- ✅ Automated validation every 10 minutes
- ✅ Runs continuously in the background
- ✅ Includes health monitoring
- ✅ Shares infrastructure with sync service

See [cf/README.md](../cf/README.md#blockchain-validator) for context.

#### Manual Validation (CLI)

For testing or one-off validations, you can use the CLI directly:

```bash
dc provider check-in --identity my-validator --memo "Your optional memo"
```

**Note:** Manual validation requires running the command every 10 minutes to maximize rewards. The Docker deployment handles this automatically.

**Validation Details:**
- Cost: 0.5 DCT per block
- Reward: Share of 50 DCT block reward
- Frequency: Every 10 minutes
- Distribution: Equal share among all participants

### Best Practices

1. **Use Docker Deployment**

   - Recommended for continuous validation
   - Automatic scheduling every 10 minutes
   - Built-in health monitoring
   - See [cf/README.md](../cf/README.md#blockchain-validator-optional)

2. **Token Management**

   - Maintain reserve for fees (minimum 0.5 DCT per validation)
   - Monitor balance regularly: `dc account --identity my-validator --balance`
   - Track your rewards over time
   - Plan any reinvestments in validation or other services

3. **System Monitoring**
   - Check validator logs: `docker logs decent-cloud-api-validate-prod`
   - Monitor health status: `docker ps --filter name=validate`
   - Ensure ledger stays synchronized

4. **Community Engagement**
   - Participate in forums and discussions
   - Collaborate with fellow validators
   - Contribute ideas for platform improvements

## Monitoring

### Check Current Fee

```bash
dc ledger-remote get-registration-fee
```

### View Participation Status

```bash
dc ledger-remote fetch
dc provider list --balances
```

### Track Rewards

```bash
dc account --identity my-validator
```

## Economics

For detailed information about token economics, block rewards, and costs, see the [Token Distribution Guide](token-distribution.md).

Key points for validators:
- Block rewards (50 DCT every 10 minutes) are shared equally among active validators
- Registration requires 0.5 DCT (one-time fee)
- Each validation requires 0.5 DCT
- Your ROI depends on consistent participation and total number of validators


For support and community discussions, visit the [main documentation](../docs/README.md#getting-help).

