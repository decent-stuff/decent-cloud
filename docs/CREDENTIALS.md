# Credentials

> **RULE (non-negotiable): NEVER ask the user for a credential before checking
> this document AND running `scripts/dc-secrets list`.** Credentials already
> persist — SOPS-sealed to **both** the agent `age` key and the operator `gpg`
> key (age key resolved from `$SOPS_AGE_KEY_FILE` / `$SOPS_AGE_KEY`, or the repo
> `.age-identity`; operator key resolved from the operator's gpg keyring), and
> a working subset is additionally injected as env vars each session. Re-asking
> wastes a round-trip and erodes trust. This file lists every credential **NAME**
> and where it lives; it contains **no values**.

This is the value-free manifest of every credential name used across decent-cloud,
groupled by area, with its source(s), purpose, and environment scope. It is the
single place to look to answer "does this credential already exist, and where?".

---

## One-command discovery

```bash
# Print every credential NAME available across env + SOPS (NEVER values),
# grouped by source. This is the first thing to run before asking for anything.
scripts/dc-secrets list

# Inject the SOPS credentials for the current env into your shell:
eval "$(scripts/dc-secrets export)"            # common-only (no env leakage)
eval "$(scripts/dc-secrets export play)"       # common + local-dev (play) layer
eval "$(scripts/dc-secrets export dev)"        # common + stage (dev) layer
eval "$(scripts/dc-secrets export prod)"       # common + prod layer
```

`dc-secrets list` (no args) is **aggregated discovery**: it walks the session
env (names that look credential-ish, or that also appear in a SOPS file) and
every SOPS `*.yaml` under `$DC_SECRETS_DIR`, and prints each name with its
source — `NAME<TAB>env` or `NAME<TAB>sops:<file>` — plus an `(empty)` marker for
set-but-empty values. Values are decrypted to memory only to test emptiness and
are **never** written to stdout. An unreadable SOPS file warns to stderr and is
skipped; it never aborts the listing.

`dc-secrets list <path>` (with a path) still lists the key **names** within one
SOPS file, e.g. `scripts/dc-secrets list shared/env`.

---

## The `dc-secrets` CLI

Location: `repo/scripts/dc-secrets` (uv inline script; run as
`scripts/dc-secrets <cmd>` or `uv run scripts/dc-secrets <cmd>`). SOPS-sealed
YAML with `flock` concurrency; values round-trip byte-identical through pyyaml.

| Command | What it does |
|---|---|
| `init` | Initialise the store: generate an age keypair + `.sops.yaml`, or **adopt** a key already provided via `$SOPS_AGE_KEY` / `$SOPS_AGE_KEY_FILE` / a host bind-mount. |
| `get <path> <key>` | Read **one** credential value to stdout. |
| `set <path> <key>=<value>...` | Write one or more credentials (flock-protected). |
| `delete <path> <key>` | Remove one credential. |
| `export [<env>] [--agent <n>]` | Print credentials as `KEY=value`, layered: `shared/common.yaml` + `shared/<env>.yaml` (+ `agents/<n>` + `hires/<n>` generic overlay). `<env>` ∈ `{common, play, dev, prod}`. Bare `export` is **common-only** (never leaks another env). Last file wins under shell `eval`. |
| `list [<path>]` | No args: **aggregated credential discovery** (names only — see above). With a path: key names within one file. |
| `import <env-file> <path>` | Import a `.env` file into encrypted storage. |
| `edit <path>` | Open a SOPS file in `$EDITOR` (decrypt, edit, re-encrypt). |
| `age-key export` | Print the resolved age identity (for backup/migration). |
| `age-key import [--from <f>]` | Seed the repo identity from a host key (fresh sandbox). |
| `help` | Show the built-in help. |

### To persist a NEW user-provided credential

```bash
scripts/dc-secrets set shared/env NEW_CREDENTIAL_NAME="$NEW_CREDENTIAL_NAME"
# then add NEW_CREDENTIAL_NAME + its source to the manifest below
```
Prefer the `set` subcommand (atomic, flock-protected) over hand-editing SOPS
files. **Never commit a plaintext value** — SOPS-encrypt everything. After
adding a key, add its name + source to the relevant table in this document so
the next agent discovers it.

---

## Store layout

| Layer | Path | Committed? | Scope |
|---|---|---|---|
| Shared (env-agnostic) | `secrets/shared/common.yaml` | **yes** (encrypted, keys visible) | all |
| Local-dev slim stack | `secrets/shared/play.yaml` | **yes** (encrypted) | local dev |
| Stage deploy (`dev.decent-cloud.org`) | `secrets/shared/dev.yaml` | **yes** (encrypted) | stage |
| Production | `secrets/shared/prod.yaml` | **yes** (encrypted) | prod |
| Per-agent overrides | `secrets/agents/<name>.yaml` | yes | overlay |
| Per-hire overlay (generic) | `secrets/hires/<name>.yaml` | yes | generic per-name overlay only |

**Committed SOPS files** (`common.yaml`, `dev.yaml`, `play.yaml`) live in
`repo/secrets/shared/` and are tracked in git — SOPS encrypts the values so the
files are safe to commit (keys visible, values encrypted).

**Local-only SOPS files** (`env.yaml`, `gh.yaml`) live under the operator's
`$DC_SECRETS_DIR` (outer `/project/decent-cloud/secrets/shared/`) and are
**never committed** — they hold the consolidated cross-env secrets (prod keys,
seeds, the full operator set) and release-CI tokens.

### Environment & key resolution

| Variable | Meaning |
|---|---|
| `DC_SECRETS_DIR` | Secrets directory (default: `<repo>/secrets`). The operator/sandbox typically points this at the outer `/project/decent-cloud/secrets`. |
| `SOPS_AGE_KEY` | Inline age key material (CI / secret-manager injection). Highest priority. |
| `SOPS_AGE_KEY_FILE` | Path to an age identity file (host bind-mount in a sandbox). |
| `secrets/.age-identity` | Repo-local bootstrap identity (gitignored; generated by `init`, portable via the two vars above). |

age-key resolution priority: `SOPS_AGE_KEY` → `SOPS_AGE_KEY_FILE` →
`secrets/.age-identity`. A fresh clone/sandbox has no `.age-identity`; it MUST
receive the canonical key via one of the two env vars (see
`agent/docs/secrets.md`). Without it the committed SOPS files cannot decrypt.

### Recipients (agent age key + operator gpg key)

**Every secret store encrypts to BOTH recipients**, so the agent and the operator
can each decrypt a file independently:

- **agent age key** `age1vdj457g4pyp7u5834sypdt3ys3gum939wwwggqz3jch8aes9lstsh5y9mr`
  — the agent decrypts via age (`sops -d <file>` with `$SOPS_AGE_KEY_FILE` /
  `$SOPS_AGE_KEY` set). No gpg keyring needed.
- **operator gpg key** fingerprint `FA5814CF1935EE80C454C9F1660DCCF069EC9176`
  (`Saša Tomić <sasa.gpg@kalaj.org>`) — the operator decrypts via their own gpg
  keyring: `sops -d <file>` with **no** age env var set (sops then falls back to
  gpg). No agent age key needed.

Both recipients are written into every `.sops.yaml` `creation_rule` by
`dc-secrets` (the `age:` and `pgp:` lines). To rotate or **add a NEW operator gpg
key**, set `DC_SOPS_PGP_RECIPIENT=<fingerprint>` (it overrides the default) and
re-wrap the existing data keys without touching values:

```bash
DC_SOPS_PGP_RECIPIENT=<new-fingerprint> scripts/dc-secrets init   # regenerate .sops.yaml
sops updatekeys --yes <file>        # per file, run from the store dir (sops finds .sops.yaml from CWD)
```

### k8s counterpart

Deploy-time secrets for `dc-prod` / `dc-stage` on k8s use a **separate** PGP-SOPS
store in the k8s repo, applied via `third_party/k8s/scripts/manage-secrets.py`
(not `dc-secrets`). See `cf/CONFIG.md` and `cf/DEPLOYMENT_CONFIG.md`.
`dc-secrets` is the developer/operator store; `manage-secrets.py` is the cluster
store.

---

## Manifest

**Source notation** (used in every table below):

- `env` — injected into the session environment each run.
- `common.yaml` / `dev.yaml` / `play.yaml` — committed encrypted SOPS file under `repo/secrets/shared/`.
- `env.yaml` / `gh.yaml` — local-only SOPS file under the operator `$DC_SECRETS_DIR`.
- `agents/<n>.yaml` — per-agent overlay.
- `hires/<n>.yaml` — generic per-name overlay (the dc-secrets tool still creates + overlays it).

**Scope**: `all` (env-agnostic / spans envs) · `dev` (local slim stack) · `stage`
(`dc-stage` / `dev.decent-cloud.org`) · `prod`.

### Cloud providers

> **Hetzner token rule for AI agents:** use `HETZNER_API_TOKEN_DEV` (read-write)
> for ALL development/experimentation (PoC scripts, probes, `api-cli e2e`, the
> provider-scraper, real-Hetzner verification). No other Hetzner token is
> injected into agent sessions — if a script can't find `_DEV`, it fails fast
> rather than probing for another token (a read-only Hetzner token exists in
> operator-local stores only; never use it — it 403s on create/delete and would
> strand a test VM). All four `HETZNER_API_TOKEN*` SOPS keys share ONE Hetzner
> project; `_DEV` is the read-write one. The agent-injection store
> (`repo/secrets/shared/common.yaml`) ships `_DEV` and no other Hetzner token, so
> agents get `_DEV` only. Prod provisioning does NOT use these as env vars — it
> provisions via per-`cloud_account` stored (encrypted) tokens. See `repo/AGENTS.md`
> "Hetzner tokens" for the full rule + injection note.

| Name | Source | Purpose | Scope |
|---|---|---|---|
| `HETZNER_API_TOKEN_DEV` | common.yaml (agent-injected); env.yaml | Hetzner API, **read-write, dev environment — the token AI agents use for ALL dev/experimentation** | dev |
| `HETZNER_API_TOKEN_STAGE` | env.yaml | Hetzner API, stage environment | stage |
| `HETZNER_API_TOKEN_PROD` | env.yaml | Hetzner API, production | prod |
| `PROXMOX_SSH` | env; common.yaml; env.yaml | Proxmox provider host SSH access | all |

### Payments (Stripe)

| Name | Source | Purpose | Scope |
|---|---|---|---|
| `STRIPE_PUBLISHABLE_KEY` | dev.yaml; env.yaml | Stripe publishable key (frontend) | stage |
| `STRIPE_SECRET_KEY` | dev.yaml; env.yaml | Stripe secret key (server-side) | stage |
| `STRIPE_WEBHOOK_SECRET` | dev.yaml; env.yaml | Stripe webhook signature verification | stage |
| `VITE_STRIPE_PUBLISHABLE_KEY` | dev.yaml; env.yaml | Stripe publishable key baked into the Vite build | stage |

### Auth / OAuth (Google)

| Name | Source | Purpose | Scope |
|---|---|---|---|
| `GOOGLE_OAUTH_CLIENT_ID` | dev.yaml; env.yaml | Google OAuth client identifier | stage |
| `GOOGLE_OAUTH_CLIENT_SECRET` | dev.yaml; env.yaml | Google OAuth client secret (real key name; sometimes informally called `GOOGLE_OAUTH_SECRET`) | stage |
| `GOOGLE_OAUTH_REDIRECT_URL` | dev.yaml; env.yaml | Google OAuth redirect URL | stage |

### Comms (email / SMS / chat)

| Name | Source | Purpose | Scope |
|---|---|---|---|
| `MAILCHANNELS_API_KEY` | env; common.yaml; env.yaml | MailChannels email delivery API | all |
| `SMTP_ADDRESS` | dev.yaml; env.yaml | Outbound SMTP host | stage |
| `SMTP_USERNAME` | dev.yaml; env.yaml | Outbound SMTP username | stage |
| `SMTP_PASSWORD` | dev.yaml; env.yaml | Outbound SMTP password | stage |
| `SMTP_PORT` | dev.yaml; env.yaml | Outbound SMTP port | stage |
| `DKIM_DOMAIN` | env; common.yaml; env.yaml | DKIM signing domain | all |
| `DKIM_SELECTOR` | env; common.yaml; env.yaml | DKIM selector | all |
| `DKIM_PRIVATE_KEY` | env; common.yaml; env.yaml | DKIM private key | all |
| `TEXTBEE_API_KEY` | env; common.yaml; env.yaml | TextBee SMS gateway API key | all |
| `TEXTBEE_DEVICE_ID` | env; common.yaml; env.yaml | TextBee linked device id | all |
| `TELEGRAM_BOT_TOKEN` | dev.yaml | Telegram bot token | stage |
| `TELEGRAM_BOT_USERNAME` | dev.yaml | Telegram bot username | stage |
| `CHATWOOT_BASE_URL` | dev.yaml; env.yaml | Chatwoot API base URL | stage |
| `CHATWOOT_WEBSITE_TOKEN` | dev.yaml; env.yaml | Chatwoot website widget token | stage |
| `CHATWOOT_API_TOKEN` | dev.yaml; env.yaml | Chatwoot API token | stage |
| `CHATWOOT_HMAC_SECRET` | dev.yaml; env.yaml | Chatwoot webhook HMAC signing secret | stage |
| `CHATWOOT_PLATFORM_API_TOKEN` | dev.yaml; env.yaml | Chatwoot platform API token | stage |
| `CHATWOOT_ACCOUNT_ID` | dev.yaml; env.yaml | Chatwoot account identifier | stage |
| `CHATWOOT_FRONTEND_URL` | dev.yaml; env.yaml | Chatwoot frontend URL | stage |
| `CHATWOOT_SECRET_KEY_BASE` | dev.yaml; env.yaml | Chatwoot Rails secret key base | stage |
| `CHATWOOT_POSTGRES_PASSWORD` | dev.yaml; env.yaml | Chatwoot Postgres password | stage |

### Infra / Cloudflare / tunnels

| Name | Source | Purpose | Scope |
|---|---|---|---|
| `CF_API_TOKEN` | env; common.yaml; env.yaml | Cloudflare API token (DNS management) | all |
| `CF_ZONE_ID` | env; common.yaml; env.yaml | Cloudflare zone identifier | all |
| `CF_DOMAIN` | env; common.yaml; env.yaml | Cloudflare base domain | all |
| `CF_ACCOUNT_ID` | env; common.yaml | Cloudflare account identifier | all |
| `CF_GW_PREFIX` | dev.yaml; env.yaml | Gateway DNS prefix (e.g. `dev-gw`) | stage |
| `TUNNEL_TOKEN` | dev.yaml | Cloudflare tunnel token | stage |

### Identity / DB / app

| Name | Source | Purpose | Scope |
|---|---|---|---|
| `DC_PROD_RESELLER_PUBKEY` | env.yaml | Operator `decent-cloud` ("Decent Cloud") Ed25519 pubkey (64 hex) | prod |
| `DC_PROD_RESELLER_SEED` | env.yaml | Operator `decent-cloud` ("Decent Cloud") BIP-39 seed — **MASTER key** (see `repo/AGENTS.md` "Acting as an existing provider identity") | prod |
| `CREDENTIAL_ENCRYPTION_KEY` | env; play.yaml; dev.yaml | API credential-at-rest encryption key | dev/stage |
| `DATABASE_URL` | play.yaml; env.yaml | Primary Postgres connection URL | dev |
| `TEST_DATABASE_URL` | play.yaml; env.yaml | Test Postgres connection URL | dev |
| `API_DATABASE_URL` | dev.yaml; env.yaml | API Postgres connection URL | stage |
| `PG_HOST` | play.yaml; env.yaml | Postgres host | dev |
| `PG_PORT` | play.yaml; env.yaml | Postgres port | dev |
| `PG_DB` | play.yaml; env.yaml | Postgres database name | dev |
| `PG_USER` | play.yaml; env.yaml | Postgres username | dev |
| `PG_PASSWORD` | env; play.yaml; env.yaml | Postgres password | dev |
| `PROD_POSTGRES_PASSWORD` | env.yaml | Production Postgres password | prod |
| `API_PUBLIC_URL` | play.yaml; dev.yaml; env.yaml | Public API base URL | dev/stage |
| `FRONTEND_URL` | play.yaml; dev.yaml; env.yaml | Frontend base URL | dev/stage |
| `API_SERVER_PORT` | play.yaml; env.yaml | API server bind port (local `59011`) | dev |
| `CANISTER_ID` | common.yaml; env.yaml | Internet Computer canister identifier | all |
| `IDENTITY_LOCAL_PATH` | env.yaml | Local identity keystore path | all |
| `DEFAULT_ESCALATION_USER` | common.yaml; env.yaml | Default escalation user | all |
| `INVOICE_SELLER_IBAN` | env.yaml | Invoice seller IBAN | all |
| `EMAIL_BATCH_SIZE` | common.yaml; env.yaml | Email processor batch size | all |
| `EMAIL_PROCESSOR_INTERVAL_SECS` | common.yaml; env.yaml | Email processor poll interval | all |

### GitHub

The legacy bare service tokens (`GITHUB_API_TOKEN`, `GITHUB_TEST_PAT`,
`GITHUB_PAT`) were removed as unused — they had zero CI/code consumers. CI
GitHub access uses the repo-only Actions secrets below.

**Repo-only (NOT in env or SOPS)** — these MUST be set in the GitHub repo
**Settings → Secrets and variables → Actions** for the `release.yml` workflow;
they cannot be read via `dc-secrets`:

`FORGEJO_OWNER`, `FORGEJO_USER`, `FORGEJO_TOKEN`, `NUC_K3S_REPO_WRITE`,
`DC_REPO_WRITE`.

### LLM / vision

| Name | Source | Purpose | Scope |
|---|---|---|---|
| `LLM_API_KEY` | env; common.yaml; env.yaml | LLM provider API key | all |
| `LLM_API_MODEL` | env; common.yaml; env.yaml | LLM model identifier | all |
| `LLM_API_URL` | env; common.yaml; env.yaml | LLM API base URL | all |
| `ANTHROPIC_API_KEY` | env.yaml | Anthropic API key | all |
| `ANTHROPIC_BASE_URL` | env.yaml | Anthropic API base URL | all |
| `ANTHROPIC_MODEL` | env.yaml | Anthropic model identifier | all |
| `ZAI_API_KEY` | env; common.yaml; env.yaml | ZAI vision MCP API key | all |

---

## Maintenance

- This manifest is hand-maintained. When you add a credential via `set`/`import`,
  add its **name + source + scope** to the relevant table above in the same
  commit. A credential not listed here is effectively invisible to future agents.
- To re-derive the list of names at any time, run `scripts/dc-secrets list` and
  reconcile against this document; drift means someone added a key without
  documenting it.
- Values are never recorded here. If a value appears in this file, that is a bug
  — remove it immediately and rotate the credential.
