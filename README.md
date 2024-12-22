# Install dependencies

Install `cargo` by following https://rustup.rs/ -- it should be something along the lines of:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

To run end-to-end tests, we use [cargo-make](https://github.com/sagiegurari/cargo-make), which you can install by running

```bash
cargo install --force cargo-make
```

Python is only used to build the whitepaper. We use `pixi` as a dependency manager for Python.

Install `pixi` by following https://pixi.sh/latest/ -- the installation should be something along the lines of:

```bash
curl -fsSL https://pixi.sh/install.sh | bash
```

After that you can install all project dependencies with a simple `pixi install` in the project root.

# Current Status

The project is already usable, and it's in active development. Please open a GitHub issue if you notice problems.

At the moment the main development is on Linux (Ubuntu 24.04), where all testing is also done. MacOS should similarly work without problems.

Also, the `dc` binary should build for Windows without problems by build with [cross](https://github.com/cross-rs/cross): `cross build --release --target x86_64-pc-windows-gnu`.

Example of built release binaries:

```
-rwxrwxr-x 2 ubuntu ubuntu 13637456 Dez 20 22:26 target/release/dc
-rwxr-xr-x 2 ubuntu ubuntu 20445254 Dez 20 22:11 target/x86_64-pc-windows-gnu/release/dc.exe
```

Release binaries are still not published on GitHub -- please feel free to contribute by adding a GitHub workflow that publishes release binaries.

# Usage

## Key generation

### Option 1 (recommended): key generation with the `dc` cli tool:

```bash
cargo run --bin dc -- keygen --generate --identity <id-slug>
```

<details>
<summary>Example invocation and output:</summary>

```
cargo run --bin dc -- keygen --generate --identity my-provider
[...]
INFO - Mnemonic: <some words that you should save in a very safe place>
INFO - Generated identity: [Ed25519 signing] rb26m-cxrhj-t63qa-xg33g-hvla2-pr25n-nmc5y-rc2la-v4zuv-fjkec-wqe
```

</details>

<details>
<summary>Option 2 (alternative): key generation with openssl</summary>

For node provider:

```bash
mkdir -p $HOME/.dcc/identities/np
openssl genpkey -algorithm ED25519 -out $HOME/.dcc/identities/np/private.pem
```

For user:

```bash
mkdir -p $HOME/.dcc/identities/user
openssl genpkey -algorithm ED25519 -out $HOME/.dcc/identities/user/private.pem
```

</details>

## Registering an account in the DecentCloud Ledger Canister

You first need to get some DC tokens to pay for the registration fee. You can buy tokens for example on [icpswap](https://app.icpswap.com/swap?input=ryjl3-tyaaa-aaaaa-aaaba-cai&output=ggi4a-wyaaa-aaaai-actqq-cai).
The registration fee should be 0.5 tokens or less. You can get the latest registration fee on the [ICP Dashboard](https://dashboard.internetcomputer.org/canister/gplx4-aqaaa-aaaai-actra-cai)

After sending funds to the principal of your generated identity, you can register a node provider account with:

```
cargo run --bin dc -- np --register my-provider
[...]
INFO - Registering principal: my-provider as [Ed25519 signing] rb26m-cxrhj-t63qa-xg33g-hvla2-pr25n-nmc5y-rc2la-v4zuv-fjkec-wqe
```

Or a user account can be similarly registered with: `cargo run --bin dc -- user --register my-user`

## Participating in the periodic token distribution

The Decent Cloud platform utilizes a periodic token distribution mechanism, driven by a dual-token system designed to balance platform operations and incentivize participant behavior. Below is an overview of the periodic token distribution model:

1. **Token Minting**:

   - New tokens, called Decentralized Cloud Tokens (DCT), are minted approximately every 10 minutes with the creation of a new block.
   - The initial block generates 50 tokens, and the number of minted tokens reduces by half every 210,000 blocks, following a deflationary model similar to Bitcoin. This ensures a capped total supply of approximately 21 million DCT.

2. **Distribution Mechanism**:

   - Minted tokens are allocated fairly with every new block to all participants who paid the participation fee.
   - If there are no participants in a block, the reward is carried over to the next block.

3. **Eligibility and Fees**:

   - Participants must pay a registration fee equivalent to 1/100th of the block reward (0.5 DCT until the first halving) to be eligible for token rewards. These fees are directed to a transparent, DAO-controlled wallet for funding platform development.

4. **Incentives and Stability**:

   - The model promotes stability and aligns supply with demand. Developers use DCT to rent nodes, creating inherent demand for the token. Node providers may retain DCT in anticipation of price increases, further stabilizing the market, or sell the tokens to cover their expenses.
   - Upcoming governance through a Decentralized Autonomous Organization (DAO) ensures flexibility in addressing market volatility and adapting the reward system as necessary.

5. **Transparency and Compliance**:
   - All token operations are governed by smart contracts and adhere to relevant regulatory standards. This ensures secure and transparent transactions while fostering community trust.

The periodic distribution model reflects our commitment to equitable resource allocation, incentivizing long-term participation, and maintaining economic stability within the ecosystem.

For more technical details, refer to the whitepaper that can be found on the project[website](https://decent-cloud.org/).

Example of participation:

```
cargo run --bin dc -- np --check-in my-provider --check-in-memo "Oh yeah! I'm getting DCT!"
```

In the future, the memos will be shown on the project dashboard (help is welcome!).

Note that the above operation will synchronize the entire ledger to your local machine, ensuring the upstream ledger remains secure from malicious modifications. As the ledger is a cryptographically protected blockchain, maintaining multiple copies enhances protection against any tampering with its history.

You can also manually fetch the ledger by running `cargo run --bin dc -- ledger_remote fetch`

## Updating a Provider Profile

Node provider profile can be prepared locally, as a yaml file. Please check [the template in the repository](https://github.com/decent-stuff/decent-cloud/blob/main/examples/np-profile-template.yaml), and make changes locally. You can check yaml validity on [some online websites](https://www.yamllint.com/) if you have problems.

When ready, update the provider profile in the canister with:

```
cargo run --bin dc -- np --update-profile my-provider my-provider-profile.yaml
```

You need to pay a fee for this operation, to prevent DOS attacks.

## Updating Provider Offering

Similar to the profile, each provider can have an offering prepared locally, as a yaml file, and then published. Please refer to the [the template in the repository](https://github.com/decent-stuff/decent-cloud/blob/main/examples/np-offering-template.yaml).

When ready, update the provider offering with:

```
cargo run --bin dc -- np --update-profile my-provider my-provider-offering.yaml
```

You need to pay a fee for this operation, to prevent DOS attacks.

## User contracting an offering

Search for suitable offerings, preferably after running `ledger_remote fetch` to get the latest offerings:

```
cargo run --bin dc -- offering --query 'memory >= 512MB AND storage.size > 1gb'
```

This will give you the list of DC principals and their matching offerings. From the offerings, inspect the offerings and take the instance id from your preferred one.
You should also check the reputation and historical data for the provider (FIXME: add CLI and an example).

After finding the id:

```
cargo run --bin dc -- offering --contract-request <offering-id>
```

This gives you the contract-id that you can query later to check for the status.

To which the provider needs to reply with:

```
cargo run --bin dc -- offering --contract-reply <contract-id>
```

The provider can accept or reject the request, based on the user reputation, and the supplied payment amount.
Once the provider accepts the request, they allocate the machine and provide access details to the user.

Accepting the request increases the reputation of both the provider and the user, which helps future users pick reputable providers, or providers to pick and select users.

All the above should also be doable through the WebUI. Help is welcome!

FIXME: implement refunds and reducing the reputation of others.

# Developer notes

<details>
<summary>Running tests</summary>

You can run unit tests with:

```bash
cargo test
```

Or you can run the complete suite of unit tests and the canister tests using PocketIC, with [cargo-make](https://github.com/sagiegurari/cargo-make):

```bash
cargo make
```

</details>

<details>
<summary>Build whitepaper</summary>

There is a Python build script that uses a docker image with LaTeX and mermaid.js to build the whitepaper PDF.

You can invoke the build script with:

```bash
pixi run python3 ./docs/whitepaper/build.py
```

The result PDF document will be at `build/docs/whitepaper/whitepaper.pdf`.

</details>

<details>
<summary>Update CI image</summary>

There is a CI workflow that you can manually trigger on GitHub to refresh the CI build image: https://github.com/decent-stuff/decent-cloud/actions/workflows/build-container-image.yaml

If that fails, you can build the image locally and push it manually.

```
docker build .github/container/ --tag ghcr.io/decent-stuff/decent-cloud/ci-image:latest
docker push ghcr.io/decent-stuff/decent-cloud/ci-image:latest
```

If `docker push` fails with `denied: denied` or similar error, refresh the ghcr token at https://github.com/settings/tokens?page=1 and run

```
docker login ghcr.io
username: yanliu38
password: <generated token>
```

</details>
