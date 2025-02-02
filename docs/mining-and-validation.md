# Mining and Validation Guide

This guide explains how to participate in the Decent Cloud network as a miner/validator and the benefits of doing so.

## Overview

Unlike traditional blockchain networks that use Proof of Work, Decent Cloud uses a more environmentally friendly validation mechanism. Validators participate in block creation and token distribution through regular check-ins, making the system more energy-efficient and accessible.

## Benefits

### For Validators

- 🪙 Earn DCT tokens through regular participation
- 💰 Share in block rewards (50 DCT every 10 minutes)
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
   - Purchase tokens on [KongSwap](https://www.kongswap.io/swap?from=cngnf-vqaaa-aaaar-qag4q-cai&to=ggi4a-wyaaa-aaaai-actqq-cai)
   - Required: minimum 0.5 DCT for registration, plus 0.5 DCT per check-in (validation)

2. **Register as Provider**

```bash
dc np register --identity my-validator
```

Registration fee: 0.5 DCT (one-time)

## Participation

### Regular Check-ins

A check-in will automatically pull the complete ledger, which provides you with the latest ledger hash, necessary for the check-in process. The more people with the complete ledger, the better the security.

```bash
dc np check-in --identity my-validator --memo "Your optional memo"
```

- Cost: 0.5 DCT per block
- Reward: Share of 50 DCT block reward
- Frequency: Every 10 minutes
- Distribution: Equal share among all participants

### Best Practices

1. **Automation**

   - Set up automated check-ins
   - Monitor system health
   - Keep sufficient token balance
   - Track participation status

2. **Token Management**

   - Maintain reserve for fees
   - Monitor rewards
   - Plan reinvestment strategy
   - Keep track of earnings

3. **Community Engagement**
   - Participate in discussions
   - Share experiences
   - Help new validators
   - Suggest improvements

## Monitoring

### Check Current Fee

```bash
dc ledger-remote get-registration-fee
```

### View Participation Status

```bash
dc ledger-remote fetch
dc np list --balances
```

### Track Rewards

```bash
dc account --identity my-validator
```

## Economics

### Block Rewards

- 50 DCT per block
- Approximately every 10 minutes
- Equal distribution among participants
- Halving schedule similar to Bitcoin

### Costs

- Registration: 0.5 DCT (one-time)
- Check-in: 0.5 DCT per block
- Transaction fees: Minimal

### ROI Considerations

- Regular participation required
- Rewards based on total participants
- Network growth potential
- Token value appreciation

## Tips for Success

1. **Consistent Participation**

   - Regular check-ins
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
