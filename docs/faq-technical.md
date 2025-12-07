# Decent Cloud FAQ - Technical & Security

Frequently asked questions about trust & reputation, security, technical architecture, and troubleshooting.

---

## Trust & Reputation System

### How does the reputation system work?

Reputation is calculated from real, paid transactions:
- Every completed contract affects reputation
- Positive outcomes increase reputation
- Failed deliveries or disputes decrease reputation
- All reputation changes recorded on blockchain (immutable)

### Can providers fake their reputation?

No. The system has multiple safeguards:
- **Transaction-based**: Only real payments count
- **Blockchain record**: All history is immutable
- **Public visibility**: Anyone can audit reputation history
- **Statistical analysis**: Patterns detected and flagged

### What is the difference between Trust Score and Reputation?

- **Reputation**: Raw numeric score from all transactions (can be negative)
- **Trust Score**: 0-100 composite score combining multiple metrics

Trust Score provides a more holistic view including response times, completion rates, and customer satisfaction - not just transaction count.

### Can I see a provider's complete history?

Yes. All transaction history is public and visible:
- Contract count and value
- Completion rates
- Response times
- Customer feedback
- Active contracts
- Historical reputation changes

### What happens to reputation after disputes?

Disputes affect both parties:
- **Provider loses reputation**: If dispute resolved against them
- **User loses reputation**: If dispute was frivolous
- **Dispute cost**: Both parties risk reputation in disputes

This discourages false disputes while protecting legitimate complaints.

---

## Security & Privacy

### How secure is my account?

Very secure. Decent Cloud uses:
- **ED25519 cryptography**: Industry-standard digital signatures
- **No passwords**: Pure cryptographic authentication
- **Seed phrase recovery**: BIP39 standard backup
- **Multi-device keys**: Separate keys per device for isolation

### What data is public vs private?

**Public:**
- Username and display name
- Reputation score and history
- Provider offerings and prices
- Contract history (anonymized)
- Trust metrics

**Private:**
- Email address (optional, for notifications only)
- SSH keys and credentials
- Personal identity details
- Specific rental contents

### How do I recover my account if I lose access?

Two recovery methods:
1. **Seed phrase**: Regenerate keys from your backup phrase
2. **Email recovery**: If you set up email recovery, request account access reset

Always keep your seed phrase stored securely offline.

### Is my payment information safe?

Yes:
- DCT transactions: Cryptographically signed on blockchain
- Stripe payments: Industry-standard PCI-compliant processing
- ICPay: Blockchain-native secure payments
- No payment data stored on Decent Cloud servers

### Can someone impersonate my identity?

Only if they have your private key or seed phrase. Protect these:
- Never share your seed phrase
- Store backup securely offline
- Use separate keys for separate devices
- Remove keys from devices you no longer control

### How does Decent Cloud prevent fraud?

Multiple mechanisms:
- **Escrow payments**: Funds protected until delivery
- **Reputation system**: Fraudsters quickly identified
- **Red flag detection**: Automated warning system
- **Dispute resolution**: Fair process for conflicts
- **Blockchain transparency**: All activity auditable

---

## Technical Questions

### What technology stack does Decent Cloud use?

- **Frontend**: SvelteKit (Svelte 5)
- **Backend API**: Rust with Poem framework
- **Blockchain**: Internet Computer (ICP) canister
- **Database**: SQLite with ledger synchronization
- **Client library**: WASM (Rust compiled to WebAssembly)
- **CLI**: Native Rust binary

### How does blockchain integration work?

The Internet Computer hosts the reputation ledger:
1. All transactions signed with ED25519 keys
2. Transactions submitted to ICP canister
3. Canister validates and stores permanently
4. API syncs local database from blockchain
5. Users query API for fast responses

Blockchain provides immutability; API provides speed.

### What is Proof-of-Ledger-Possession?

Instead of energy-intensive Proof-of-Work, validators prove they:
1. Possess the complete transaction ledger
2. Can compute cryptographic hash of entire history
3. Maintain honest copy of blockchain

This is environmentally friendly while maintaining security.

### Can I run my own node?

Yes. The software is open source:
1. Clone the repository
2. Build the Rust binaries
3. Configure your environment
4. Connect to the network

See the development documentation for detailed setup.

### Is there an API I can use?

Yes. The REST API at `api.decent-cloud.org` provides:
- Marketplace listings
- User and provider management
- Contract operations
- Reputation queries
- Payment processing

API documentation available in the repository.

### What are the system requirements for running the CLI?

Minimal requirements:
- **Linux**: Ubuntu 20.04+ or equivalent
- **macOS**: 10.15+ (Catalina or later)
- **Windows**: Windows 10 or later
- **Memory**: 256MB RAM minimum
- **Storage**: 100MB for application + ledger data
- **Network**: Stable internet connection

