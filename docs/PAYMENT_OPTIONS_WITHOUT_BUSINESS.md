# Payment Options for Individuals Without Business Registration

**Date**: 2025-11-14
**Context**: You want to accept payments before registering a formal business entity to avoid upfront costs

## TL;DR - Best Options for You

### Recommended: Merchant of Record Platforms

**Top Choice: Lemon Squeezy** or **Paddle**
- ✅ No business entity required
- ✅ They become the "merchant of record" (handle all compliance/taxes)
- ✅ Accept card payments globally
- ✅ 2-3 day approval time
- ❌ Higher fees: 5% + $0.50 per transaction
- ❌ Product restrictions (no physical goods, some services rejected)

### Alternative: Stripe as Sole Proprietor

**Stripe with your SSN**
- ✅ No LLC/business required - use "Sole Proprietor" business type
- ✅ Industry-standard 2.9% + $0.30 fees (cheaper than MoR)
- ✅ Best developer experience
- ✅ Can upgrade to business later seamlessly
- ⚠️ You're responsible for taxes/compliance
- ⚠️ May flag your account if volume spikes (crypto-adjacent risk)

### For Marketplace: Stripe Connect Express

**Stripe Connect Express Accounts**
- ✅ Platform doesn't need business entity
- ✅ Stripe handles seller verification/compliance
- ✅ Moderate integration complexity
- ✅ Split payments, escrow, payouts built-in
- ⚠️ Small per-account fee (~$2/month per active seller)
- ⚠️ Platform still needs some verification

---

## Detailed Analysis

### Option 1: Merchant of Record (Lemon Squeezy, Paddle, Gumroad)

#### What is a Merchant of Record?

A merchant of record (MoR) **becomes the seller** on behalf of your business. They:
- Accept payments from customers
- Handle all tax calculations and remittance globally
- Manage refunds, chargebacks, fraud
- Take on liability and compliance burden
- Pay you out periodically (minus their fee)

**You're selling TO the MoR, who then sells to the end customer.**

#### Lemon Squeezy

**Requirements:**
- Fill out questionnaire about your business/customers
- Verify identity with government ID
- 2-3 business day approval (can vary)
- NO formal business entity required
- KYC/KYB checks performed

**Fees:**
- 5% + $0.50 per transaction
- Handles all tax/VAT globally
- No monthly fees

**Best for:**
- Digital products (SaaS, software, ebooks, courses)
- International sales (they handle all tax compliance)
- Individuals who don't want tax complexity

**Restrictions:**
- May reject service-based businesses
- Physical goods typically not accepted
- Must comply with their terms (review takes 2-3 days)

**Verdict**: Great for getting started quickly without business entity, but fees are high (almost 2x Stripe).

#### Paddle

**Similar to Lemon Squeezy:**
- Merchant of record model
- 5% + $0.50 transaction fees
- Handles global tax/compliance
- KYC/identity verification required
- Digital products focus (SaaS/software)

**Differences:**
- More enterprise-focused
- Better for recurring/subscription revenue
- Local payment methods (Pix, UPI, BLIK, etc.)

**Verdict**: Solid choice, similar trade-offs to Lemon Squeezy.

#### Gumroad

**Requirements:**
- Super simple setup as individual
- No business entity needed
- Verify identity

**Fees:**
- 10% of sales (free plan)
- OR $10/month + 3.5% + $0.30 (paid plan)

**Best for:**
- Creators selling digital products
- Very simple use case
- Lowest setup friction

**Restrictions:**
- Less feature-rich than Lemon Squeezy/Paddle
- Not ideal for complex SaaS/marketplaces
- Higher fees on free plan

**Verdict**: Too simple for a marketplace platform like Decent Cloud. Skip this.

---

### Option 2: Stripe as Sole Proprietor

#### What You Need

**To open Stripe account as individual:**
- Full legal name
- Email address
- Social Security Number (SSN) - NO EIN required
- Date of birth
- Address
- Business type: Select "Sole Proprietor" or "Individual"

**You do NOT need:**
- LLC or incorporated business
- EIN (Employer Identification Number)
- Business license
- Formal registration

#### How It Works

1. Sign up at stripe.com
2. Select "Sole Proprietor" as business type
3. Provide SSN for tax reporting (Stripe reports to IRS)
4. Verify identity (may ask for ID photo)
5. Start accepting payments

