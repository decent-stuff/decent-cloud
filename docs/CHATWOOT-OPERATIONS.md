# Chatwoot Operations Runbook

Operator runbook for the self-hosted **Chatwoot** support integration in decent-cloud.
Every claim below is grounded in code with `file:line` references so it stays
auditable as the code moves.

> **TL;DR for on-call:** Chatwoot is an optional support subsystem. If it is
> down or misconfigured, rentals/provisioning are unaffected; only the in-app
> support widget, the AI support bot, and provider Help Centers degrade. The api
> stays up and loudly warns at boot when Chatwoot config is missing
> (`api/src/main.rs:1294-1303`).

---

## 1. What Chatwoot is in this stack

Chatwoot is a **self-hosted** customer-support platform (open-source, Rails) that
we run **alongside** the decent-cloud api — it is *not* a SaaS dependency and
*not* part of the api's own database.

| Aspect | Value / Location |
|--------|------------------|
| Where it runs | k8s `dc-prod` namespace (the manifest lives in the **external** ArgoCD repo `sasa-tomic/nuc-k3s`, `cluster/apps/decent-cloud/base/`, not in this product repo — `release.yml:234-237` clones & bumps it there). The local docker-compose equivalent is `cf/docker-compose.dev.yml:25-58`. |
| Public URL | `https://support.decent-cloud.org` (HTTP 200 when healthy) — exposed via the cloudflared tunnel; configured as `CHATWOOT_FRONTEND_URL`. |
| In-cluster URL (api → Chatwoot) | `http://dc-chatwoot-web.dc-prod.svc:80` — configured as `CHATWOOT_BASE_URL` so the api talks to Chatwoot without leaving the cluster (`cf/CONFIG.md:167`). |
| Chatwoot's own database | A **separate** Postgres DB `chatwoot_prod` on the prod host — *not* the api's `DATABASE_URL` (`cf/CONFIG.md:172`). |
| How the api talks to it | Two REST clients in `api/src/chatwoot/client.rs`: **Platform API** (`ChatwootPlatformClient`, admin ops) + **Account API** (`ChatwootClient`, agent-level ops). |
| How the website talks to it | The website embeds the Chatwoot **widget SDK** (`website/src/routes/+layout.svelte:5-26` → `website/src/lib/components/ChatwootWidget.svelte:34-74`). |

```
                         ┌───────────────────────────────┐
   browser (widget SDK)  │  https://support.decent-cloud.org   │
        ───────────────▶ │  Chatwoot web (k8s dc-prod)        │
   website token + HMAC  │  DB: chatwoot_prod (separate PG)  │
                         └───────────────┬───────────────────┘
                                         │ webhooks (message_created)
                                         ▼
                         ┌───────────────────────────────┐
                         │  decent-cloud api              │
                         │  POST /api/v1/webhooks/chatwoot│  api/src/openapi/webhooks.rs:531
                         │  → AI support bot / escalation │  api/src/support_bot/
                         └───────────────┬───────────────────┘
   Platform API (bot CRUD, users)        │ Account API (messages, inboxes, articles)
   client.rs:19  ────────────────────────┘ client.rs:429
```

The api **auto-configures** its Agent Bot on every boot using the Platform API,
then assigns it to every inbox via the Account API (`api/src/main.rs:1231-1303`).
This is why a freshly-deployed api "just works" against an already-set-up
Chatwoot — provided the three operator tokens are valid.

---

## 2. Credential table

All `CHATWOOT_*` keys, grouped by who owns them. Source of truth for prod wiring
is `cf/CONFIG.md:163-177` (the `D`=dev / `sec`=SOPS-secret / `lit`=ConfigMap-literal
annotations) plus the code reads cited below.

### 2a. decent-cloud operator-side credentials (live in the api's `dc-secret`/`dc-config`)

