# Target configs

Each file here is a base config for one environment (`dev` / `stage` / `prod`).
Load with `--target <name>` (e.g. `node run.js --target stage`).

Env vars (`DC_E2E_*`) **override** the file values, so commit public URLs here
and inject secrets via env / your secret manager — never commit a real key.

```jsonc
{
  "target": "stage",
  "webUrl": "https://stage.decent-cloud.org",   // website origin (sign-up flow)
  "apiUrl": "https://stage-api.decent-cloud.org", // API origin (health, signed calls)
  "hetznerToken": "PLACEHOLDER",                  // real Hetzner API token (env: DC_E2E_HETZNER_TOKEN)
  "accountEmailPrefix": "PLACEHOLDER",            // sign-up email prefix (env: DC_E2E_ACCOUNT_EMAIL_PREFIX)
  "includeProvision": false,                      // enable rent-provision-cancel (SPENDS MONEY)
  "expectedEnvironment": "stage"                  // optional: /health environment assertion
}
```

`PLACEHOLDER` is treated as **missing** by validation — running a flow that
needs a placeholder field fails loud (exit 2) with the named env var. This
prevents accidentally running against a target with an unset secret.