---

## Comparison with Traditional Cloud

### When should I use Decent Cloud vs AWS/Azure/GCP?

**Use Decent Cloud when:**
- You want transparent, competitive pricing
- You need to avoid vendor lock-in
- You value community governance
- You prefer decentralized infrastructure
- You want peer-to-peer cost efficiency
- You need specialized resources (GPUs, niche services)

**Use traditional cloud when:**
- You need guaranteed SLAs (99.9%+)
- You require compliance certifications (HIPAA, PCI-DSS)
- You need 24/7 enterprise support
- You want managed services (databases, ML platforms)
- You need global infrastructure guarantees

### Is Decent Cloud cheaper than AWS?

Often yes, due to:
- Peer-to-peer marketplace dynamics
- No corporate overhead
- Competitive provider pricing
- Transparent fee structure

However, prices vary by provider and resource type. Compare specific offerings for your use case.

### Can I migrate from AWS to Decent Cloud?

Yes. Migration steps:
1. Identify equivalent resources on Decent Cloud
2. Find providers matching your specifications
3. Set up your Decent Cloud account
4. Deploy workloads to new provider
5. Test thoroughly
6. Migrate production workloads

No proprietary lock-in on Decent Cloud means easy future migrations too.

### Does Decent Cloud offer SLAs?

SLAs are provider-specific:
- Each provider sets their own terms
- Trust scores indicate reliability
- Check provider history before committing
- Platform doesn't guarantee specific uptime

For mission-critical workloads, choose providers with high trust scores and proven track records.

### What about data sovereignty and compliance?

- **Data location**: Choose providers in specific regions
- **Data control**: You control your data, not a corporation
- **Compliance**: Provider-specific; verify before use
- **Auditing**: Blockchain provides transparent history

For strict compliance requirements, verify provider capabilities directly.

---

## Troubleshooting & Support

### Where can I get help?

- **Community discussions**: `https://github.com/orgs/decent-stuff/discussions`
- **GitHub issues**: Report bugs at `https://github.com/decent-stuff/decent-cloud/issues`
- **Documentation**: Detailed guides in the `/docs` directory
- **Provider support**: Each provider has their own support portal

### My transaction is stuck. What do I do?

1. Check the transaction status in your dashboard
2. Wait a few minutes (blockchain confirmations take time)
3. If stuck for more than 30 minutes, contact support
4. Provide transaction ID for investigation

### A provider isn't responding. What should I do?

1. Wait at least 24-48 hours for initial response
2. Use the provider's support portal if available
3. If no response after 48 hours, consider:
   - Opening a dispute
   - Canceling for a refund
   - Choosing a different provider

Remember: Providers with >48 hour response times get red-flagged.

### I forgot my seed phrase. Can I recover my account?

If you set up email recovery: Yes, request a recovery via email.

If you didn't set up email recovery and don't have your seed phrase: Unfortunately, no. Cryptographic security means nobody can access your account without the keys.

**Prevention**: Always store your seed phrase securely when creating an account.

### How do I report a bug?

1. Check existing issues on GitHub
2. If new, create an issue with:
   - Clear description of the problem
   - Steps to reproduce
   - Expected vs actual behavior
   - Your environment (OS, CLI version, etc.)
3. Submit at `https://github.com/decent-stuff/decent-cloud/issues`

### How do I request a new feature?

1. Check existing discussions on GitHub
2. Start a new discussion describing:
   - The feature you want
   - Why it would be useful
   - Any implementation ideas
3. Community and developers will discuss
4. Popular requests may be prioritized

### The website isn't loading. What should I do?

1. Check your internet connection
2. Try refreshing the page
3. Clear browser cache
4. Try a different browser
5. Check status at GitHub discussions for known outages
6. Try CLI as an alternative access method

---

## Additional Resources

### Official Links

- **Website**: `https://decent-cloud.org/`
- **Dashboard**: `https://decent-cloud.org/dashboard`
- **API**: `https://api.decent-cloud.org`
- **GitHub**: `https://github.com/decent-stuff/decent-cloud`
- **Discussions**: `https://github.com/orgs/decent-stuff/discussions`

### Documentation

- Getting Started Guide: `docs/getting-started.md`
- User Guide: `docs/user-guide.md`
- Provider Guide: `docs/mining-and-validation.md`
- Reputation System: `docs/reputation.md`
- Whitepaper: `docs/decent-cloud-whitepaper.pdf`

### Exchanges

- **KongSwap**: Trade DCT for ICP
- **ICPSwap**: Alternative DCT/ICP trading

---

*See also: [FAQ - General](faq-general.md)*

*Last updated: December 2025*
