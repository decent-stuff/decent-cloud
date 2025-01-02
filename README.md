# Install dependencies

Install `cargo` by following [rustup](https://rustup.rs/) instructions. Typically:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

For end-to-end tests, we use [cargo-make](https://github.com/sagiegurari/cargo-make). Install it by running:

```bash
cargo install --force cargo-make
```

Python is used solely to build the whitepaper. We use `pixi` as a Python dependency manager.

Install `pixi` by following [pixi.sh](https://pixi.sh/latest/). Typically:

```bash
curl -fsSL https://pixi.sh/install.sh | bash
```

After installation, you can install all project dependencies with a simple `pixi install` in the project root.

# Current Status

The project is already usable and is under active development. If you encounter problems, please open a GitHub [issue](https://github.com/decent-stuff/decent-cloud/issues) or start a conversation in our [discussions](https://github.com/orgs/decent-stuff/discussions).

Currently, main development and testing happens on Linux (Ubuntu 24.04). MacOS should work without issues. The `dc` binary also builds for Windows via [cross](https://github.com/cross-rs/cross):

```bash
cross build --release --target x86_64-pc-windows-gnu
```

Example of built release binaries:

```
-rwxrwxr-x 2 ubuntu ubuntu 13637456 Dez 20 22:26 target/release/dc
-rwxr-xr-x 2 ubuntu ubuntu 20445254 Dez 20 22:11 target/x86_64-pc-windows-gnu/release/dc.exe
```

Release binaries are not yet published on GitHub. Contributions are welcome — for example, consider adding a GitHub workflow that publishes these release binaries.

# Usage

## Key generation

### Option 1 (recommended): key generation with the `dc` CLI tool:

```bash
cargo run --bin dc -- keygen --generate --identity <id-slug>
```

<details>
<summary>Example invocation and output:</summary>

```
cargo run --bin dc -- keygen --generate --identity my-provider
[...]
INFO  dc >  Generated mnemonic: <some words that you should save in a very safe place>
INFO  dc >  Generated identity: [Ed25519 signing] rb26m-cxrhj-t63qa-xg33g-hvla2-pr25n-nmc5y-rc2la-v4zuv-fjkec-wqe
```

</details>

Make sure you keep the mnemonic in a safe place, as it can be used to recreate the exact same identity.

<details>
<summary>Option 2 (alternative): key generation with openssl</summary>

```bash
mkdir -p $HOME/.dcc/identity/my-id
openssl genpkey -algorithm ED25519 -out $HOME/.dcc/identity/my-id/private.pem
```

</details>

Locally generated identities must be registered in the Ledger to interact with other users.

## Registering an account in the DecentCloud Ledger Canister

To prevent excessive account creation, a registration fee is required.
To get initial tokens, you can use [icpswap](https://app.icpswap.com/swap?input=ryjl3-tyaaa-aaaaa-aaaba-cai&output=ggi4a-wyaaa-aaaai-actqq-cai).
The registration fee is 0.5 DC tokens at the time of this writing and will decrease after each halving.
You can check the current fee on the [ICP Dashboard](https://dashboard.internetcomputer.org/canister/gplx4-aqaaa-aaaai-actra-cai) or via CLI:

```bash
❯ cargo run --bin dc -- ledger-remote get-registration-fee
[...]
Registration fee: 0.500000000
```

After obtaining DCT on the principal of your generated identity, you can register a node provider account:

```bash
cargo run --bin dc -- np register --identity my-provider
[...]
INFO - Registering principal: my-provider as [Ed25519 signing] rb26m-cxrhj-t63qa-xg33g-hvla2-pr25n-nmc5y-rc2la-v4zuv-fjkec-wqe
```

Or register a user account similarly:

```bash
cargo run --bin dc -- user register --identity my-user
```

## Participating in the periodic token distribution

The Decent Cloud platform uses a periodic token distribution mechanism to balance operations and incentivize participant behavior.

1. **Token Minting**

   - New tokens, called Decentralized Cloud Tokens (DCT), are minted approximately every 10 minutes with the creation of a new block.
   - The initial block generates 50 tokens, and the number of minted tokens is halved every 210,000 blocks (similar to Bitcoin), ensuring a capped total supply of about 21 million DCT.

2. **Distribution Mechanism**

   - Minted tokens are allocated with each new block to participants who paid the participation fee.
   - If there are no participants in a block, the reward carries over to the next block.

3. **Eligibility and Fees**

   - Participants must pay a registration fee equal to 1/100th of the block reward (0.5 DCT until the first halving) to be eligible for token rewards. Fees are directed to a DAO-controlled wallet, funding platform development.

4. **Incentives and Stability**

   - The model promotes stability by aligning supply with demand. Developers use DCT to rent nodes, creating a built-in demand. Node providers may hold onto DCT anticipating price increases or sell tokens to cover operational costs.
   - A Decentralized Autonomous Organization (DAO) will govern the system, allowing flexibility to address market volatility and adjust the reward system as needed.

5. **Transparency and Compliance**
   - All token operations are governed by smart contracts and comply with relevant regulations. This ensures secure, transparent transactions that build community trust.

This periodic distribution model underscores our commitment to fair resource allocation, long-term participation, and economic stability within the ecosystem. For technical details, please see the whitepaper at our [website](https://decent-cloud.org/).

Any provider can participate. For example:

```bash
cargo run --bin dc -- np check-in --identity my-id --memo "Oh yeah! I'm getting free DCT!"
```

In the future, these memos will appear on the project dashboard, so be creative — contributions are always welcome!

Note that there is no fundamental difference between a user and a provider; the same identity can be both.

Another note: the check-in operation first synchronizes the entire ledger to your local machine, keeping the upstream ledger secure from modifications. The ledger is cryptographically protected, and having multiple copies adds security against tampering, similar to `git`.

You can also manually fetch the ledger by running:

```bash
cargo run --bin dc -- ledger-remote fetch
```

## Updating a Provider Profile

Prepare your node provider profile locally as a YAML file. Check out [the template in the repository](https://github.com/decent-stuff/decent-cloud/blob/main/examples/np-profile-template.yaml), then edit it to your needs. If you run into issues, you can validate YAML on [some online tools](https://www.yamllint.com/).

When ready, update your provider profile in the canister:

```bash
cargo run --bin dc -- np update-profile --identity my-provider --profile-file my-provider-profile.yaml
```

A small fee is required for this operation to prevent DoS attacks.

## Updating Provider Offering

Similar to the profile, you can prepare your offering locally as a YAML file, then publish it. Refer to [the template in the repository](https://github.com/decent-stuff/decent-cloud/blob/main/examples/np-offering-template.yaml).

When ready, update the provider offering:

```bash
cargo run --bin dc -- np update-offering --identity my-provider --offering-file my-provider-offering.yaml
```

Again, a small fee is required for this operation, preventing DoS attacks.

## User contracting an offering

Run `ledger_remote fetch` to get the latest offerings, then search for suitable ones:

```bash
cargo run --bin dc -- offering query 'memory >= 512MB AND storage.size > 1gb'
```

Or list all offerings:

```bash
cargo run --bin dc -- offering list
```

You will see the DC principals and their associated offerings. Inspect them, grab the instance ID, and review the provider’s reputation and historical data (e.g., `cargo run --bin dc -- np list --balances`).

After you find the right offering, you can run something like:

```bash
cargo run --bin dc -- contract sign-request --offering-id xxx-small --identity my-user --requester-ssh-pubkey "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDbPVy5BWvjp6bm1wanPbH+hkPuOrx4AjUoczADfYpcx test-ssh-user" --requester-contact "https://github.com/orgs/decent-stuff/discussions/5" --memo "Oh yeah I'm signing a contract!" --provider-pubkey-pem MCowBQYDK2VwAyEAbxvOReOGb95hG/zXWheKtofsAP86+Q/bfVsPsgscQBE= --interactive
```

If you provide `--interactive`, the CLI will prompt for missing arguments instead of failing with an error.

After confirmation, it sends a contract sign request. Both you and the provider can periodically check for open contract requests:

```bash
cargo run --bin dc -- contract list-open
```

To complete the contract signing, the provider must confirm (or reject) the contract:

```bash
cargo run --bin dc --  contract sign-reply --identity my-provider --contract-id <contract-id-base64> --sign-accept true --response-text "It works!" --interactive
```

The provider may accept or reject the request based on user reputation or other factors.

- If rejected, the user is refunded minus the transaction fee, and the rejection is recorded in the ledger.
- If accepted, the user pays the full amount plus a transaction fee, and the provider allocates resources and supplies access details. Both parties’ reputations increase, helping other users and providers make informed choices.

All of this will also be accessible via a WebUI in the future; contributions are welcome!
_(FIXME: implement refunds in case of user dissatisfaction and accordingly adjust reputation for such providers.)_

# Developer notes

<details>
<summary>Running tests</summary>

You can run unit tests with:

```bash
cargo test
```

Or run the complete suite of unit tests and the canister tests using PocketIC, with [cargo-make](https://github.com/sagiegurari/cargo-make):

```bash
cargo make
```

</details>

<details>
<summary>Build whitepaper</summary>

A Python build script uses a Docker image with LaTeX and mermaid.js to build the whitepaper PDF:

```bash
pixi run python3 ./docs/whitepaper/build.py
```

The resulting PDF will be at `build/docs/whitepaper/whitepaper.pdf`.

</details>

<details>
<summary>Update CI image</summary>

A GitHub workflow can be manually triggered to refresh the CI build image:
[CI build image](https://github.com/decent-stuff/decent-cloud/actions/workflows/build-container-image.yaml)

If it fails, you can build the image locally and push it:

```bash
docker build .github/container/ --tag ghcr.io/decent-stuff/decent-cloud/ci-image:latest
docker push ghcr.io/decent-stuff/decent-cloud/ci-image:latest
```

If `docker push` fails with `denied: denied`, refresh your ghcr token at [GitHub settings/tokens](https://github.com/settings/tokens?page=1) and run:

```bash
docker login ghcr.io
username: <your-username>
password: <generated token>
```

</details>
