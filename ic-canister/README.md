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
