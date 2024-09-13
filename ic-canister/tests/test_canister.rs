use crate::canister_backend::icrc1::Icrc1StandardRecord;
use candid::{encode_one, Encode, Nat, Principal};
use dcc_common::{
    np_registration_fee_e9s, reward_e9s_per_block_recalculate, DccIdentity, BLOCK_INTERVAL_SECS,
    FIRST_BLOCK_TIMESTAMP_NS, MINTING_ACCOUNT_ICRC1,
};
use decent_cloud_canister::*;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use icrc_ledger_types::icrc1::account::Account as Icrc1Account;
use icrc_ledger_types::icrc1::transfer::{Memo as Icrc1Memo, TransferArg, TransferError};
use once_cell::sync::Lazy;
use pocket_ic::PocketIc;
use pocket_ic::WasmResult;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

static CANISTER_WASM: Lazy<Vec<u8>> = Lazy::new(|| {
    let mut path = workspace_dir();
    Command::new("dfx")
        .arg("build")
        .current_dir(&path.join("ic-canister"))
        .output()
        .unwrap();
    path.push("target/wasm32-unknown-unknown/release/decent_cloud_canister.wasm");
    std::fs::read(path).unwrap()
});

macro_rules! query_check_and_decode {
    ($pic:expr, $can:expr, $method_name:expr, $method_arg:expr, $decode_type:ty) => {{
        let reply = $pic
            .query_call(
                $can,
                Principal::anonymous(),
                $method_name,
                $method_arg.clone(),
            )
            .expect("Failed to run query call on the canister");
        let reply = match reply {
            WasmResult::Reply(reply) => reply,
            WasmResult::Reject(_) => panic!("Received a reject"),
        };

        candid::decode_one::<$decode_type>(&reply).expect("Failed to decode")
    }};
}

macro_rules! update_check_and_decode {
    ($pic:expr, $can:expr, $sender:expr, $method_name:expr, $method_arg:expr, $decode_type:ty) => {{
        let reply = $pic
            .update_call($can, $sender, $method_name, $method_arg.clone())
            .expect("Failed to run update call on the canister");
        let reply = match reply {
            WasmResult::Reply(reply) => reply,
            WasmResult::Reject(_) => panic!("Received a reject"),
        };

        candid::decode_one::<$decode_type>(&reply).expect("Failed to decode")
    }};
}

fn create_test_canister() -> (PocketIc, Principal) {
    let pic = PocketIc::new();
    let canister_id = pic.create_canister();
    pic.add_cycles(canister_id, 20_000_000_000_000);

    pic.install_canister(
        canister_id,
        CANISTER_WASM.clone(),
        encode_one(true).expect("failed to encode"),
        None,
    );

    // Ensure deterministic timestamp
    let ts_ns = FIRST_BLOCK_TIMESTAMP_NS + 100 * BLOCK_INTERVAL_SECS * 1_000_000_000;
    let ts_1 = encode_one(ts_ns).unwrap();
    update_check_and_decode!(
        pic,
        canister_id,
        Principal::anonymous(),
        "set_timestamp_ns",
        ts_1,
        ()
    );
    commit(&pic, canister_id);

    (pic, canister_id)
}

fn upgrade_test_canister(pic: &PocketIc, can: Principal) -> Result<(), pocket_ic::CallError> {
    let no_args = encode_one(true).expect("failed to encode");
    pic.upgrade_canister(can, CANISTER_WASM.clone(), no_args, None)
}

fn get_account_balance(pic: &PocketIc, can: Principal, account: &Icrc1Account) -> Nat {
    query_check_and_decode!(
        pic,
        can,
        "icrc1_balance_of",
        encode_one(account).expect("failed to encode"),
        Nat
    )
}

fn get_timestamp_ns(pic: &PocketIc, can: Principal) -> u64 {
    query_check_and_decode!(pic, can, "get_timestamp_ns", encode_one(()).unwrap(), u64)
}

