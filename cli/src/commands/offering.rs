use crate::argparse::OfferingCommands;
use dcc_common::offerings::do_get_matching_offerings;
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
    for (dcc_id, offering) in offerings {
        println!(
            "{} ==>\n{}",
            dcc_id.display_as_ic_and_pem_one_line(),
            &offering.as_json_string_pretty().unwrap_or_default()
        );
    }

    Ok(())
}
