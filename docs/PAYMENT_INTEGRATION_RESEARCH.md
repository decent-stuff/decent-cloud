# Payment Integration Research

**Date**: 2025-11-14
**Status**: Research phase - no implementation yet

## Context

Decent Cloud currently only supports DC Token (DCT) payments. This creates a barrier for mainstream adoption since:
- Most people view crypto as risky or scam-like
- No card payment support exists
- Users must acquire DCT before using the platform

**Goal**: Support easy card payments alongside crypto to enable mainstream adoption while moving the project away from blockchain-first positioning.

## ICPay Research Findings

### What is ICPay?

ICPay is a cryptocurrency payment processor built on Internet Computer blockchain.

**Key Features:**
- 0.5% transaction fee (vs Stripe's 2.9% + $0.30)
- Sub-2-second settlement via chain key encryption
- Multi-crypto support: ICP, ckUSDC, other ICRC tokens
- Developer SDK: `@ic-pay/icpay-sdk` (TypeScript/JavaScript)
- No chargebacks, monthly fees, or setup costs

**Documentation**: https://docs.icpay.org/

### Integration Modes

#### Public SDK (Frontend)
- Initialized with `publishableKey` (safe for client-side)
- Users pay from their IC wallets
- Requires wallet integration (DC already has via `@dfinity/agent`)
- Methods: `createPayment()`, `createPaymentUsd()`, event handling

#### Private SDK (Backend)
- Initialized with `secretKey` (server-only)
- Payment verification, account management, webhooks
- Accessed via `icpay.protected.*` namespace

### Technical Compatibility with Decent Cloud

**Advantages:**
- Uses ICRC ledgers (same standard as DCT)
- TypeScript SDK fits Svelte frontend
- Compatible with existing `@dfinity/agent` wallet integration
- Can support multiple tokens beyond DCT

**Integration Points:**
- Frontend: `website/src/lib/components/RentalRequestDialog.svelte`
- Backend: `api/src/database/contracts.rs`, `api/src/api_handlers.rs`
- Common: `common/src/contract_sign_request.rs` (add ICPay payment ID field)

### Limitations

**CRITICAL**: ICPay only supports cryptocurrency payments. Does NOT support credit/debit cards.

This makes ICPay insufficient for the stated goal of mainstream adoption through card payments.

## Card Payment Provider Options

### Requirements

1. **Card support**: Credit/debit cards (Visa, Mastercard, Amex)
2. **Fiat currency**: USD, EUR, etc.
3. **Developer-friendly**: Good API/SDK
4. **Reasonable fees**: Competitive pricing
5. **Trust**: Recognized brand for mainstream users
6. **Compatibility**: Works with decentralized infrastructure

### Provider Comparison

#### Stripe

**Pros:**
- Industry standard, trusted brand
- Excellent developer experience
- 2.9% + $0.30 per transaction (US)
- Supports 135+ currencies
- Strong fraud protection
- Extensive documentation
- Webhook support for payment events

**Cons:**
- Higher fees than crypto alternatives
- Requires business verification (KYC)
- Account can be frozen/terminated
- Not available in all countries
- Centralized (single point of failure)

**Tech Stack:**
- REST API
- Official SDKs: JavaScript/TypeScript, Python, Ruby, PHP, etc.
- `@stripe/stripe-js` for frontend

#### PayPal

**Pros:**
- Very familiar to general population
- Buyer protection (trust factor)
- 2.9% + $0.30 per transaction (similar to Stripe)
- Supports crypto (can buy/sell crypto within PayPal)
- Available in 200+ countries

**Cons:**
- Known for account freezes/holds
- Customer service issues
- Higher international fees
- Less developer-friendly than Stripe
- Redirects users away from site

**Tech Stack:**
- REST API
- PayPal Checkout SDK
- More complex integration than Stripe

#### Square

**Pros:**
- Good for small businesses
- 2.6% + $0.10 per transaction (slightly cheaper)
- Good developer tools
- Owned by Block (formerly Twitter founder's company)

**Cons:**
- More focused on point-of-sale than online
- Less international support
- Smaller ecosystem than Stripe

#### Coinbase Commerce

**Pros:**
- Crypto-focused but accepts fiat via Coinbase accounts
- Users can pay with cards through Coinbase
- 1% fee
- No chargebacks
- Good for crypto-to-fiat bridge

**Cons:**
- Requires Coinbase account (friction for new users)
- Not truly direct card payment
- Mixed reputation in crypto community

#### Modern Alternatives

**Paddle:**
- Merchant of record model (handles taxes, compliance)
- 5% + $0.50 per transaction (higher but includes more services)
- Good for SaaS businesses
- Handles VAT/sales tax automatically

**Lemon Squeezy:**
- Similar to Paddle (merchant of record)
- Slightly lower fees than Paddle
- Newer, growing platform

## Strategic Recommendation

### Hybrid Payment Architecture

Implement **multiple payment methods** with abstraction layer:

```
User chooses payment method:
├── Card Payment (Stripe) → Mainstream users (easiest, most trusted)
├── Crypto Direct (MoonPay/NOWPayments) → Card-to-crypto conversion
├── Crypto Native (ICPay) → ICP, ckUSDC for crypto-savvy users
└── DCT Payment (Native) → Power users, providers
```

### Payment Method Comparison

| Method | Best For | User Friction | Fees | Trust Factor |
|--------|----------|---------------|------|--------------|
| **Stripe (Card → Fiat)** | General public | Low | 2.9% + $0.30 | Highest |
| **MoonPay (Card → Crypto)** | Crypto buyers | Medium | ~4.5% | Medium |
| **NOWPayments (Crypto)** | Crypto holders | Medium | 0.5-1% | Medium |
| **ICPay (ICP/ICRC)** | IC ecosystem | High | 0.5% | Low (new) |
| **DCT (Native)** | Platform natives | High | Protocol only | Low (unknown) |

### Recommended Primary Approach: Stripe

**Why Stripe should be your main payment method:**

1. **Trust**: Most recognized payment brand globally - eliminates "crypto scam" perception
2. **Developer Experience**: Best-in-class API, documentation, SDKs
3. **Reliability**: 99.99%+ uptime, battle-tested infrastructure
4. **Features**: Subscriptions, invoicing, webhooks, fraud detection, dispute handling
5. **Compliance**: Handles PCI compliance, reduces your liability
6. **Integration**: Clean TypeScript SDK (`@stripe/stripe-js`) for Svelte frontend
7. **Global Coverage**: Supports 135+ currencies, available in 47+ countries
8. **No Blockchain Dependency**: Works independently of crypto infrastructure

**Updated 2025 insights:**
- PayPal fees are slightly lower (2.38% vs 2.75% for $100 transaction) but worse developer experience
- Square is cheaper for in-person (2.6% + $0.10) but less suitable for online SaaS
- Stripe dominates SaaS payment space - used by vast majority of successful SaaS companies

### Secondary: Crypto Payment Options

Keep crypto payments as **optional alternatives** for users who prefer them:

#### Option A: MoonPay (Card-to-Crypto Bridge)
**Pros:**
- Accepts credit cards, Apple Pay (bridges fiat to crypto)
- 160+ country coverage
- Partnership with Mastercard (2025)
- Can enable "pay with card, receive in crypto" flow

**Cons:**
- High fees: up to 4.5% + minimum $3.99
- Primarily designed for consumers buying crypto, not merchant payments
- Complex merchant integration for payment acceptance
- Not purpose-built for recurring/subscription payments

**Verdict**: Could work as fiat-to-crypto on-ramp for users wanting to buy DCT/ICP with cards, but NOT ideal for primary payment flow.

#### Option B: NOWPayments (Crypto-Only)
**Pros:**
- 0.5% fee for mono-currency, 1% for multi-currency
- 300+ cryptocurrencies supported
- Mentions "fiat on-ramp" but details unclear
- Non-custodial

**Cons:**
- Primarily crypto-to-crypto, not card payments
- "Fiat on-ramp" feature not well documented
- Less mature than Stripe
- Requires users to already have crypto

**Verdict**: Good for accepting crypto payments from users who already hold crypto, but doesn't solve the card payment problem.

#### Option C: ICPay (IC Ecosystem)
**Pros:**
- 0.5% fee, lowest cost
- Native ICP/ICRC integration
- Aligns with IC ecosystem
- Already researched (see above)

**Cons:**
- Crypto-only, no card support
- New platform (founded 2024), higher risk
- Requires IC wallet
- Low mainstream recognition

**Verdict**: Keep as option for IC-native users, but not primary payment method.

### Why Keep DCT?

1. **Zero external fees**: Only protocol fees
2. **Incentivization**: Can reward users with DCT
3. **Provider payouts**: Direct settlement in DCT
4. **Token economics**: Maintain token utility and governance
5. **Long-term vision**: As platform grows, DCT becomes more valuable

**However**: Don't force users to buy DCT. Let them pay with Stripe, platform converts to DCT internally if needed.

## Implementation Approach

### Phase 1: Payment Abstraction Layer

Create unified payment interface in backend:

```rust
// common/src/payment_methods.rs
pub enum PaymentMethod {
    DCToken { amount_e9s: TokenAmountE9s },
    Stripe { payment_intent_id: String, amount_cents: u64 },
    ICPay { payment_id: String, ledger_canister_id: String, amount: String },
}

pub struct PaymentResult {
    pub method: PaymentMethod,
    pub status: PaymentStatus,
    pub verified_at: Option<u64>,
}
```

### Phase 2: Stripe Integration

**Frontend:**
- Add `@stripe/stripe-js` dependency
- Payment UI component with card input
- Handle 3D Secure authentication

**Backend:**
- Stripe API client (consider `stripe-rs` crate)
- Webhook handler for payment events
- Payment verification before contract acceptance

**Database:**
- Extend contracts table with payment method metadata
- Store Stripe payment intent IDs
- Track payment status transitions

### Phase 3: ICPay Integration

- Similar to Stripe but using ICPay SDK
- Wallet connection via existing `@dfinity/agent`
- Support ICP and ckUSDC initially

### Phase 4: Provider Payouts

Critical decision needed:
- **Convert to fiat?** Use Stripe Connect for provider payouts
- **Pay in crypto?** Convert Stripe payments to DCT/ICP via exchange
- **Hybrid?** Let providers choose payout method

## Cost Analysis

**For $100 service rental:**

| Payment Method | User Pays | Platform Gets | Provider Gets* | Notes |
|----------------|-----------|---------------|----------------|-------|
| Stripe (card)  | $102.90   | $100.00       | $95.00         | 2.9% + $0.30 fee |
| ICPay (crypto) | $100.50   | $100.00       | $99.50         | 0.5% fee |
| DCT (native)   | $100.00   | $100.00       | $100.00        | Protocol fees only |

*Assumes 5% platform fee; adjust based on business model

**Recommendation**: Pass payment processing fees to users (transparent pricing) rather than absorbing them.

## Risks & Mitigations

### Risk: Stripe Account Termination/Freezing
**Reality Check**: Stripe is known for freezing accounts, especially for:
- High-risk industries (crypto-adjacent services may qualify)
- Sudden volume spikes
- High chargeback rates
- Insufficient business verification

**Evidence from 2025 research:**
- Can hold funds for 30-180 days after termination
- Appeals process exists but no guarantee
- "Unacceptable Risk Policy" gives Stripe broad discretion

**Mitigation Strategy:**
1. Complete business verification thoroughly upfront
2. Maintain low chargeback rate (<0.5%)
3. Have backup provider (PayPal) integrated and ready
4. Keep 90 days operating capital reserve
5. Don't rely solely on Stripe - diversify payment methods
6. Consider escrow system to minimize funds held in Stripe

### Risk: Payment/Crypto Reconciliation Complexity
**Mitigation**: Robust database schema with clear payment state machine

### Risk: Mixing Fiat and Crypto
**Challenge**: Users pay in USD via Stripe, but providers want crypto payouts.

**Mitigation Options:**
1. **Platform absorbs conversion**: Convert fiat to DCT/ICP via exchange, pay providers in crypto
2. **Provider choice**: Let providers choose fiat OR crypto payout (adds complexity)
3. **Hybrid**: Platform fee charged in fiat, service payment in crypto
4. **Treasury management**: Maintain both fiat and crypto reserves

**Recommendation**: Start with Option 1 - simplest for users and providers.

### Risk: Provider Payout Complexity
**Mitigation**: Start with crypto payouts only, add fiat later if needed

### Risk: Regulatory Compliance
**Concerns:**
- If converting fiat to crypto: May need money transmitter licenses
- If holding user funds: May need financial institution licensing
- Tax reporting requirements vary by jurisdiction
- KYC/AML requirements for both fiat and crypto sides

**Mitigation**:
1. Stripe handles most payment compliance
2. Consult legal counsel on crypto conversion regulations
3. Consider acting as pure platform (funds go directly provider-to-renter)
4. Use payment facilitator model initially (Stripe handles regulatory burden)

### Risk: High Fees Eating Margins
**Reality**:
- Stripe: 2.9% + $0.30
- Platform fee: TBD (5-10%?)
- Currency conversion: 1-3%
- Crypto on-ramp (if needed): 1-4.5%

**Total cost could reach 8-15% for fiat→platform→crypto flow**

**Mitigation**:
1. Pass payment processing fees transparently to users
2. Offer discounts for crypto-native payments
3. Volume-based fee reductions
4. Bundle services to increase transaction value

## Critical Business Questions to Answer First

### 1. Business Entity & Legal Structure
**Question**: Do you have a registered business entity that can accept Stripe payments?

**Requirements**:
- Stripe requires business verification (EIN/tax ID for US, equivalent abroad)
- Need Terms of Service, Privacy Policy, Refund Policy
- May need money transmitter licenses if converting fiat to crypto
- Jurisdiction matters for regulatory compliance

**Action needed**: If no entity exists, establish one before Stripe integration.

### 2. Fiat-to-Crypto Conversion Strategy
**Question**: How do you handle fiat payments if providers want crypto?

**Options**:
- **A**: Platform keeps fiat, pays providers in fiat (requires provider bank accounts)
- **B**: Platform converts fiat→crypto via exchange, pays providers in DCT/ICP (requires exchange account, liquidity)
- **C**: Providers choose their payout currency (complex but flexible)
- **D**: Escrow fiat until rental period ends, then convert and pay

**Recommendation**: Start with Option B (auto-convert to crypto) for simplicity, but requires:
- Exchange account (Coinbase, Kraken, etc.)
- Liquidity management
- Potential licensing depending on jurisdiction

### 3. Platform Fee Structure
**Question**: What percentage does the platform take?

**Considerations**:
- Stripe costs 2.9% + $0.30 minimum
- Need to cover: payment processing, development, hosting, support, conversion fees
- Competitors: AWS Marketplace (3%), Digital Ocean (varies), Traditional cloud providers (0% but higher base prices)

**Suggestions**:
- **7-10% total fee** for fiat payments (includes Stripe costs)
- **3-5% fee** for crypto payments (lower overhead)
- **0-2% fee** for DCT payments (encourage platform token usage)
- Transparent fee breakdown shown to users

### 4. Escrow vs Direct Payment
**Question**: Do you hold funds in escrow or pay providers immediately?

**Trade-offs**:
- **Escrow**: Safer (protects against non-delivery), but holds funds longer, complex refunds
- **Direct**: Faster provider payout, simpler, but riskier for users

**Recommendation**: Implement escrow system:
1. User pays → funds held in escrow
2. Provider deploys service → user confirms receipt
3. After confirmation or timeout (e.g. 48 hours) → release to provider
4. Dispute mechanism for failed deployments

This protects both parties and is standard for marketplaces.

### 5. Refund & Chargeback Policy
**Question**: How to handle refunds and chargebacks?

**Challenges**:
- Stripe allows chargebacks (user disputes charge with bank)
- Crypto payments are irreversible
- Need consistent policy across payment methods

**Recommended Policy**:
- **Pre-deployment**: Full refund available
- **Post-deployment**: Prorated refund based on usage
- **Chargeback handling**: If user does chargeback, ban from platform + pursue dispute
- **Provider dispute**: Escrow protects users if service not delivered

### 6. Currency Support
**Question**: USD only or multi-currency?

**Recommendation**: Start USD-only
- Simpler implementation
- Most cloud services price in USD anyway
- Can add EUR, GBP later based on demand
- Stripe handles currency conversion if needed

### 7. Payment Timing for Rentals
**Question**: One-time vs recurring payments?

**Cloud rental scenarios**:
- **Short-term** (hourly/daily): Prepay or credit card hold + charge on use
- **Long-term** (monthly): Subscription with recurring billing
- **Pay-as-you-go**: Credit balance + automatic top-up

**Recommendation**: Hybrid approach
- **Prepaid credits**: Users load account balance via Stripe/crypto
- **Auto-topup**: Automatically charge when balance low
- **Recurring subscriptions**: For predictable long-term rentals

This mirrors AWS billing and is familiar to cloud users.

## Recommended Phased Approach

### Phase 0: Foundation (Before Any Code)

**Legal & Business Setup:**
1. Establish business entity if not already done
2. Get legal review on:
   - Terms of Service (especially for crypto conversion)
   - Refund/Dispute policy
   - Privacy Policy (payment data handling)
   - Regulatory requirements for your jurisdiction
3. Open Stripe account and complete verification
4. Decision on fiat-to-crypto conversion strategy
5. Set up exchange account if doing auto-conversion (Coinbase, Kraken, etc.)

**Design Decisions:**
1. Platform fee structure
2. Escrow vs direct payment model
3. Payment timing (prepaid, subscription, pay-as-you-go)
4. Refund policy
5. Currency support (USD-only initially recommended)

### Phase 1: Stripe MVP (Fiat Payments Only)

**Goal**: Enable card payments for cloud rentals, keep it simple.

**Backend (Rust):**
1. Design payment abstraction layer (common/src/payment_methods.rs)
2. Database schema updates:
   - Add payment_method, payment_status to contracts table
   - Add stripe_payment_intent_id, stripe_customer_id
   - Add escrow_status, escrow_release_at fields
3. Stripe API integration:
   - Consider `async-stripe` crate (unofficial but maintained)
   - Payment Intent creation
   - Webhook handler for payment events
4. Payment verification before contract acceptance
5. Unit tests for payment flows (TDD per AGENTS.md)

**Frontend (Svelte):**
1. Add `@stripe/stripe-js` dependency
2. Payment method selection UI in RentalRequestDialog
3. Card input component (Stripe Elements)
4. 3D Secure authentication handling
5. Payment status display
6. Tests for payment components

**Database:**
```sql
ALTER TABLE contracts ADD COLUMN payment_method TEXT;
ALTER TABLE contracts ADD COLUMN payment_status TEXT;
ALTER TABLE contracts ADD COLUMN stripe_payment_id TEXT;
ALTER TABLE contracts ADD COLUMN stripe_customer_id TEXT;
ALTER TABLE contracts ADD COLUMN escrow_status TEXT;
ALTER TABLE contracts ADD COLUMN escrow_release_at INTEGER;
```

**Testing:**
- Use Stripe test mode with test cards
- Small real transactions ($1-5) before going live
- Test refund flow end-to-end

**Rollout:**
- Launch as "beta" payment option
- Monitor for issues, chargebacks, disputes
- Iterate based on user feedback

### Phase 2: Crypto Payments (Optional/Secondary)

**Only after Stripe is stable and working well.**

**Add ICPay integration:**
- Reuse existing ICP wallet connection
- Add payment option in UI
- Track ICPay payment IDs in database
- Lower fees for crypto users (incentivize adoption)

**Add DCT native payments:**
- Direct ledger transfers (already partially implemented)
- Lowest fees (encourage platform token usage)

### Phase 3: Advanced Features

**Only after payment basics are solid:**
- Escrow automation with timeout/confirmation flows
- Recurring billing/subscriptions
- Credit balance + auto-topup
- Multi-currency support (EUR, GBP)
- Provider payout preferences
- Analytics dashboard for payment metrics
- Dispute resolution workflow

### Phase 4: Optimization

**When you have volume:**
- Negotiate better Stripe rates (volume discounts kick in at $1M+/year)
- Add alternative payment methods based on user requests
- Optimize fiat→crypto conversion (if doing that)
- Advanced fraud detection

## Final Recommendations Summary

### Start Here (Minimum Viable Payments)

1. **Primary payment method: Stripe**
   - Solves the "crypto scam" perception problem
   - Easiest for mainstream users (card payments)
   - Best developer experience
   - Handles compliance/PCI automatically

2. **Keep DCT as optional alternative**
   - For platform-native users
   - Lowest fees (incentivize adoption)
   - Maintain token utility

3. **Skip these for now:**
   - ICPay - too niche, doesn't solve card payment problem
   - MoonPay - high fees (4.5%), not designed for merchant payments
   - NOWPayments - crypto-only, no card support
   - PayPal - worse developer experience than Stripe for same fees

### Success Metrics

Track these after launch to validate payment integration:
- Payment success rate (target: >95%)
- Chargeback rate (target: <0.5%)
- Average transaction value
- Payment method distribution (Stripe vs DCT)
- User complaints about payment friction

### When to Revisit Crypto Payments

Add ICPay/other crypto options if:
- 20%+ of users explicitly request crypto payment option
- Stripe fees are eating too much margin
- You have significant IC ecosystem adoption
- Regulatory environment changes to favor crypto

But for mainstream adoption, Stripe is the pragmatic choice. Don't let perfect (decentralized payments) be the enemy of good (payments that work for normal people).

## References

- ICPay Docs: https://docs.icpay.org/
- Stripe Docs: https://stripe.com/docs
- Stripe Pricing: https://stripe.com/pricing
- PayPal Developer: https://developer.paypal.com/
- Internet Computer Payment Integration: https://internetcomputer.org/docs/current/developer-docs/integrations/
- MoonPay: https://www.moonpay.com/
- NOWPayments: https://nowpayments.io/
- Stripe Rust Crate: https://github.com/arlyon/async-stripe
- Payment Gateway Comparisons (2025): Multiple sources via web-search-prime

## Research Sources

This document was compiled from:
- Direct documentation review of ICPay, Stripe, PayPal
- 2025 payment gateway comparison articles
- Crypto payment gateway research
- Stripe account termination risk analysis
- SaaS payment provider recommendations
- Merchant of record platform analysis (Paddle, Lemon Squeezy)

## Appendix: Code Examples

### ICPay Payment (Frontend)

```typescript
import { Icpay } from '@ic-pay/icpay-sdk'

const icpay = new Icpay({
  publishableKey: process.env.NEXT_PUBLIC_ICPAY_PK!,
  actorProvider: (canisterId, idl) => walletSelect.getActor({...}),
  connectedWallet: { owner: principalId },
})

const payment = await icpay.createPaymentUsd({
  usdAmount: 100,
  ledgerCanisterId: 'ryjl3-tyaaa-aaaaa-aaaba-cai', // ICP
  symbol: 'ICP',
  metadata: { contractId: '12345' },
})
```

### Stripe Payment (Frontend)

```typescript
import { loadStripe } from '@stripe/stripe-js'

const stripe = await loadStripe(PUBLIC_STRIPE_PUBLISHABLE_KEY)

const { error } = await stripe.confirmCardPayment(clientSecret, {
  payment_method: {
    card: cardElement,
    billing_details: { name: 'Customer Name' }
  }
})
```

### Payment Abstraction (Backend - Conceptual)

```rust
// Verify payment regardless of method
pub async fn verify_payment(
    payment_method: &PaymentMethod,
    expected_amount: u64,
) -> Result<PaymentResult, PaymentError> {
    match payment_method {
        PaymentMethod::DCToken { amount_e9s } => {
            verify_dct_transfer(*amount_e9s, expected_amount).await
        }
        PaymentMethod::Stripe { payment_intent_id, .. } => {
            verify_stripe_payment(payment_intent_id).await
        }
        PaymentMethod::ICPay { payment_id, .. } => {
            verify_icpay_payment(payment_id).await
        }
    }
}
```

---

**Document Owner**: Research conducted by Claude Code
**Last Updated**: 2025-11-14
**Status**: Awaiting business decisions before implementation
