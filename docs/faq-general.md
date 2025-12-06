# Decent Cloud FAQ - General

Frequently asked questions about Decent Cloud platform basics, getting started, using services, and becoming a provider.

---

## General Questions

### What is Decent Cloud?

Decent Cloud is a peer-to-peer marketplace for decentralized cloud resources. It connects people who need computing resources (users) with those who have spare capacity to offer (providers). Think of it as an "Airbnb for cloud computing" - but with transparent reputation, no vendor lock-in, and blockchain-backed trust.

### What types of resources can I find on Decent Cloud?

The marketplace offers a wide variety of cloud resources:
- **GPU compute** - For AI/ML training, rendering, and scientific computing
- **General compute** - Virtual machines and containers
- **Storage** - Object storage, block storage, and backup solutions
- **Bandwidth** - Network capacity and CDN services
- **Web services** - Hosting, APIs, and specialized applications
- **ML/LLM services** - AI model inference and fine-tuning

### How is Decent Cloud different from traditional cloud providers?

| Aspect | Decent Cloud | AWS/Azure/GCP |
|--------|--------------|---------------|
| Vendor Lock-in | None | High |
| Pricing | Transparent, competitive | Complex, opaque |
| Trust Model | Transaction-based reputation | Brand-based |
| Governance | Community DAO | Corporate |
| Data Ownership | Complete user control | Provider terms apply |

### Who runs Decent Cloud?

Decent Cloud is community-governed through a DAO (Decentralized Autonomous Organization). There's no single company controlling the platform. The core development team consists of experienced developers specializing in cryptography, distributed systems, and blockchain technology. The project is open source under the Apache 2.0 license.

### Is Decent Cloud open source?

Yes. The entire codebase is open source under the Apache 2.0 license. You can review, audit, and contribute to the code on GitHub at `https://github.com/decent-stuff/decent-cloud`.

### What blockchain does Decent Cloud use?

Decent Cloud uses the Internet Computer (ICP) blockchain to store its immutable ledger. This provides:
- Tamper-proof transaction history
- Decentralized reputation records
- Transparent token distribution
- No single point of failure

### Is Decent Cloud live and usable today?

Yes. The platform is in production with:
- Web interface at `app.decent-cloud.org/dashboard`
- API running at `api.decent-cloud.org`
- CLI tools available for Linux, macOS, and Windows
- Active providers offering resources

---

## Getting Started

### Do I need an account to browse the marketplace?

No. You can browse all offerings, view provider trust scores, and compare prices without creating an account. You only need to register when you're ready to rent resources or become a provider.

### How do I create an account?

1. Visit `app.decent-cloud.org/dashboard`
2. Click on any offering and select "Rent Resource"
3. Choose "Create Account"
4. Select your preferred login method:
   - **Google**: Quickest - just sign in with your Gmail
   - **Seed phrase**: For users who want full cryptographic control
5. Follow the setup wizard (takes about 2 minutes)
6. Your account is ready

### What login options are available?

You have multiple ways to access Decent Cloud:

- **Google (Gmail)**: Quick login with your existing Google account - easiest option
- **Seed phrase**: Cryptographic key-based authentication for maximum control
- **More options coming**: Additional login methods will be added based on user requests

You don't have to use a seed phrase if you prefer simpler login methods.

### What is a seed phrase?

A seed phrase is a series of 12 or 24 words that can regenerate your cryptographic keys. It's an alternative to social logins for users who want:
- No dependency on third-party accounts (Google, etc.)
- Full cryptographic control over their identity
- Industry-standard (BIP39) recovery method
- Access from any device without external services

**Important**: If you choose seed phrase authentication, store it securely offline. Anyone with your seed phrase can access your account.

### Can I use the same account on multiple devices?

Yes. Decent Cloud supports multi-device access through multiple keys linked to one account. You can:
- Add new device keys to your existing account
- Remove keys from devices you no longer use
- Each device has its own key but shares the same identity

---

## For Users & Renters

### How do I find the right resource for my needs?

Use the marketplace search with powerful filters:
- **Basic search**: Type keywords like "gpu" or "storage"
- **Advanced DSL**: Use precise queries like `type:gpu price:[50 TO 500] memory:>=32GB`
- **Filter by**: Price, location, trust score, specs, availability

### How do I know if a provider is trustworthy?

Every provider has a visible **Trust Score** (0-100) based on real transaction data:
- **Completion rate**: Do they deliver what's promised?
- **Response time**: How quickly do they respond?
- **Customer satisfaction**: What do past customers think?
- **Repeat customers**: Do people come back?

The system also shows **red flags** for risky providers:
- Early cancellation rate above 20%
- Response time over 48 hours
- Provisioning failures above 15%
- Long periods of inactivity

### What payment methods are accepted?

- **DCT tokens**: Native platform currency (no payment processing fees)
- **Credit/Debit cards**: Via Stripe (USD, EUR, and other currencies)
- **Cryptocurrencies**: BTC, ETH, SOL, ICP, USDC, and more via ICPay

### What happens after I pay for a resource?

1. Your payment is held in escrow
2. The provider receives a notification
3. The provider provisions your resource
4. Once delivered, funds are released to the provider
5. You receive access credentials for your resource

### Can I cancel a rental early?

Yes. Decent Cloud offers prorated refunds:
- You pay only for the time used
- Unused time is refunded automatically
- Refund processed in the original payment method
- No cancellation penalties (though providers track early cancellation rates)