**Tax implications:**
- Stripe reports income to IRS using your SSN
- You report on Schedule C of personal tax return
- You're responsible for sales tax compliance (if applicable)

#### Fees

- **Standard**: 2.9% + $0.30 per transaction
- No monthly fees
- Volume discounts available at higher volume

**Compared to Merchant of Record:**
- Stripe: $2.90 + $0.30 = **$3.20 on $100 sale**
- Lemon Squeezy: $5.00 + $0.50 = **$5.50 on $100 sale**
- **Savings: $2.30 per $100 transaction (72% more expensive with MoR)**

#### Risks

**Account freezing/termination:**
- Crypto-adjacent businesses are higher risk for Stripe
- Can freeze account if sudden volume spike
- Can hold funds 30-180 days if terminated
- Positioning as "cloud marketplace" vs "crypto platform" helps

**Compliance burden:**
- You're responsible for collecting/remitting sales tax
- Must track income for tax filing
- Handle chargebacks/disputes yourself

#### Upgrade Path

**Start as sole proprietor, upgrade later:**
- Can convert to business account when you register LLC
- Seamless transition, keep same account
- No customer impact

**Verdict**: Best balance of cost and features if you're willing to handle compliance yourself.

---

### Option 3: PayPal Business (Sole Proprietor)

#### Requirements

**To open PayPal Business account as sole proprietor:**
- Legal name
- Email address
- SSN (Social Security Number) - NO EIN required
- Address
- Business type: Select "Sole Proprietor / Individual"
- Business name (can use your own name)

**You do NOT need:**
- Formal business registration
- EIN
- Business license

#### Fees

- **Standard**: 2.9% + $0.30 per transaction (same as Stripe)
- **Alternative pricing available**: 2.29% + fixed fee on volume
- Slightly cheaper than Stripe for $100+ transactions

**For $100 transaction:**
- PayPal: $2.38 (cheaper)
- Stripe: $2.75

#### Pros vs Stripe

- Slightly lower fees
- More familiar to general public (trust factor)
- Buyer protection (users feel safe)
- International coverage

#### Cons vs Stripe

- Much worse developer experience
- Less feature-rich API
- Known for account freezes/holds (similar to Stripe)
- Redirects users away from your site during checkout
- Less suitable for embedded payment flows

#### Verdict

**Skip PayPal for Decent Cloud** because:
- Your users are technical (cloud providers/renters), not general public
- You need programmatic API integration
- Stripe's developer experience is vastly better
- Fee difference is minimal (~$0.37 on $100)

---

### Option 4: Stripe Connect for Marketplace

#### Three Account Types

Stripe Connect offers three models for marketplace platforms:

##### Standard Accounts
**How it works:**
- Each provider creates their own Stripe account
- Platform just facilitates connection
- Providers handle their own compliance/verification

**Pros:**
- Simplest for platform
- No additional fees beyond standard Stripe rates
- Platform has minimal liability

**Cons:**
- Each provider must set up Stripe independently (friction)
- Platform has less control
- Harder to maintain consistent UX
- Not ideal for small/casual providers

**Verdict**: Too much friction for Decent Cloud providers.

##### Express Accounts (Recommended for Marketplaces)
**How it works:**
- Platform creates Stripe accounts on behalf of providers
- Stripe handles verification/compliance
- Platform customizes onboarding flow
- Shared control between platform and Stripe

**Requirements:**
- Platform needs Stripe account (can be sole proprietor!)
- Providers need to verify identity
- Platform handles UX, Stripe handles compliance

**Fees:**
- Standard processing fees (2.9% + $0.30)
- ~$2/month per active connected account
- **For 10 active providers: $20/month extra**

**Pros:**
- ✅ Stripe handles all provider verification/compliance
- ✅ Platform can brand the experience
- ✅ Built-in split payments, escrow, payouts
- ✅ Moderate integration effort
- ✅ Perfect for marketplaces like Decent Cloud

**Cons:**
- Per-account fees add up (but small)
- Some shared control with Stripe

**Verdict**: **This is likely your best option for marketplace payments.**

##### Custom Accounts
**How it works:**
- Platform has complete control
- Platform handles all compliance/verification
- Fully white-labeled

**Cons:**
- Highest fees
- Significant integration complexity
- Platform responsible for compliance
- Overkill for early-stage marketplace

**Verdict**: Skip this unless you're at scale.

---

## Decision Matrix for Decent Cloud

### Scenario A: You Want Simplest Path (No Compliance Burden)

