use crate::argparse::ContractCommands;
use ledger_map::LedgerMap;

pub async fn handle_contract_command(
    contract_args: ContractCommands,
    _network_url: &str,
    _ledger_canister_id: candid::Principal,
    _identity: Option<String>,
    _ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    match contract_args {
        ContractCommands::ListOpen(_list_open_args) => {
            todo!("Listing all open contracts...");
        }
        ContractCommands::SignRequest(_sign_req_args) => {
            todo!("Get the offering from the decent-cloud api server, and sign it with the local identity");
        }
        ContractCommands::SignReply(_sign_reply_args) => {
            todo!("Reply to a contract-sign request...");
        }
    }
}