| Key | Secret or config? | What it's for | Where obtained | How to rotate |
|-----|-------------------|---------------|----------------|---------------|
| `CHATWOOT_BASE_URL` | **config** (`lit`) | Internal base the api's two REST clients use to reach Chatwoot. Prod: `http://dc-chatwoot-web.dc-prod.svc:80`. | k8s service DNS. | Only changes if the Chatwoot k8s service moves. Edit `dc-config`, restart api. |
| `CHATWOOT_FRONTEND_URL` | **config** (`lit`) | Public HTTPS URL. Used for the Help Center article API (rejects internal hostnames) and the support-portal login URL surfaced to users. Prod: `https://support.decent-cloud.org`. | Your public Chatwoot hostname. | Only changes if the public hostname moves. Edit `dc-config`, restart api. Read at `client.rs:503`, `api/src/openapi/chatwoot.rs:110,229,343`. |
| `CHATWOOT_ACCOUNT_ID` | **config** (`lit`) | The Chatwoot account the api operates on. Almost always `1`. | Chatwoot admin UI. | Set once at setup (`client.rs:89,506`, `main.rs:850`). |
| `CHATWOOT_API_TOKEN` | **secret** (`sec`) | **Account API** token — an agent/admin user's access token. Used to list/assign agent bots to inboxes, send messages, manage Help Center articles. | Chatwoot admin → Profile → "Access Token" (Account-scope). | See §4. Read at `client.rs:505`, doctor `main.rs:848`. |
| `CHATWOOT_PLATFORM_API_TOKEN` | **secret** (`sec`) | **Platform API** token — from the SuperAdmin "Platform App". The *only* token that can create users, reset passwords, and CRUD agent bots. | Chatwoot SuperAdmin → Applications → Platform App → token. | See §4. Read at `client.rs:87`, boot `main.rs:1236-1244`, doctor `main.rs:860`. |
| `CHATWOOT_HMAC_SECRET` | **shared secret** (`sec`) | Symmetric HMAC-SHA256 key used to sign widget user identities (`api/src/chatwoot/hmac.rs:8-13`, consumed at `api/src/openapi/chatwoot.rs:73-85`). **Must match the value injected into the Chatwoot pod** as `CHATWOOT_INBOX_HMAC_SECRET_KEY` (`cf/docker-compose.dev.yml:42`; prod manifest does the same mapping in the external nuc-k3s repo). If the two drift, logged-in users cannot be verified in the widget. | `openssl rand -hex 32`. | **Rotate on BOTH sides in lock-step** — see §4. Read/checked at `main.rs:855`. |
| `API_PUBLIC_URL` | **config** (`lit`) | Not `CHATWOOT_*`-prefixed, but required for the Agent Bot: the api builds its webhook URL as `${API_PUBLIC_URL}/api/v1/webhooks/chatwoot` (`main.rs:1239-1242`). Missing it → bot configured without a reachable webhook (`main.rs:1291-1293`). | Your public api hostname. | Edit `dc-config`, restart api. |

### 2b. The website widget token (operator-side, but lives in the *website* build, not the api)

| Key | What it's for | Where obtained |
|-----|---------------|----------------|
| `CHATWOOT_WEBSITE_TOKEN` → `VITE_CHATWOOT_WEBSITE_TOKEN` | The public, client-embeddable token identifying the support inbox in the website widget (`website/src/routes/+layout.svelte:7`). It is **not secret** (it is meant to be published in a browser bundle), but it must match a real Chatwoot "Website" inbox. | Chatwoot admin → Inboxes → (your website inbox) → Configuration → "Website Token". |

**How it reaches the prod bundle today:** the legacy `cf/deploy.py` website build
writes it into a gitignored `website/.env.local` as `VITE_CHATWOOT_WEBSITE_TOKEN=…`
(`cf/deploy.py:623-629`), and Vite inlines it at build time. See §8 for the
release-CI gap.

### 2c. Chatwoot's *own* internal keys (live in the Chatwoot deployment, NOT the api)

These belong to Chatwoot the application; the decent-cloud api never reads them.
They are documented in `cf/chatwoot.env.example` and `cf/CONFIG.md:172-173`.

| Key | What it's for | Notes |
|-----|---------------|-------|
| `CHATWOOT_SECRET_KEY_BASE` (`SECRET_KEY_BASE` in the pod) | Rails session/cookie signing key. | Generate with `openssl rand -hex 64` (`cf/chatwoot.env.example:5-6`). Rotating it invalidates all Chatwoot sessions. |
| `CHATWOOT_POSTGRES_PASSWORD` (`POSTGRES_PASSWORD` in the pod) | Password Chatwoot uses to connect to its **own** `chatwoot_prod` DB on the prod host. | Separate from the api's DB user. `cf/CONFIG.md:172`. |

---

## 3. Set up from scratch

Assumes the Chatwoot k8s manifests are already deployed and `support.decent-cloud.org`
returns 200 (the tunnel + `chatwoot_prod` DB exist). If not, start from
`docs/chatwoot-integration-plan.md` (the original provisioning plan) and the
`cf/chatwoot.env.example` template.

1. **One-time SuperAdmin + Platform App (manual, in the Chatwoot UI).**
   Browse to `https://support.decent-cloud.org` and:
   - Create the SuperAdmin account (Chatwoot first-run wizard).
   - Create the first **Account** (the support account; note its numeric ID —
     this becomes `CHATWOOT_ACCOUNT_ID`, usually `1`).
   - Create a **Platform Application**: SuperAdmin → *Applications / Platform Apps*
     → New. Copy its **Platform access token** → this is `CHATWOOT_PLATFORM_API_TOKEN`.
   - *(Confirm exact menu labels against the live Chatwoot v4.8 admin UI — the
     names above are the best-known navigation and Chatwoot has reorganized this
     surface across minor versions.)*