#[test]
fn test_get_set_timestamp() {
    let (pic, can_id) = create_test_canister();
    let no_args = encode_one(()).expect("failed to encode");
    let timestamp = query_check_and_decode!(pic, can_id, "get_timestamp_ns", no_args.clone(), u64);
    println!("canister timestamp: {:?}", timestamp);

    assert!(timestamp > 1600000000000000000u64);

    let ts_1 = encode_one(2000000000000000000u64).unwrap();
    update_check_and_decode!(
        pic,
        can_id,
        Principal::anonymous(),
        "set_timestamp_ns",
        ts_1,
        ()
    );

    assert_eq!(get_timestamp_ns(&pic, can_id), 2000000000000000000u64);
}

fn test_ffwd_to_next_block(mut ts_ns: u64, p: &PocketIc, c: Principal) -> u64 {
    ts_ns += BLOCK_INTERVAL_SECS * 1_000_000_000;
    let ts_2 = encode_one(ts_ns).unwrap();
    update_check_and_decode!(p, c, Principal::anonymous(), "set_timestamp_ns", ts_2, ());
    commit(p, c);
    ts_ns
}

#[test]
fn test_icrc1_compatibility() {
    // From https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-1/ICRC-1.did#L41-L52
    let (pic, can_id) = create_test_canister();

    let no_args = encode_one(()).expect("failed to encode");
    assert_eq!(
        query_check_and_decode!(
            pic,
            can_id,
            "icrc1_metadata",
            no_args.clone(),
            Vec<(String, MetadataValue)>
        ),
        vec![
            MetadataValue::entry("icrc1:decimals", DC_TOKEN_DECIMALS as u64),
            MetadataValue::entry("icrc1:name", DC_TOKEN_NAME.to_string()),
            MetadataValue::entry("icrc1:symbol", DC_TOKEN_SYMBOL.to_string()),
            MetadataValue::entry("icrc1:fee", DC_TOKEN_TRANSFER_FEE_E9S),
        ]
    );
    assert_eq!(
        query_check_and_decode!(pic, can_id, "icrc1_name", no_args.clone(), String),
        DC_TOKEN_NAME.to_string()
    );
    assert_eq!(
        query_check_and_decode!(pic, can_id, "icrc1_symbol", no_args.clone(), String),
        DC_TOKEN_SYMBOL.to_string()
    );
    assert_eq!(
        query_check_and_decode!(pic, can_id, "icrc1_decimals", no_args.clone(), u8),
        DC_TOKEN_DECIMALS
    );
    assert_eq!(
        query_check_and_decode!(pic, can_id, "icrc1_fee", no_args.clone(), Nat),
        DC_TOKEN_TRANSFER_FEE_E9S
    );
    assert_eq!(
        query_check_and_decode!(pic, can_id, "icrc1_total_supply", no_args.clone(), Nat),
        DC_TOKEN_TOTAL_SUPPLY
    );
    assert_eq!(
        query_check_and_decode!(
            pic,
            can_id,
            "icrc1_minting_account",
            no_args.clone(),
            Option<Icrc1Account>
        ),
        Some(MINTING_ACCOUNT_ICRC1)
    );
    assert_eq!(
        query_check_and_decode!(
            pic,
            can_id,
            "icrc1_supported_standards",
            no_args.clone(),
            Vec<Icrc1StandardRecord>
        ),
        vec![Icrc1StandardRecord {
            name: "ICRC-1".to_string(),
            url: "https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-1".to_string(),
        }]
    );
    // The following two methods are tested in test_balances_and_transfers()
    // icrc1_balance_of : (Account) -> (nat) query;
    // icrc1_transfer : (TransferArgs) -> (variant { Ok : nat; Err : TransferError });
}

fn mint_tokens_for_test(
    pic: &PocketIc,
    can_id: Principal,
    acct: &Icrc1Account,
    amount: u64,
) -> Nat {
    update_check_and_decode!(
        pic,
        can_id,
        acct.owner,
        "mint_tokens_for_test",
        candid::encode_args((acct, Nat::from(amount), None::<Option<Icrc1Memo>>)).unwrap(),
        Nat
    )
}

