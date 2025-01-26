use dcc_common::{account_balance_get_as_string, reputation_get, DccIdentity};
use ic_agent::identity::BasicIdentity;

mod list;
pub use list::{list_identities, list_local_identities, ListIdentityType};

pub fn println_identity(dcc_id: &DccIdentity, show_balance: bool) {
    if show_balance {
        println!(
            "{}, reputation {}, balance {}",
            dcc_id.display_as_ic_and_pem_one_line(),
            reputation_get(dcc_id.to_bytes_verifying()),
            account_balance_get_as_string(&dcc_id.as_icrc_compatible_account())
        );
    } else {
        println!(
            "{} reputation {}",
            dcc_id.display_as_ic_and_pem_one_line(),
            reputation_get(dcc_id.to_bytes_verifying())
        );
    }
}

pub fn dcc_to_ic_auth(dcc_identity: &DccIdentity) -> Option<BasicIdentity> {
    dcc_identity
        .signing_key_as_ic_agent_pem_string()
        .map(|pem_key| {
            let cursor = std::io::Cursor::new(pem_key.as_bytes());
            BasicIdentity::from_pem(cursor).expect("failed to parse pem key")
        })
}
