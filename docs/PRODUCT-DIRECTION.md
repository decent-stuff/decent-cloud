# Product Direction — decent-cloud

**Status:** Authoritative north star. **All product decisions, designs, and code changes must
align with this document.** When a change conflicts with it, flag it for the operator rather than
proceeding. Last updated 2026-08-03 from the operator's strategic guidance.

> This is the human-expectations document referenced from `AGENTS.md`. Keep it current whenever the
> operator gives new product direction.

## The vision

**decent-cloud is "OpenRouter, but for cloud resources."**

A proxy / reselling platform where a user discovers compute from **many** cloud providers and
provisions it through **one common API**, one account, one bill, one trust/reputation system. The
platform sits between the user and the upstream providers: it abstracts their divergent APIs behind
a uniform catalog + lifecycle, resells/allocates capacity, and presents a single trustworthy
surface.

Just as OpenRouter unified LLM access across model providers behind one API, decent-cloud unifies
cloud-resource access across compute providers behind one API.

## The first concrete step (near-term)

1. **Drop the demo/synthetic offerings.** The marketplace currently ships placeholder rows seeded
   from a fake provider key. They look real and mislead. Remove them so the catalog is honest.
2. **Add the first REAL offerings**, provided by the operator personally, by **proxying / reselling
   Hetzner**. The operator becomes the platform's first real provider. The platform already has a
   working Hetzner provisioner (`dc-agent` + `api` `provisioner/hetzner`); this step is about
   listing real, purchasable, provisionable offerings backed by real Hetzner capacity — not new
   provisioning code.
3. Until real offerings exist, the marketplace shows an **honest empty state** (never fake data).

## The long-term platform

The same reselling path the operator takes for Hetzner must generalize: **anyone** should be able to
proxy & resell cloud resources from arbitrary providers through the platform, surfacing them in the
unified catalog. The platform's value is the common API + the aggregation + the trust layer on top.

Direction-guided implications:

- **Provider identity matters.** Real provider/company names (collected at onboarding) are shown,
  not auto-generated handles. (The marketplace now does this; keep it that way.)
- **Trust & reputation are central.** A platform whose entire promise is "one trustworthy surface
  for many providers" must make reputation prominent and honest: a top-providers leaderboard so
  reputation is browseable by default, and an honest N/A score when there is no track record (never
  a green "Reliable" badge on empty data).
- **"Become a Provider" must mean real onboarding** to the technical path (install the agent,
  register a pool, list an offering), not just a support-profile wizard.
- **One provider's offering must be indistinguishable in quality of presentation from another's.**
  The catalog is the product.

## Alignment checklist (apply to every change)

Before shipping, confirm the change moves the platform toward this vision:

- [ ] Does it make the catalog more honest / more real? (Remove demos; add real offerings; never
      show fake or placeholder provider data.)
- [ ] Does it advance the single-common-API abstraction (more providers behind one uniform surface)?
- [ ] Does it make trust/reputation more prominent and more honest?
- [ ] Does it lower the friction for a new real provider to list offerings?
- [ ] Does it avoid building provider-specific special-cases that the common abstraction should own?

If a change is neutral or negative on these, it is likely technical debt or scope creep — reconsider.

## Explicit non-goals (for now)

- **ICP as a payment rail or offering currency** — retired. Stripe-supported currencies only.
- **Maintaining the demo/synthetic catalog** — being removed.
- **Multi-secret-store fragmentation** — the k8s consolidation (PR #454) collapses staging onto one
  GitOps-managed store; do not reintroduce parallel secret stores.

## Related specs (background, not the direction)

These predate and inform the direction; they describe mechanics, not the north star:

- `docs/specs/2026-02-14-hetzner-provisioner.md` — the Hetzner provisioner the first real offerings reuse.
- `docs/specs/2026-02-14-self-provisioning-platform.md` — self-provisioning foundation.
- `docs/specs/2025-12-07-reseller-infrastructure-spec.md` — early reseller-infra design.
- `docs/reputation.md` — the reputation system the leaderboard surfaces.
