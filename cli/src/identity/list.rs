use dcc_common::{DccIdentity, LABEL_PROV_REGISTER, LABEL_USER_REGISTER};
use ledger_map::LedgerMap;

use super::println_identity;

#[derive(PartialEq)]
pub enum ListIdentityType {
    Providers,
    Users,
    All,
}

pub fn list_local_identities(include_balances: bool) -> Result<(), Box<dyn std::error::Error>> {
    let identities_dir = DccIdentity::identities_dir();
    println!("Available identities at {}:", identities_dir.display());
    let mut identities: Vec<_> = fs_err::read_dir(identities_dir)?
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(e) => {
                eprintln!("Failed to read identity: {}", e);
                None
            }
        })
        .collect();

    identities.sort_by_key(|identity| identity.file_name());

    for identity in identities {
        let path = identity.path();
        if path.is_dir() {
            let identity_name = identity.file_name();
            let identity_name = identity_name.to_string_lossy();
            match DccIdentity::load_from_dir(&path) {
                Ok(dcc_identity) => {
                    print!("{} => ", identity_name);
                    println_identity(&dcc_identity, include_balances);
                }
                Err(e) => {
                    println!("{} => Error: {}", identity_name, e);
                }
            }
        }
    }
    Ok(())
}

pub fn list_identities(
    ledger: &LedgerMap,
    identity_type: ListIdentityType,
    show_balances: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if identity_type == ListIdentityType::Providers || identity_type == ListIdentityType::All {
        println!("\n# Registered providers");
        for entry in ledger.iter(Some(LABEL_PROV_REGISTER)) {
            let dcc_id = DccIdentity::new_verifying_from_bytes(entry.key()).unwrap();
            println_identity(&dcc_id, show_balances);
        }
    }
    if identity_type == ListIdentityType::Users || identity_type == ListIdentityType::All {
        println!("\n# Registered users");
        for entry in ledger.iter(Some(LABEL_USER_REGISTER)) {
            let dcc_id = DccIdentity::new_verifying_from_bytes(entry.key()).unwrap();
            println_identity(&dcc_id, show_balances);
        }
    }
    Ok(())
}
