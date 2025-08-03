use crate::argparse::OfferingCommands;
use dcc_common::{offerings::do_get_matching_offerings, DccIdentity};
use ledger_map::LedgerMap;

pub async fn handle_offering_command(
    cmd: OfferingCommands,
    ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = match cmd {
        OfferingCommands::List => "",
        OfferingCommands::Query(query_args) => &query_args.query.clone(),
    };

    let offerings = do_get_matching_offerings(&ledger_local, query);
    println!("Found {} matching offerings:", offerings.len());
    for provider_offering in offerings {
        let dcc_id = DccIdentity::new_verifying_from_bytes(&provider_offering.provider_pubkey)
            .unwrap_or_else(|_| DccIdentity::new_verifying_from_bytes(&[0; 32]).unwrap());

        for offering in &provider_offering.server_offerings {
            println!(
                "{} ==>\n{}",
                dcc_id.display_as_ic_and_pem_one_line(),
                offering
            );
        }
    }

    Ok(())
}