**Choose: Lemon Squeezy or Paddle**

**Pros:**
- Start accepting payments in 2-3 days
- Zero compliance/tax burden
- No business entity needed
- They handle everything

**Cons:**
- High fees (5% + $0.50) eat into margins
- Less control over payment flow
- May reject your marketplace use case (they prefer simple digital products)

**Cost for $100 rental:**
- User pays: $100
- Lemon Squeezy takes: $5.50
- You receive: $94.50
- Provider receives: ~$90 (after platform fee)

### Scenario B: You Want Lower Fees + Control

**Choose: Stripe as Sole Proprietor**

**Pros:**
- Much lower fees (2.9% + $0.30)
- Best developer experience
- Full control
- Can upgrade to business later

**Cons:**
- You handle tax compliance
- Account freeze risk (crypto-adjacent business)
- Need to track income for taxes

**Cost for $100 rental:**
- User pays: $100
- Stripe takes: $3.20
- You receive: $96.80
- Provider receives: ~$92 (after platform fee)

**Savings vs Lemon Squeezy: $2.30 per $100 (41% more profit)**

### Scenario C: You Want Proper Marketplace Solution (Recommended)

**Choose: Stripe Connect Express**

**Why this is best for Decent Cloud:**

1. **Platform can start as sole proprietor** (no business entity needed initially)
2. **Stripe verifies all providers** (you don't handle KYC/compliance)
3. **Built for marketplaces** (split payments, escrow, payouts native)
4. **Moderate complexity** (well-documented, manageable integration)
5. **Reasonable fees** (2.9% + $0.30 + $2/provider/month)

**How it works:**
```
User rents server from Provider
    ↓
User pays $100 via Stripe
    ↓
Stripe takes $3.20 (fee)
    ↓
Platform receives $96.80
    ↓
Platform holds in escrow (or immediately splits)
    ↓
Platform fee: $7-10 (configurable)
    ↓
Provider receives: $87-90 via Stripe transfer
```

**Provider onboarding:**
- Provider clicks "Register as Provider" in your app
- Stripe Express onboarding flow (embedded in your site)
- Provider provides ID, bank account
- Stripe verifies (takes 1-3 days)
- Provider can start receiving payments

**Monthly costs:**
- 10 active providers: $20/month
- 50 active providers: $100/month
- 100 active providers: $200/month

**Scales well as you grow.**

---

## Honest Recommendations for Decent Cloud

### Phase 0: Pre-Launch / MVP Testing

**Use: Lemon Squeezy**

**Why:**
- Get started TODAY (2-3 day approval)
- Zero compliance burden
- Test market without legal setup
- Validate pricing/demand

**When to switch:**
- After you have 10-20 paying customers
- OR after you validate the business model
- OR when fees become painful (5% is a lot)

### Phase 1: Early Growth (First 50 Customers)

**Use: Stripe as Sole Proprietor**

**Why:**
- Lower fees (save ~$2.30 per $100)
- Better developer control
- Can upgrade to LLC later seamlessly
- More professional integration

**Costs saved:**
- 50 customers @ $100/month avg
- Lemon Squeezy fees: $275/month
- Stripe fees: $160/month
- **Savings: $115/month** (enough to matter)

**Tax implications:**
- Report income on Schedule C
- Quarterly estimated tax payments if >$1k annual profit
- Track expenses for deductions

### Phase 2: True Marketplace (50+ Providers)

**Use: Stripe Connect Express**

**Why:**
- Proper marketplace infrastructure
- Provider payouts automated
- Compliance handled by Stripe
- Scales to hundreds of providers

**Migration:**
- Keep your Stripe account (upgrade to Connect)
- Onboard providers to Express accounts
- Implement split payment flow
- Add escrow logic

**When to form LLC:**
- Revenue >$50k/year (tax benefits kick in)
- Want liability protection
- Seeking investment
- Hiring employees

**Stripe transition:**
- Contact Stripe support
- Update business type from Sole Prop to LLC
- Update EIN (get one for LLC)
- NO customer disruption

---

## Step-by-Step: Getting Started Today

### Option A: Lemon Squeezy (Fastest)

1. Go to lemonsqueezy.com
2. Create account with email
3. Fill out business questionnaire (be honest about marketplace)
4. Upload government ID for verification
5. Wait 2-3 days for approval
6. Integrate API (TypeScript SDK available)
7. Start accepting payments

**Timeline: 3-5 days to first payment**

### Option B: Stripe Sole Proprietor (Best Long-term)

1. Go to stripe.com
2. Sign up with email
3. Select "Individual" or "Sole Proprietor" business type
4. Provide SSN, name, address, DOB
5. Verify identity (may request ID photo)
6. Add bank account for payouts
7. Get API keys (test mode immediately available)
8. Integrate Stripe SDK
9. Test with test cards
10. Activate live mode (instant after verification)

**Timeline: 1-2 days to first payment**

### Option C: Stripe Connect Express (Marketplace)

1. Complete Option B (get Stripe account as sole prop)
2. Contact Stripe support to enable Connect
3. Choose Express account type
4. Implement Connect onboarding flow in your app
5. Test with test provider accounts
6. Launch provider onboarding

**Timeline: 1-2 weeks for full integration**

---

## Compliance & Tax Considerations

### As Sole Proprietor

**What you're responsible for:**
- Income tax on net profit (report on Schedule C)
- Self-employment tax (15.3% on net profit)
- Sales tax (if applicable in your state - likely NO for cloud services)
- Quarterly estimated tax payments if profit >$1k/year

**What Stripe/PayPal handle:**
- Payment processing compliance (PCI-DSS)
- Fraud detection
- Chargeback management
- 1099-K reporting to IRS (if >$5k in transactions)

**What you DON'T need to worry about (for cloud marketplace):**
- Sales tax in most states (cloud services typically exempt)
- International VAT (if under thresholds - check your country)

### With Merchant of Record

**They handle:**
- All tax calculations
- VAT/GST globally
- Tax remittance
- Compliance burden

**You handle:**
- Income tax on your payout (what they send you)

---

## Risk Mitigation Strategies

### Stripe Account Freeze Risk

**How to minimize:**
1. **Position correctly**: Emphasize "cloud marketplace" not "crypto platform"
2. **Gradual growth**: Don't go $0 → $50k overnight (triggers fraud alerts)
3. **Clear descriptions**: Transparent product descriptions, ToS, refund policy
4. **Low chargeback rate**: Aim for <0.5% chargebacks
5. **Respond quickly**: Answer Stripe requests same-day
6. **Have backup**: Keep Lemon Squeezy or PayPal as backup payment method

**If account frozen:**
- Funds typically held 30-90 days
- Can appeal with business documentation
- Having backup payment method = business continuity

### Compliance Risk

**Things that could get you in trouble:**
- NOT reporting income to IRS (Stripe will report, you must too)
- Ignoring sales tax obligations (research your state)
- Not handling customer data properly (GDPR if EU customers)

**Mitigations:**
- Use accounting software (QuickBooks Self-Employed, Wave)
- Consult tax professional once profitable
- Have clear Privacy Policy + Terms of Service
- Keep business/personal expenses separate (even as sole prop)

---

## Cost Comparison Summary

### For $10,000/month in rental transactions:

| Provider | Transaction Fee | Monthly Cost | Platform Receives | Savings vs Lemon Squeezy |
|----------|----------------|--------------|-------------------|------------------------|
| **Lemon Squeezy** | 5% + $0.50 | ~$550 | $9,450 | Baseline |
| **Paddle** | 5% + $0.50 | ~$550 | $9,450 | $0 |
| **Stripe (Sole Prop)** | 2.9% + $0.30 | ~$320 | $9,680 | **+$230/month** |
| **PayPal (Sole Prop)** | 2.29% + $0.30 | ~$268 | $9,732 | **+$282/month** |
| **Stripe Connect Express** | 2.9% + $0.30 + $2/provider | ~$320 + $20* | $9,660 | **+$210/month** |

*Assumes 10 active providers

**At $10k/month revenue, Stripe saves you $2,760/year vs Merchant of Record.**

---

## Final Recommendation

### For Decent Cloud Specifically:

**Start with: Stripe Connect Express as Sole Proprietor**

**Why:**
1. ✅ No business entity needed (use SSN)
2. ✅ Built for marketplace use case
3. ✅ Much lower fees than MoR (save $200+/month at scale)
4. ✅ Professional solution that scales
5. ✅ Stripe handles provider verification/compliance
6. ✅ Can upgrade to LLC later seamlessly

**Migration path:**
```
Now: Stripe Connect Express (Sole Prop)
    ↓
$50k/year: Form LLC, update Stripe account
    ↓
$500k/year: Consider Stripe Embedded Finance / Banking
    ↓
$5M/year: Negotiate custom pricing, consider alternatives
```

**Integration effort:**
- 1-2 weeks for full Connect integration
- Well-documented, TypeScript SDK available
- Fits your Rust backend + Svelte frontend architecture

**Risk mitigation:**
- Keep Lemon Squeezy integrated as backup payment method
- Position as "decentralized cloud marketplace" not "crypto platform"
- Gradual growth plan (don't spike from $0 to $100k overnight)

---

---

## Appendix: Why NOT Adyen, Payoneer, or Square

### Adyen - ❌ NOT Suitable for Early-Stage

**What it is:**
- Enterprise-focused payment platform
- Built for large companies (Uber, Spotify, Microsoft)
- Handles high-volume, complex global payments

**Requirements:**
- ❌ **Minimum $120/month invoice** regardless of volume
- ❌ **Minimum processing volume requirements** (industry-specific)
- ❌ **Enterprise-level verification** (formal business entity likely required)
- ❌ **Complex onboarding** process
- ❌ **Higher technical complexity** than Stripe

**Fees:**
- Interchange++ pricing (variable, complex)
- ~0.6% markup + interchange costs
- Currency conversion: 0.6-1.2%
- Minimum monthly fees

**Best for:**
- Enterprises processing >$1M/month
- Companies needing ultra-complex payment flows
- Businesses with dedicated payment operations team

**Verdict for Decent Cloud:**
**Skip entirely.** Adyen is overkill and has minimums you won't meet. It's designed for Uber-scale companies, not early-stage marketplaces.

---

### Payoneer - ❌ NOT a Payment Gateway

**What it is:**
- Cross-border payment/payout platform
- Designed for receiving payments from marketplaces (Upwork, Fiverr, Amazon)
- NOT a merchant payment gateway

**What it does:**
- Receive payouts from platforms
- Send payments to freelancers/vendors
- Multi-currency accounts
- Virtual bank accounts

**What it does NOT do:**
- ❌ Accept credit card payments from end customers
- ❌ Provide payment gateway for your website
- ❌ Enable marketplace split payments
- ❌ Process transactions on your platform

**Fees:**
- $29.95/year inactivity fee (if <$2k received)
- ~1% for currency conversion
- Varies by payment method

**Best for:**
- Freelancers receiving payments from platforms like Upwork
- Businesses paying international contractors
- Cross-border B2B payments

**Verdict for Decent Cloud:**
**Not applicable.** Payoneer is for receiving/sending payouts, not processing customer payments. It's the wrong tool for your use case.

**Could be useful later for:** Paying providers internationally if they prefer Payoneer over direct bank transfer.

---

### Square - ⚠️ Not Optimized for Online Marketplace

**What it is:**
- Point-of-sale (POS) focused payment platform
- Great for in-person retail/restaurants
- Has online payment capabilities but not core strength

**Sole Proprietor Support:**
- ✅ Yes, can use as individual with SSN
- ✅ Simple setup similar to Stripe
- ✅ No business entity required

**Fees:**
- **In-person**: 2.6% + $0.10 (cheaper than Stripe)
- **Online**: 2.9% + $0.30 (same as Stripe)
- No monthly fees

**Marketplace Features:**
- Has split payment capability (similar to Stripe Connect)
- Less mature than Stripe Connect for online marketplaces
- Developer experience not as polished as Stripe

**Best for:**
- Retail stores with in-person sales
- Restaurants/cafes
- Small businesses doing both in-person + online
- Simpler use cases than complex marketplaces

**Comparison to Stripe:**

| Feature | Square | Stripe |
|---------|--------|--------|
| **Online marketplace** | Possible but clunky | Purpose-built (Connect) |
| **Developer experience** | Good | Excellent |
| **Documentation** | Decent | Industry-best |
| **In-person payments** | Best-in-class | Available but not core |
| **API maturity (online)** | Good | Excellent |
| **Fees (online)** | 2.9% + $0.30 | 2.9% + $0.30 |
| **Marketplace split payments** | Available | Mature (Connect) |

**Verdict for Decent Cloud:**
**Choose Stripe instead.** Square is excellent for in-person retail but Stripe is better for online-first marketplaces. Same fees, but Stripe has:
- Better marketplace tooling (Connect is more mature)
- Superior developer experience
- More comprehensive documentation
- Purpose-built for online/SaaS use cases

**Only use Square if:** You plan to have in-person payment needs (which you don't for a cloud marketplace).

---

## Final Comparison Table: All Options

| Provider | Type | No Business Needed? | Marketplace Ready? | Fees (Online) | Best For | Verdict for Decent Cloud |
|----------|------|--------------------|--------------------|---------------|----------|--------------------------|
| **Stripe Connect Express** | Gateway | ✅ Yes (SSN only) | ✅ Yes (purpose-built) | 2.9% + $0.30 + $2/provider | Online marketplace | ⭐⭐⭐ **RECOMMENDED** |
| **Lemon Squeezy** | Merchant of Record | ✅ Yes | ⚠️ Maybe (not ideal) | 5% + $0.50 | Digital products | ⭐⭐ Good for MVP |
| **Paddle** | Merchant of Record | ✅ Yes | ⚠️ Maybe (not ideal) | 5% + $0.50 | SaaS subscriptions | ⭐⭐ Alternative to LS |
| **Stripe (Sole Prop)** | Gateway | ✅ Yes (SSN only) | ⚠️ Manual setup | 2.9% + $0.30 | Simple online sales | ⭐⭐ Works but not optimal |
| **PayPal Business** | Gateway | ✅ Yes (SSN only) | ❌ No native support | 2.9% + $0.30 | General e-commerce | ⭐ Poor developer UX |
| **Square** | POS + Gateway | ✅ Yes (SSN only) | ⚠️ Basic support | 2.9% + $0.30 | In-person + online | ⭐ Not optimized |
| **Adyen** | Enterprise Gateway | ❌ No (minimums) | ✅ Yes | $120/mo minimum | Enterprise (>$1M/mo) | ❌ **Skip - too enterprise** |
| **Payoneer** | Payout Platform | N/A | N/A | N/A | B2B payouts | ❌ **Wrong tool entirely** |
| **Gumroad** | Merchant of Record | ✅ Yes | ❌ No | 10% or $10+3.5% | Individual creators | ❌ Too simple |

---

## Why Stripe Connect Express Wins

**It's the only solution that checks all boxes:**

✅ **No business entity needed** (sole proprietor with SSN)
✅ **Purpose-built for marketplace** (split payments, escrow, provider payouts)
✅ **Reasonable fees** (2.9% + $0.30 + small per-provider fee)
✅ **Stripe handles provider compliance** (you don't KYC everyone yourself)
✅ **Scales from 1 to 10,000 providers** seamlessly
✅ **Best-in-class developer experience** (docs, SDKs, support)
✅ **Proven at scale** (Lyft, Instacart, Kickstarter use Connect)
✅ **Can upgrade to LLC later** without disruption

**The alternatives:**
- **Lemon Squeezy/Paddle**: 72% more expensive, not designed for marketplaces
- **Square**: Works but not optimized for online-first marketplaces
- **PayPal**: Worse developer experience for same fees
- **Adyen**: Enterprise minimums, overkill for your stage
- **Payoneer**: Wrong tool (not a payment gateway)

---

## Resources

### Stripe
- Stripe Connect Docs: https://stripe.com/docs/connect
- Sole Proprietor Guide: https://support.stripe.com/questions/selling-on-stripe-without-a-separate-business-entity
- Express Accounts: https://stripe.com/docs/connect/express-accounts
- Stripe Rust Crate: https://github.com/arlyon/async-stripe

### Merchant of Record
- Lemon Squeezy: https://www.lemonsqueezy.com/
- Paddle: https://www.paddle.com/
- Comparison: https://fungies.io/lemonsqueezy-vs-paddle/

### PayPal
- Business Account: https://www.paypal.com/us/business
- Sole Prop Guide: https://www.paypal.com/us/brc/article/what-is-a-sole-proprietor

### Square
- Square Online: https://squareup.com/us/en/online-checkout
- Developer Docs: https://developer.squareup.com/

### Tax/Legal
- Schedule C (Sole Prop Taxes): https://www.irs.gov/forms-pubs/about-schedule-c-form-1040
- Self-Employment Tax: https://www.irs.gov/businesses/small-businesses-self-employed/self-employment-tax-social-security-and-medicare-taxes
- When to Form LLC: https://www.nolo.com/legal-encyclopedia/seven-reasons-not-form-llc.html

---

**Document Status**: Complete - Research finished
**Next Step**: Choose payment provider and begin integration
**Recommendation**: Stripe Connect Express as Sole Proprietor
**Update Date**: 2025-11-14