fn transfer_funds(
    pic: &PocketIc,
    can: Principal,
    send_from: &Icrc1Account,
    send_to: &Icrc1Account,
    amount_send: u64,
) -> Result<candid::Nat, TransferError> {
    // Transfer amount_send tokens from one account to another
    let transfer_args = TransferArg {
        from_subaccount: send_from.subaccount,
        to: *send_to,
        fee: Some(DC_TOKEN_TRANSFER_FEE_E9S.into()),
        created_at_time: None,
        memo: None,
        amount: Nat::from(amount_send),
    };
    update_check_and_decode!(
        pic,
        can,
        send_from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_args).unwrap(),
        Result<candid::Nat, TransferError>
    )
}

fn np_register(
    pic: &PocketIc,
    can: Principal,
    seed: &[u8],
    initial_funds: u64,
) -> (DccIdentity, Result<String, String>) {
    let dcc_identity = DccIdentity::new_from_seed(seed).unwrap();
    if initial_funds > 0 {
        mint_tokens_for_test(
            pic,
            can,
            &dcc_identity.as_icrc_compatible_account().into(),
            initial_funds,
        );
    }
    let result = update_check_and_decode!(
        pic,
        can,
        dcc_identity.to_ic_principal(),
        "node_provider_register",
        Encode!(&dcc_identity.to_bytes_verifying(), &Vec::<u8>::new()).unwrap(),
        Result<String, String>
    );
    (dcc_identity, result)
}

fn user_register(
    pic: &PocketIc,
    can: Principal,
    seed: &[u8],
    initial_funds: u64,
) -> (DccIdentity, Result<String, String>) {
    let dcc_identity = DccIdentity::new_from_seed(seed).unwrap();
    if initial_funds > 0 {
        mint_tokens_for_test(
            pic,
            can,
            &dcc_identity.as_icrc_compatible_account().into(),
            initial_funds,
        );
    }
    let result = update_check_and_decode!(
        pic,
        can,
        dcc_identity.to_ic_principal(),
        "user_register",
        Encode!(&dcc_identity.to_bytes_verifying(), &Vec::<u8>::new()).unwrap(),
        Result<String, String>
    );
    (dcc_identity, result)
}

fn identity_reputation_get(pic: &PocketIc, can: Principal, identity: &Vec<u8>) -> u64 {
    let args = Encode!(&identity).unwrap();
    query_check_and_decode!(pic, can, "get_identity_reputation", args, u64)
}

fn np_check_in(
    pic: &PocketIc,
    can: Principal,
    dcc_identity: &DccIdentity,
) -> Result<String, String> {
    let no_args = encode_one(()).expect("failed to encode");
    let nonce_bytes = query_check_and_decode!(pic, can, "get_np_check_in_nonce", no_args, Vec<u8>);
    let nonce_string = hex::encode(&nonce_bytes);
    println!(
        "Checking-in NP {}, using nonce: {} ({} bytes)",
        dcc_identity,
        nonce_string,
        nonce_bytes.len()
    );

    let payload = dcc_identity.sign(&nonce_bytes).unwrap().to_bytes();

    update_check_and_decode!(
        pic,
        can,
        dcc_identity.to_ic_principal(),
        "node_provider_check_in",
        Encode!(
            &dcc_identity.to_bytes_verifying(),
            &payload
        )
        .unwrap(),
        Result<String, String>
    )
}

fn icrc1_account_from_slice(bytes: &[u8]) -> Icrc1Account {
    Icrc1Account {
        owner: Principal::from_slice(bytes),
        subaccount: None,
    }
}

