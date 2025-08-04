# Token Distribution System

This document explains the Decent Cloud Token (DCT) distribution system, its mechanisms, and how to participate.

## Overview

Decent Cloud uses a periodic token distribution mechanism that balances operations and incentivizes participant behavior. The system is designed to be fair, transparent, and sustainable.

## Token Economics

### Minting Schedule

- **Block Time**: New tokens are minted approximately every 10 minutes
- **Initial Block Reward**: 50 DCT per block
- **Halving Schedule**: Every 210,000 blocks (similar to Bitcoin)
- **Total Supply Cap**: Approximately 21 million DCT

### Distribution Breakdown

1. **Block Rewards**

   - Tokens are allocated to active participants who have paid the participation fee
   - If no participants are present, rewards carry over to the next block

2. **Registration Fees**
   - Equal to 1/100th of the block reward (0.5 DCT until first halving)
   - Directed to DAO-controlled wallet
   - Funds platform development and maintenance

## Participation Guide

### Checking Current Fee

```bash
dc ledger-remote get-registration-fee
```

### Provider Participation

```bash
dc provider check-in --identity my-id --memo "Your optional memo here"
```

### Viewing Distribution Status

```bash
dc ledger-remote fetch
```

## Economic Model

### Built-in Demand

- Developers use DCT to rent nodes
- Creates natural token circulation
- Balances supply with actual platform usage

### Provider Incentives

- Earn DCT through participation
- Option to hold tokens for potential value increase
- Can sell tokens to cover operational costs

### Price Stability

- DAO governance can adjust parameters
- Reward system adaptable to market conditions
- Built-in mechanisms to prevent excessive volatility

## Governance

### DAO Control

- Community-driven decision making
- Transparent parameter adjustments
- Focus on long-term sustainability

### Smart Contracts

- All token operations governed by smart contracts
- Ensures secure and transparent transactions
- Compliant with relevant regulations

## Technical Implementation

### Ledger Synchronization

- Local copy maintained for security
- Cryptographically protected
- Similar to git's distributed model

### Security Features

- Multiple ledger copies prevent tampering
- Cryptographic verification of transactions
- Transparent audit trail

## Best Practices

1. **Regular Participation**

   - Set up automated check-ins
   - Monitor reward distribution
   - Keep local ledger synchronized

2. **Token Management**

   - Maintain secure wallet practices
   - Regular backup of credentials
   - Monitor transaction history

3. **Community Engagement**
   - Participate in governance decisions
   - Provide feedback on distribution mechanics
   - Engage in improvement proposals

## Future Developments

- WebUI integration for easier participation
- Enhanced analytics and reporting
- Additional governance features

For technical details and the complete tokenomics model, please refer to our [whitepaper](https://decent-cloud.org/).

## Support and Resources

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
- 📚 [Read the Whitepaper](https://decent-cloud.org/)
