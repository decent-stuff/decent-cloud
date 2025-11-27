use crate::argparse::OfferingCommands;
use ledger_map::LedgerMap;

pub async fn handle_offering_command(
    cmd: OfferingCommands,
    _ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = match cmd {
        OfferingCommands::List => "",
        OfferingCommands::Query(query_args) => &query_args.query.clone(),
    };
    todo!("Query offerings in the API server, with query: {}", query);
}