2. **Create the website inbox.**
   In the support Account: *Inboxes* → *New Inbox* → *Website Channel* → give it
   the support domain. Copy its **Website Token** → this is `CHATWOOT_WEBSITE_TOKEN`.

3. **Generate the Account API token.**
   Create a dedicated agent/admin user for the api, then from that user's
   *Profile* → *Access Token*, copy the token → this is `CHATWOOT_API_TOKEN`.

4. **Generate the shared HMAC secret.**
   `openssl rand -hex 32` → this is `CHATWOOT_HMAC_SECRET`. It must be set on the
   api **and** injected into the Chatwoot pod as `CHATWOOT_INBOX_HMAC_SECRET_KEY`
   (the prod manifest maps `CHATWOOT_HMAC_SECRET` → `CHATWOOT_INBOX_HMAC_SECRET_KEY`;
   see the dev equivalent at `cf/docker-compose.dev.yml:42`).

5. **Paste into the prod secret/config store.**
   Put the secrets into `dc-secret` (SOPS) and the non-sensitive values into
   `dc-config` (ConfigMap), per `cf/CONFIG.md:163-177` and the deploy-time rules
   in `repo/AGENTS.md` ("Deployment & secrets"). Apply via the k8s repo tooling.

6. **Restart the api.**
   On boot the api auto-creates/updates the Agent Bot named
   `"Decent Cloud Support Bot"` and assigns it to **every** inbox
   (`api/src/main.rs:1231-1303`; the bot-create/update logic is
   `api/src/chatwoot/client.rs:297-413`, the inbox assignment is
   `client.rs:943-978`). You do **not** create the agent bot by hand.

7. **Verify** with the doctor commands in §5.

---

## 4. Rotation procedures

### `CHATWOOT_API_TOKEN` (Account API)
1. In Chatwoot, open the api's dedicated user → Profile → reset/regenerate
   *Access Token*.
2. Update `CHATWOOT_API_TOKEN` in `dc-secret`.
3. Restart the api. The boot loop re-assigns the bot to all inboxes with the new
   token (`main.rs:1254-1274`).
4. Run `api-server doctor` (§5); a **401** in the "Checking Chatwoot API
   connectivity" line means the token is wrong/stale.

### `CHATWOOT_PLATFORM_API_TOKEN` (Platform API)
1. In Chatwoot SuperAdmin → regenerate the Platform App token.
2. Update `CHATWOOT_PLATFORM_API_TOKEN` in `dc-secret`.
3. Restart the api. Without a valid Platform token the api logs
   `"Chatwoot Platform API not configured - agent bot auto-setup disabled"`
   (`main.rs:1294-1298`) — the bot is never created/updated.

### `CHATWOOT_HMAC_SECRET` (shared symmetric secret — rotate BOTH sides together)
1. Generate a new secret: `openssl rand -hex 32`.
2. Update **both**, in either order but before either side is relied on:
   - `CHATWOOT_HMAC_SECRET` in the api `dc-secret`; **and**
   - `CHATWOOT_INBOX_HMAC_SECRET_KEY` in the Chatwoot deployment (same value).
3. Restart the api **and** roll the Chatwoot pods.
4. A mismatch shows up as logged-in users *not* being recognized in the widget
   (identity-hash verification silently fails).

### `CHATWOOT_WEBSITE_TOKEN` (widget)
1. In Chatwoot, *Inboxes* → (website inbox) → Configuration → copy/regenerate the
   Website Token.
2. Rebuild the website so the new token is inlined into the bundle (see §8).
3. No api restart needed.

> **Stage reuse warning.** Stage (`dc-stage`) points at the **same** prod
> Chatwoot instance, so stage must carry tokens that are valid against prod
> Chatwoot. A stage build carrying an old/rotated `CHATWOOT_API_TOKEN` returns
> **401** from Chatwoot — this is the known cause of stage's "Chatwoot 401".

---

## 5. Doctor / verify commands

```bash
# 1. Chatwoot UI is reachable from the public internet (cloudflared tunnel up)
curl -s -o /dev/null -w 'support.decent-cloud.org -> HTTP %{http_code}\n' \
  https://support.decent-cloud.org/

# 2. api-side health + Chatwoot config/connectivity (runs as a deploy gate)
#    Doctor prints a "Chatwoot Integration" block: lists each CHATWOOT_* key as
#    OK/MISSING, does a connectivity probe, and configures+assigns the agent bot.
#    Source: api/src/main.rs:845-937.
api-server doctor            # or, in k8s: kubectl -n dc-prod exec deploy/dc-api -- api-server doctor
```

