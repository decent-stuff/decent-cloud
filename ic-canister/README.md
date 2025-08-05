# IC Canister Deployment Guide

This document provides clear instructions for developing and deploying the IC canister both locally and on the Internet Computer mainnet.

## Local Development

To develop locally:

1. **Start the Replica:**  
   Begin by launching the local replica in the background.
   ```bash
   dfx start --background
   ```
2. **Deploy Canisters:**  
   Deploy your canisters and generate the candid interfaces.
   ```bash
   dfx deploy
   ```
3. **Reset Local Environment:**  
   For a fresh start, reset the state:
   ```bash
   dfx start --background --clean
   dfx deploy
   ```

## Mainnet Deployment

For production deployments on the Internet Computer mainnet:

1. **Set Up Mainnet Identity:**  
   Create and switch to a mainnet identity.
   ```bash
   dfx identity new mainnet-eu
   dfx identity use mainnet-eu
   dfx identity get-principal
   ```
2. **Prepare for Deployment:**  
   Check available subnet types and create a canister with an initial funding amount.
   ```bash
   dfx ledger --network ic show-subnet-types
   dfx ledger --network ic create-canister --amount 0.5 --subnet-type european <your-canister-id>
   ```
3. **Deploy Wallet and Canisters:**  
   Deploy your wallet and then the canisters.
   ```bash
   dfx identity --network ic deploy-wallet <your-wallet-id>
   dfx deploy --ic
   ```
4. **Interact with Your Canister:**  
   Optionally, call a method to verify deployment.
   ```bash
   dfx canister --ic call <canister-id> get_logs_info
   ```

## Local Instance Testing

After deployment, you can test your local instance:
```bash
dfx deploy --identity default
curl http://<local-canister-id>.localhost:8000/metrics
curl http://<local-canister-id>.localhost:8000/logs
```
Access the Candid UI at:  
[http://127.0.0.1:8000/?canisterId=<local-canister-id>](http://127.0.0.1:8000/?canisterId=<local-canister-id>)

## Mainnet Instance Access

To interact with a mainnet deployed canister:
```bash
dfx deploy --ic --identity mainnet-eu
curl https://<canister-id>.raw.icp0.io/metrics
curl https://<canister-id>.raw.icp0.io/logs
```
Access the mainnet Candid UI via:  
[https://<your-canister-id>.raw.ic0.app/](https://<your-canister-id>.raw.ic0.app/)

## Advanced: Creating a Canister with an Alternate Identity

1. **Retrieve Principal ID:**  
   For a different identity:
   ```bash
   dfx identity get-principal --identity mainnet-01
   ```
2. **Check Ledger Balance:**  
   Verify your account balance.
   ```bash
   dfx ledger --network ic --identity mainnet-01 balance
   ```
3. **Create a New Identity:**  
   Create a new identity if needed.
   ```bash
   dfx identity new mainnet-01
   ```
4. **Create and Fund a Canister:**  
   Use the ledger to create a new canister.
   ```bash
   dfx ledger --network ic --identity mainnet-01 create-canister --amount 1 <wallet-address>
   ```
   Note: The command output confirms the new canister ID.
5. **Configure Wallet:**  
   Set the wallet for this identity.
   ```bash
   dfx identity --network ic --identity mainnet-01 set-wallet <wallet-id>
   ```
6. **Deploy the Canister:**  
   Create and deploy the canister.
   ```bash
   dfx canister --network ic --identity mainnet-01 create decent_cloud
   dfx deploy --network ic --identity mainnet-01 decent_cloud
   ```

## Provider Offerings and Network Services

For comprehensive information about network provider offerings, such as VPS, dedicated servers, and cloud instances, please refer to the [provider-offering README](../provider-offering/README.md).

This document contains detailed information about:
- CSV format specifications for offerings
- Provider identity management and offerings

---

For more details, refer to the [Documentation Home](../docs/README.md).

**Dashboard Links (replace placeholders with actual IDs):**  
- **Wallet:** [Dashboard](https://dashboard.internetcomputer.org/canister/<wallet-id>)  
- **Canister Code:** [Dashboard](https://dashboard.internetcomputer.org/canister/<canister-id>)  
- **Subnet:** [Dashboard](https://dashboard.internetcomputer.org/subnet/<subnet-id>)