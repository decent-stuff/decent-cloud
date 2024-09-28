# ic-canister

## Running the project locally

For local development:

```bash
# Starts the replica, running in the background
dfx start --background

# Deploys your canisters to the replica and generates your candid interface
dfx deploy
```

To start local development from scratch, resetting the state

```bash
# Starts the replica, running in the background
dfx start --background --clean

# Deploys your canisters to the replica and generates your candid interface
dfx deploy
```

Deploying on the IC Mainnet

```bash
dfx identity new mainnet-eu
dfx identity use mainnet-eu
dfx identity get-principal
dfx ledger --network ic show-subnet-types
dfx ledger --network ic create-canister --amount 0.5 --subnet-type european 3cghu-vatzl-bylaa-ere6r-yudpn-5zaah-uwbdp-zie7s-wcmkh-3v267-yqe
dfx identity --network ic deploy-wallet  tmuuj-diaaa-aaaas-aaaba-cai
dfx deploy --ic
dfx canister --ic call tlvs5-oqaaa-aaaas-aaabq-cai get_logs_info
```

# Interacting with the canister

## Local canister instance

```bash
dfx deploy --identity default
curl http://bkyz2-fmaaa-aaaaa-qaaaq-cai.localhost:8000/metrics
curl http://bkyz2-fmaaa-aaaaa-qaaaq-cai.localhost:8000/logs
```

Candid UI: http://127.0.0.1:8000/?canisterId=bd3sg-teaaa-aaaaa-qaaba-cai&id=bkyz2-fmaaa-aaaaa-qaaaq-cai

## Mainnet canister instance

```bash
dfx deploy --ic --identity mainnet-eu
curl https://tlvs5-oqaaa-aaaas-aaabq-cai.raw.icp0.io/metrics
curl https://tlvs5-oqaaa-aaaas-aaabq-cai.raw.icp0.io/logs
```

Candid UI: https://a4gq6-oaaaa-aaaab-qaa4q-cai.raw.ic0.app/?id=tlvs5-oqaaa-aaaas-aaabq-cai

# Creating a canister with other identity, on a regular subnet

```bash
dfx identity get-principal --identity mainnet-01
dfx ledger --network ic --identity mainnet-01 balance
dfx identity new mainnet-01
```

```bash
❯ dfx ledger --network ic --identity mainnet-01 create-canister --amount 1 74cze-reen5-6nkrr-m5f7s-pflwz-ej5mq-ptcfh-obfra-jdeqq-kb5u5-kae
[...]
Canister created with id: "gbj2u-3aaaa-aaaai-actqa-cai"
```

Configure dfx to use the new wallet for the given identity

```bash
dfx identity --network mainnet-01 --identity mainnet-01 set-wallet gbj2u-3aaaa-aaaai-actqa-cai
```

Now create the canister with the wallet created above:

```bash
❯ dfx canister --network ic --identity mainnet-01 create decent_cloud
Creating canister decent_cloud...
decent_cloud canister created on network ic with canister id: ggi4a-wyaaa-aaaai-actqq-cai
```

(also on the same subnet)

Finally, deploy the canister wasm:

```bash
dfx deploy --network ic --identity mainnet-01 decent_cloud
```

WALLET: https://dashboard.internetcomputer.org/canister/gbj2u-3aaaa-aaaai-actqa-cai
CODE 01: https://dashboard.internetcomputer.org/canister/ggi4a-wyaaa-aaaai-actqq-cai
CODE 02: https://dashboard.internetcomputer.org/canister/gplx4-aqaaa-aaaai-actra-cai
SUBNET: https://dashboard.internetcomputer.org/subnet/brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae
