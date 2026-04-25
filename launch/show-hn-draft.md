# Show HN draft -- Decent Agents

**Status:** draft for founder review. Do not post until landing page (#417) and onboarding (#418) are live and the demo video has been recorded against the cold path.

**Posting window:** Tue/Wed/Thu, ~13:00--15:00 UTC (08:00--10:00 EST). Be online for 6+ hours after posting.

---

## Title

`Show HN: Decent Agents -- rent an AI engineer for your GitHub repo, CHF 49/month`

(72 chars. HN title cap is 80. Concrete + price. No fluff word.)

## URL field

`https://agents.decent-cloud.org` (placeholder; switch to final URL on post)

## Text field (~500 words)

I am a solo dev in Switzerland and I built this because my own GitHub backlog kept growing while I shipped other things. After a few months of dogfooding I cleaned it up and put a price on it.

What it is:

- You install a GitHub App on one of your repos.
- You get a dedicated Claude-Sonnet-backed agent identity. Not a queue, not a shared pool -- one container, one HOME dir, one PAT.
- The agent watches issues and `@mention` comments. When triggered, it opens a PR. You review the PR like any other.
- CHF 49/month. 1 agent. 20 active hours/month. 3M Sonnet tokens/month (overage at 1.5x Anthropic rate). EU-hosted on Hetzner. Cancel any time, prorated refund on unused days.

What it is not:

- Not a coding IDE. There is no editor. The interface is GitHub.
- Not a multi-agent swarm. One agent per subscription. If you want N agents, you buy N subscriptions.
- Not a platform-locked sandbox. The agent works on your repo, in your GitHub org, under a PAT you can revoke.
- Not free. There is no free tier today. I will probably add one later but I want pricing signal first.

What is not built yet (be brutally honest):

- No bring-your-own-Anthropic-key. The platform key is shared, capped per subscription. BYOK is on the list.
- No GitLab / Bitbucket / Gitea. GitHub only.
- No team seats, no SLA, no SOC2. Solo operator. If you need any of that, this is not for you yet.
- No mobile UI. Dashboard is desktop-only.

Why now: Sonnet got good enough at multi-step PR work over the last 6 months that an unsupervised agent on a real repo produces useful diffs >50% of the time in my own usage. That is the threshold where it stops being a toy.

Why CHF and EU: I am in Switzerland. The infra is in Falkenstein, DE. Stripe billing in CHF. Customers outside DACH still pay; Stripe converts. I am not pretending to be a US company.

Stack: Hetzner VMs, Docker sandbox per agent identity, GitHub App webhook, Rust API, SvelteKit dashboard, Stripe billing. Everything I would build for myself.

Demo (60s, cold path): `<demo-video-link-placeholder>`
Pricing + signup: `<landing-page-link-placeholder>`
Source for the underlying agent runtime is open: `https://github.com/decent-stuff/decent-cloud`

Honest expectation: I think this is useful for teams of 1--20 engineers who already trust Claude and have a real backlog. I think it is wrong for solo hobbyists at CHF 49 and wrong for enterprises at no-SLA. Tell me if I am wrong.

I will be in this thread all day. Brutal feedback welcome.

---

## FAQ-ready replies

Pre-drafted replies for the comments you are statistically likely to get. Edit live; do not paste verbatim. Keep replies under 4 lines each.

### 1. "Why CHF 49? That is more expensive than Cursor / Copilot / etc."

Cursor and Copilot are IDE assistants priced for individuals. This is a server-side agent that runs without you. The cost driver is Claude API tokens; CHF 49 covers up to 3M Sonnet tokens, which is roughly CHF 32 of API at list price plus infra and margin. If you do not use 3M tokens you are overpaying; cancel any time.

### 2. "How is this different from Devin / Replit Agent / Cognition?"

Three things: (1) bring-your-own-repo on actual GitHub, no platform sandbox; (2) EU-hosted, EU billing; (3) one fixed price, capped, no surprise overage. I do not have Devin's funding so I cannot match its breadth. I can match honesty about what it does.

### 3. "What about hallucinations / agents that destroy your repo?"

The agent only ever opens PRs. It does not push to default branches, does not force-push, does not delete branches it did not create. PRs are reviewed by a human (you). The PAT scope is repo-level and you can revoke at any time. I have run this against my own monorepo for ~3 months without an incident I would call destructive; I have had plenty of bad PRs that I closed.

### 4. "Why should I trust a solo founder with my code?"

You should not, more than the trust the GitHub App permissions and Hetzner ToS provide you. Read-only on issues, read/write on PRs and contents, scoped to repos you install on. If I disappear tomorrow you uninstall the app and the worst case is a half-finished PR. I am not asking for SSH or full org access.

### 5. "Can I run the agent on my own infra?"

Not today. The runtime (`dc-agent` + dispatcher) is open source so technically yes, but I do not support self-hosted as a product. If there is enough demand I will package it.

### 6. "What if the Anthropic API breaks?"

Soft failure: dispatcher reschedules, you get a status banner on the dashboard. Hard failure (Anthropic outage > 24h): agent runs queue and resume; if outage extends beyond your billing month, prorated refund. I do not have a fallback to a different model today; that is on the list.

### 7. "Why EU-hosted? GDPR theatre?"

Two real reasons: (1) my customers in DACH have actual procurement requirements that exclude US infra for source code; (2) latency to GitHub.com from Falkenstein is fine and the data residency story is simpler. If you do not care about EU hosting, this offering is not better than a US competitor on that axis.

### 8. "20 hours / 3M tokens -- is that enough?"

For a typical 5-engineer team filing 10--20 issues a week and using the agent on the easy 30%, yes. For a backlog-burndown sprint where you point the agent at 200 stale issues, no. Caps exist so I can size capacity and not get bankrupt by an outlier customer. Overage is at 1.5x Anthropic list, not punitive.

### 9. "Open source the whole thing?"

Runtime is open. The provisioning, billing, and dashboard are not, today. I am one person. If usage stabilises and there is community pull I will reconsider.

### 10. "How do I trial it without paying?"

You do not, today. CHF 49 first month, prorated refund any time. I considered a free tier but free Claude tokens are how solo founders go bankrupt. If you want a 15-min walkthrough on a Zoom call before signing up, DM me.

### 11. "What is your moat?"

I do not have a moat. I have a product that works for a specific buyer (1--20 person dev teams in Europe) and a price that is honest. If a bigger company copies this, fine. If they do not, I have a real business.

### 12. "Show me the code that runs the agent."

`https://github.com/decent-stuff/decent-cloud` -- specifically `tools/team/dispatcher.py` and `dc-agent/`. The opencode runtime integration is in `repo/agent/opencode_bridge.js`.