#[test]
fn test_balances_and_transfers() {
    let (pic, c) = create_test_canister();

    let account_a = icrc1_account_from_slice(b"A");
    let account_b = icrc1_account_from_slice(b"B");

    assert_eq!(get_account_balance(&pic, c, &account_a), 0u64);
    assert_eq!(get_account_balance(&pic, c, &account_b), 0u64);

    // Mint 666 tokens on account_a
    let amount_mint = 666u64 * DC_TOKEN_DECIMALS_DIV;
    let amount_send = 111u64 * DC_TOKEN_DECIMALS_DIV;
    let response = mint_tokens_for_test(&pic, c, &account_a, amount_mint);
    println!("mint_tokens_for_test response: {:?}", response);

    assert_eq!(get_account_balance(&pic, c, &account_a), amount_mint);
    assert_eq!(get_account_balance(&pic, c, &account_b), 0u64);

    let response = transfer_funds(&pic, c, &account_a, &account_b, amount_send);

    assert!(response.is_ok());

    println!("icrc1_transfer response: {:?}", response);

    assert_eq!(
        get_account_balance(&pic, c, &account_a),
        amount_mint - amount_send - DC_TOKEN_TRANSFER_FEE_E9S
    );
    assert_eq!(get_account_balance(&pic, c, &account_b), amount_send);

    upgrade_test_canister(&pic, c).expect("Canister upgrade failed");

    assert_eq!(
        get_account_balance(&pic, c, &account_a),
        amount_mint - amount_send - DC_TOKEN_TRANSFER_FEE_E9S
    );
    assert_eq!(get_account_balance(&pic, c, &account_b), amount_send);
}

fn commit(pic: &PocketIc, can: Principal) {
    let no_args: Vec<u8> = encode_one(()).expect("failed to encode");
    update_check_and_decode!(
        &pic,
        can,
        Principal::anonymous(),
        "run_periodic_task",
        no_args,
        ()
    )
}

#[test]
fn test_np_registration_and_check_in() {
    let (p, c) = create_test_canister();

    let ts_ns = get_timestamp_ns(&p, c);

    // Register one NP and commit one block, to make sure there is something in the ledger.
    let (np_past, _reg1) = np_register(&p, c, b"np_past", 0);
    assert_eq!(
        np_check_in(&p, c, &np_past).unwrap(),
        "Signature verified, check in successful.".to_string()
    );
    commit(&p, c);
    // np_past now has 50 * 100 = 5000 tokens
    let amount = 5000u64 * DC_TOKEN_DECIMALS_DIV;
    assert_eq!(
        get_account_balance(&p, c, &np_past.as_icrc_compatible_account().into()),
        amount
    );

    // Since the ledger is not empty, NP registration requires a payment of the registration fee
    let (np1, reg1) = np_register(&p, c, b"np1", 0);
    assert_eq!(reg1.unwrap_err(), "InsufficientFunds: account w7shl-xsw5s-kduqo-kx77s-nxs35-4zdh3-3tpob-nr4yc-2c6zw-qeyzj-rqe has 0 and requested 500_000_000".to_string());
    assert_eq!(
        get_account_balance(&p, c, &np1.as_icrc_compatible_account().into()),
        0u64
    );

    let (np2, reg2) = np_register(&p, c, b"np2", 0);
    assert_eq!(reg2.unwrap_err(), "InsufficientFunds: account ejigd-cloes-e7n46-7uop4-cwkfh-ccuxk-ry2cf-adfeg-3ik3k-znob6-pae has 0 and requested 500_000_000".to_string());
    commit(&p, c);

    // Initial reputation is 0
    assert_eq!(identity_reputation_get(&p, c, &np1.to_bytes_verifying()), 0);
    assert_eq!(identity_reputation_get(&p, c, &np2.to_bytes_verifying()), 0);

    let np_past_acct = np_past.as_icrc_compatible_account().into();
    let np2_acct = np2.as_icrc_compatible_account().into();
    let amount_send = 10u64 * DC_TOKEN_DECIMALS_DIV;
    let response = transfer_funds(&p, c, &np_past_acct, &np2_acct, amount_send);

    assert!(response.is_ok());

    assert_eq!(
        get_account_balance(&p, c, &np_past.as_icrc_compatible_account().into()),
        amount - amount_send - DC_TOKEN_TRANSFER_FEE_E9S
    );
    assert_eq!(
        get_account_balance(&p, c, &np2.as_icrc_compatible_account().into()),
        amount_send
    );

    // Now np1 still can't register
    let (np1, reg1) = np_register(&p, c, b"np1", 0);
    assert_eq!(reg1.unwrap_err(), "InsufficientFunds: account w7shl-xsw5s-kduqo-kx77s-nxs35-4zdh3-3tpob-nr4yc-2c6zw-qeyzj-rqe has 0 and requested 500_000_000".to_string());
    assert_eq!(
        get_account_balance(&p, c, &np1.as_icrc_compatible_account().into()),
        0u64
    );

    // But np2 can, since it has enough funds
    let (np2, reg2) = np_register(&p, c, b"np2", 0);
    assert_eq!(reg2.unwrap(), "ok".to_string());
    assert_eq!(
        get_account_balance(&p, c, &np2.as_icrc_compatible_account().into()),
        9500000000u64
    );

    upgrade_test_canister(&p, c).expect("Canister upgrade failed");
    assert_eq!(
        get_account_balance(&p, c, &np2.as_icrc_compatible_account().into()),
        9500000000u64
    );

    assert_eq!(
        get_account_balance(&p, c, &np1.as_icrc_compatible_account().into()),
        0u64
    );

    commit(&p, c);
    // check in np2
    assert_eq!(
        np_check_in(&p, c, &np2).unwrap(),
        "Signature verified, check in successful.".to_string()
    );
    test_ffwd_to_next_block(ts_ns, &p, c);
    // Now np2 got a reward of 50 tokens distributed to it
    // The balance is 50 (reward) + 10 (np_past transfer) - 0.5 (reg fee) - 0.5 (check in) = 59000000000 e9s
    assert_eq!(
        get_account_balance(&p, c, &np2.as_icrc_compatible_account().into()),
        59000000000u64
    );

    upgrade_test_canister(&p, c).expect("Canister upgrade failed");
    assert_eq!(
        get_account_balance(&p, c, &np2.as_icrc_compatible_account().into()),
        59000000000u64
    );

    assert_eq!(
        get_account_balance(&p, c, &np1.as_icrc_compatible_account().into()),
        0u64
    );

    // At this point NP1 did not register, but NP2 did.
    // Registration sets the initial reputation (can be reconsidered in the future).
    // However, check-in and periodic reward distribution does not increase reputation!
    reward_e9s_per_block_recalculate();
    assert_eq!(identity_reputation_get(&p, c, &np1.to_bytes_verifying()), 0);
    assert_eq!(
        identity_reputation_get(&p, c, &np2.to_bytes_verifying()),
        np_registration_fee_e9s()
    );
}

