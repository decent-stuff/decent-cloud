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

Share this output in the [community discussions](https://github.com/orgs/decent-stuff/discussions) to get started.

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

```bash
dc provider validate --identity my-validator --memo "Your optional memo"
```

- Cost: 0.5 DCT per block
- Reward: Share of 50 DCT block reward
- Frequency: Every 10 minutes
- Distribution: Equal share among all participants

### Best Practices

1. **Automation**

   - Schedule regular validations
   - Keep an eye on system health
   - Ensure you always have enough token balance

2. **Token Management**

   - Maintain reserve for fees
   - Track your rewards over time
   - Plan any reinvestments in validation or other services

3. **Community Engagement**
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

### Block Rewards

- 50 DCT per block
- Every 10 minutes
- Halving schedule similar to Bitcoin
- Equal distribution among all active validators

### Costs

- Registration: 0.5 DCT (one-time)
- Validation: 0.5 DCT per block
- Transaction fees: Minimal

### ROI Considerations

- Consistent participation is key
- Rewards depend on total validators
- Future network growth may affect token value

## Tips for Success

1. **Consistent Participation**

   - Regular validations
   - Maintain adequate token balance
   - Monitor system performance
   - Stay updated with changes

2. **Community Involvement**

   - Share knowledge
   - Help other validators
   - Participate in governance
   - Suggest improvements

3. **Technical Maintenance**
   - Keep software updated
   - Monitor system resources
   - Maintain reliable connectivity
   - Regular security checks

## Support

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
- 📚 [Read Documentation](https://decent-cloud.org/)

## Future Developments

- WebUI for easier management
- Enhanced monitoring tools
- Automated reward tracking
- Advanced analytics

Remember: Your participation helps secure and grow the network while earning rewards!