### What if a provider doesn't deliver?

Several protections exist:
1. **Escrow**: Funds aren't released until service is delivered
2. **Disputes**: You can dispute and receive a refund
3. **Reputation impact**: Bad providers lose trust score quickly
4. **Red flags**: System warns future users about problematic providers

---

## For Providers & Node Operators

### How do I become a provider?

1. Create an account (web or CLI)
2. Ensure you have at least 0.5 DCT for registration
3. Register as a provider
4. Create your offerings (pricing, specs, availability)
5. Start accepting contracts

### What can I offer on Decent Cloud?

Anything computing-related:
- Spare server capacity
- GPU time
- Storage space
- Bandwidth
- Specialized services (APIs, ML models, etc.)
- Web hosting

### How do I get paid?

Decent Cloud supports multiple payout options:

**Supported payment methods:**
- **Stripe**: Credit/debit cards (USD, EUR, etc.)
- **Cryptocurrencies**: BTC, ETH, SOL, ICP, USDC, and more
- **DCT tokens**: Native platform currency

**Payment flow:**
1. User pays for your offering (any supported method)
2. Funds held in escrow during delivery
3. After successful delivery, funds released to you
4. Withdraw via your preferred method

### What does the platform handle for me as a provider?

Decent Cloud takes care of the operational heavy lifting so you can focus on delivering your services:

**Included for all providers:**
- **Professional website presence**: Your offerings displayed in the marketplace
- **Payment processing**: All payment methods handled (Stripe, crypto, DCT)
- **L1 customer support**: AI-powered support bot handles common questions
- **Billing & invoicing**: Automatic payment collection and reconciliation
- **Customer communication**: Integrated support portal (Chatwoot)
- **Trust & reputation**: Automatic trust score calculation and display
- **Contract management**: Automated contract lifecycle handling

You bring the resources; we handle everything else.

### What is the Trust Score and how do I improve it?

Your Trust Score (0-100) is calculated from six metrics:
- **Time-to-delivery**: How fast you provision resources
- **Completion rate**: Successfully completed contracts
- **Response time**: How quickly you respond to requests
- **Customer satisfaction**: User ratings and feedback
- **Repeat customers**: Users who come back
- **Active contracts**: Current workload handling

To improve your score:
- Respond to requests within 24 hours
- Deliver resources as promised
- Maintain high availability
- Provide excellent customer service
- Stay active on the platform

### What is validation (mining) and do I have to do it?

**Validation is optional** - you don't have to participate to be a provider.

**Purpose of validation:**
Validators ensure the integrity of the Decent Cloud network by:
- Verifying that provider reputation records are intact and unaltered
- Confirming DCT token transactions are valid
- Maintaining a tamper-proof copy of the blockchain ledger

**If you choose to participate:**
1. Register as a provider
2. Maintain at least 0.5 DCT balance (cost per validation)
3. Run validation either:
   - **Manual**: `dc provider check-in --identity my-validator` every 10 minutes
   - **Automated**: Deploy with Docker for continuous validation
4. Earn share of 50 DCT block reward

No special hardware required - just reliable internet and the ability to prove you possess the full ledger.

---

## Pricing & Payments

### How much does it cost to use Decent Cloud?

Costs vary by resource type and provider:
- **Registration fee**: 0.5 DCT (one-time)
- **Transaction fees**: Typically 2% per transaction
- **Resource prices**: Set by individual providers (competitive marketplace)

### What are the fees?

All fees are transparent and clearly displayed:

**Platform fee:**
- 2% on all transactions (goes to Decent Cloud development)

**Payment processing fees** (charged to providers):
- **DCT tokens**: Free (no additional fees)
- **ICPay (crypto)**: ~0.5%
- **Stripe (cards)**: 3-5% (standard payment processor rates)

**One-time fees:**
- Registration: 0.5 DCT

Payment processing fees are passed through at cost - Decent Cloud doesn't mark them up.

### How does the escrow system work?

1. You pay for a resource
2. Funds are held in escrow (not sent to provider yet)
3. Provider delivers the resource
4. After successful delivery, funds release to provider
5. If delivery fails, you receive a refund

This protects both users and providers.

### How do refunds work?

- **Prorated refunds**: Cancel early and pay only for time used
- **Failed delivery**: Full refund if provider doesn't deliver
- **Disputes**: Refund if dispute resolved in your favor
- **Same payment method**: Refunds go back to original payment source

---

## DC Token (DCT)

### What is DCT?

DCT (Decent Cloud Token) is the native currency of the Decent Cloud platform. It's used for:
- Paying for cloud resources
- Provider registration
- Validator participation
- Governance voting

### What is the total supply of DCT?

The maximum supply is approximately 21 million DCT, following a Bitcoin-like halving schedule:
- Initial block reward: 50 DCT
- Block time: ~10 minutes
- Halving every 210,000 blocks

### How can I earn DCT?

Several ways:
1. **Provide resources**: Earn from users renting your offerings
2. **Validate blocks**: Earn share of 50 DCT per block
3. **Buy on exchanges**: Purchase DCT on KongSwap or ICPSwap

### What gives DCT value?

- **Utility**: Required for platform operations
- **Scarcity**: Fixed maximum supply with halving schedule
- **Demand**: Growing user base needs DCT for transactions
- **Governance**: Token holders participate in DAO decisions

---

*See also: [FAQ - Technical & Security](faq-technical.md)*