#[test]
fn test_reputation() {
    let (p, c) = create_test_canister();
    let ts_ns = get_timestamp_ns(&p, c);

    let _ = np_register(&p, c, b"np_past", 2 * DC_TOKEN_DECIMALS_DIV); // ignored, added only to get 1 block
    test_ffwd_to_next_block(ts_ns, &p, c);

    let (np1, reg1) = np_register(&p, c, b"np1", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(reg1.unwrap(), "ok".to_string());
    let (np2, reg2) = np_register(&p, c, b"np2", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(reg2.unwrap(), "ok".to_string());
    let (np3, reg3) = np_register(&p, c, b"np3", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(reg3.unwrap(), "ok".to_string());

    let (u1, r_u1) = user_register(&p, c, b"u1", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(r_u1.unwrap(), "ok".to_string());
    let (u2, r_u2) = user_register(&p, c, b"u2", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(r_u2.unwrap(), "ok".to_string());

    test_ffwd_to_next_block(ts_ns, &p, c);

    assert!(identity_reputation_get(&p, c, &np1.to_bytes_verifying()) > 0);
    assert!(identity_reputation_get(&p, c, &np2.to_bytes_verifying()) > 0);
    assert!(identity_reputation_get(&p, c, &np3.to_bytes_verifying()) > 0);

    assert!(identity_reputation_get(&p, c, &u1.to_bytes_verifying()) > 0);
    assert!(identity_reputation_get(&p, c, &u2.to_bytes_verifying()) > 0);
}