Key boot/doctor log lines to look for (`api/src/main.rs`):
- `Chatwoot agent bot configured (id=…)` → success (`main.rs:1248-1252`).
- `Failed to configure Chatwoot agent bot: …` → bad Platform token / Chatwoot down
  (`main.rs:1288`).
- `Chatwoot client unavailable - cannot assign bot to inboxes: …` → bad Account
  token / URL (`main.rs:1276-1285`).
- Any **401** in `kubectl logs` when the api calls Chatwoot → the relevant token
  is invalid or was rotated without updating `dc-secret`.

```bash
# 3. api pod logs: confirm the boot auto-config, no 401s
kubectl -n dc-prod logs deploy/dc-api | grep -iE 'chatwoot|agent bot'
```

---

## 6. Known gaps (factual, code-grounded)

These are documented weaknesses, not runbook steps. Track/fix them separately.

1. **`POST /api/v1/webhooks/chatwoot` verifies no signature** — a latent
   security gap. The handler (`api/src/openapi/webhooks.rs:531-554`) parses the
   JSON body and acts on `message_created` events (triggering the support bot and
   DB writes) with **no** request-authentication step. Contrast this with the two
   sibling handlers in the same file, which *do* verify HMAC signatures:
   - Stripe: `stripe_webhook` requires `stripe-signature` and verifies it
     (`webhooks.rs:119-143`, helper `verify_signature` at `webhooks.rs:71-94`).
   - Telegram: `telegram_webhook` verifies its secret
     (`webhooks.rs:697`, tests `webhooks.rs:866-956`).
   Chatwoot does **not** send a per-webhook HMAC today, so closing this means
   either (a) restricting the endpoint to the in-cluster Chatwoot source IP / a
   network policy, or (b) adding a shared-secret header check. File an issue
   before changing it (this is a security-relevant change — see
   `repo/AGENTS.md` "ARCHITECTURAL ISSUES THAT REQUIRE A HUMAN DECISION").

---

## 7. Stage-specific notes

Stage (`dc-stage`) reuses prod Chatwoot. Consequences:
- All stage `CHATWOOT_*` tokens must be **the same valid values** as prod (a
  stage-only or stale token 401s). This is why a freshly-bumped stage that
  inherited old secrets shows "Chatwoot 401" in its logs.
- `CHATWOOT_HMAC_SECRET` must also match (it is the *prod* Chatwoot pod's
  `CHATWOOT_INBOX_HMAC_SECRET_KEY`).

---

## 8. Website support-widget build (finding + operator action)

**Live verdict (2026-08-03): the prod widget WORKS.** The compiled prod layout
bundle (`https://decent-cloud.org/_app/immutable/nodes/0.*.js`) ships the
Chatwoot widget code **and** an inlined website token — both
`websiteToken:"…"` and `baseUrl:"https://support.decent-cloud.org"` are present,
so the widget is loaded and configured.

**Origin of the token today.** It is **not** injected by the release workflow.
`.github/workflows/release.yml` injects only `VITE_CHATWOOT_BASE_URL` (and the API
URL + Telegram handle) into the website build (`release.yml:191-201`) but **omits
`VITE_CHATWOOT_WEBSITE_TOKEN`**. The token reaches the prod bundle via the
**legacy** `cf/deploy.py` website build, which writes
`VITE_CHATWOOT_WEBSITE_TOKEN` into a gitignored `website/.env.local`
(`cf/deploy.py:623-629`); Vite inlines it at build time. The legacy path is why
current prod works.

**CI wiring (now in place, operator action to populate).** `release.yml`'s "Build
website" env block now injects both Chatwoot vars, sourced from GitHub so the
prod bundle is never coupled to a hardcoded host:

```yaml
# .github/workflows/release.yml — "Build website" env block
VITE_CHATWOOT_BASE_URL: ${{ vars.CHATWOOT_BASE_URL }}
VITE_CHATWOOT_WEBSITE_TOKEN: ${{ secrets.CHATWOOT_WEBSITE_TOKEN }}
```

The widget renders **only when both are non-empty** (`ChatwootWidget.svelte`
gates on token AND base URL), so an unset pair yields a console-clean bundle with
the widget gated off (no dead-host fetch / 404 / X-Frame-Options error). To make
a release.yml-built website image actually show support chat, the operator must
set in `decent-stuff/decent-cloud`:
- repo **Variable** `CHATWOOT_BASE_URL` = the public Chatwoot URL (currently the
  live instance `https://dev-support.decent-cloud.org`; do **not** use the dead
  `support.decent-cloud.org` tunnel), and
- Actions **Secret** `CHATWOOT_WEBSITE_TOKEN` = the Chatwoot inbox website token
  (public/client-embeddable; safe to store as an Actions secret).

Until both are populated in GitHub, keep building the website via `cf/deploy.py`
so the widget keeps working.
