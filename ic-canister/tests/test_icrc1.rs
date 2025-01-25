use candid::{encode_one, Nat, Principal};
use dcc_common::{BLOCK_INTERVAL_SECS, FIRST_BLOCK_TIMESTAMP_NS, PERMITTED_DRIFT, TX_WINDOW};
use icrc_ledger_types::icrc1::account::Account as Icrc1Account;
use icrc_ledger_types::icrc1::transfer::{Memo, TransferArg, TransferError};
use once_cell::sync::Lazy;
use pocket_ic::{PocketIc, WasmResult};
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
        .current_dir(path.join("ic-canister"))
        .output()
        .unwrap();
    path.push("target/wasm32-unknown-unknown/release/decent_cloud_canister.wasm");
    fs_err::read(path).unwrap()
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

    (pic, canister_id)
}

fn get_timestamp_ns(pic: &PocketIc, can: Principal) -> u64 {
    query_check_and_decode!(pic, can, "get_timestamp_ns", encode_one(()).unwrap(), u64)
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
        candid::encode_args((acct, amount, None::<Option<Memo>>)).unwrap(),
        Nat
    )
}

fn get_account_balance(pic: &PocketIc, can: &Principal, account: &Icrc1Account) -> Nat {
    query_check_and_decode!(
        pic,
        *can,
        "icrc1_balance_of",
        encode_one(account).expect("failed to encode"),
        Nat
    )
}

fn get_transfer_fee(pic: &PocketIc, can: &Principal) -> Nat {
    query_check_and_decode!(
        pic,
        *can,
        "icrc1_fee",
        encode_one(()).expect("failed to encode"),
        Nat
    )
}

#[test]
fn test_basic_transfer() {
    let (pic, can_id) = create_test_canister();

    let from = Icrc1Account {
        owner: Principal::from_slice(&[1; 29]),
        subaccount: None,
    };
    let to = Icrc1Account {
        owner: Principal::from_slice(&[2; 29]),
        subaccount: None,
    };

    // Mint some tokens to the sender
    mint_tokens_for_test(&pic, can_id, &from, 1_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Perform transfer
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: to.clone(),
        amount: 500_000_000u64.into(),
        fee: Some(fee),
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );

    assert!(result.is_ok());

    // Check balances
    let from_balance = get_account_balance(&pic, &can_id, &from);
    let to_balance = get_account_balance(&pic, &can_id, &to);
    assert_eq!(from_balance, <u64 as Into<Nat>>::into(499_000_000u64)); // Original - amount - fee
    assert_eq!(to_balance, <u64 as Into<Nat>>::into(500_000_000u64));
}

#[test]
fn test_duplicate_transaction() {
    let (pic, can_id) = create_test_canister();

    let from = Icrc1Account {
        owner: Principal::from_slice(&[3; 29]),
        subaccount: None,
    };
    let to = Icrc1Account {
        owner: Principal::from_slice(&[4; 29]),
        subaccount: None,
    };

    // Mint tokens
    mint_tokens_for_test(&pic, can_id, &from, 2_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: to.clone(),
        amount: 500_000_000u64.into(),
        fee: Some(fee),
        created_at_time: Some(ts),
        memo: Some(Memo(vec![1, 2, 3].into())),
    };

    // First transfer should succeed
    let result1 = update_check_and_decode!(
        pic,
        can_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg.clone()).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(result1.is_ok());

    // Same transfer should fail as duplicate
    let result2 = update_check_and_decode!(
        pic,
        can_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(matches!(
        result2,
        Err(TransferError::Duplicate { duplicate_of: _ })
    ));
}

#[test]
fn test_transaction_timing() {
    let (pic, can_id) = create_test_canister();

    let from = Icrc1Account {
        owner: Principal::from_slice(&[5; 29]),
        subaccount: None,
    };
    let to = Icrc1Account {
        owner: Principal::from_slice(&[6; 29]),
        subaccount: None,
    };

    // Mint tokens
    mint_tokens_for_test(&pic, can_id, &from, 2_000_000_000);

    // Get current timestamp and fee
    let now = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Test too old transaction
    let old_time = now - TX_WINDOW - PERMITTED_DRIFT - 1;
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: to.clone(),
        amount: 500_000_000u64.into(),
        fee: Some(fee.clone()),
        created_at_time: Some(old_time),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(matches!(result, Err(TransferError::TooOld)));

    // Test future transaction
    let future_time = now + PERMITTED_DRIFT + 1;
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: to.clone(),
        amount: 500_000_000u64.into(),
        fee: Some(fee),
        created_at_time: Some(future_time),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(matches!(
        result,
        Err(TransferError::CreatedInFuture { ledger_time: _ })
    ));
}

#[test]
fn test_insufficient_funds() {
    let (pic, can_id) = create_test_canister();

    let from = Icrc1Account {
        owner: Principal::from_slice(&[7; 29]),
        subaccount: None,
    };
    let to = Icrc1Account {
        owner: Principal::from_slice(&[8; 29]),
        subaccount: None,
    };

    // Mint small amount
    mint_tokens_for_test(&pic, can_id, &from, 1_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Try to transfer more than available
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to,
        amount: 2_000_000u64.into(),
        fee: Some(fee),
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );

    assert!(matches!(
        result,
        Err(TransferError::InsufficientFunds { balance: _ })
    ));
}

#[test]
fn test_metadata() {
    let (pic, can_id) = create_test_canister();
    let metadata = query_check_and_decode!(
        pic,
        can_id,
        "icrc1_metadata",
        encode_one(()).unwrap(),
        Vec<(
            String,
            icrc_ledger_types::icrc::generic_metadata_value::MetadataValue
        )>
    );
    assert!(metadata.iter().any(|(k, _)| k == "icrc1:name"));
    assert!(metadata.iter().any(|(k, _)| k == "icrc1:symbol"));
    assert!(metadata.iter().any(|(k, _)| k == "icrc1:decimals"));
    assert!(metadata.iter().any(|(k, _)| k == "icrc1:fee"));
    assert!(metadata.iter().any(|(k, _)| k == "icrc1:logo"));
}

#[test]
fn test_supported_standards() {
    let (pic, can_id) = create_test_canister();
    let standards = query_check_and_decode!(
        pic,
        can_id,
        "icrc1_supported_standards",
        encode_one(()).unwrap(),
        Vec<decent_cloud_canister::canister_backend::icrc1::Icrc1StandardRecord>
    );
    assert!(standards.iter().any(|s| s.name == "ICRC-1"));
}

#[test]
fn test_minting_account() {
    let (pic, can_id) = create_test_canister();
    let minting_account = query_check_and_decode!(
        pic,
        can_id,
        "icrc1_minting_account",
        encode_one(()).unwrap(),
        Option<Icrc1Account>
    );
    assert!(minting_account.is_some());
}

#[test]
fn test_basic_info() {
    let (pic, can_id) = create_test_canister();

    let name = query_check_and_decode!(pic, can_id, "icrc1_name", encode_one(()).unwrap(), String);
    assert!(!name.is_empty());

    let symbol =
        query_check_and_decode!(pic, can_id, "icrc1_symbol", encode_one(()).unwrap(), String);
    assert!(!symbol.is_empty());

    let decimals =
        query_check_and_decode!(pic, can_id, "icrc1_decimals", encode_one(()).unwrap(), u8);
    assert_eq!(decimals, 9);

    let total_supply = query_check_and_decode!(
        pic,
        can_id,
        "icrc1_total_supply",
        encode_one(()).unwrap(),
        Nat
    );
    assert!(total_supply > <u64 as Into<Nat>>::into(0u64));
}

#[test]
fn test_fee_handling() {
    let (pic, can_id) = create_test_canister();

    let from = Icrc1Account {
        owner: Principal::from_slice(&[9; 29]),
        subaccount: None,
    };
    let to = Icrc1Account {
        owner: Principal::from_slice(&[10; 29]),
        subaccount: None,
    };

    // Mint tokens
    mint_tokens_for_test(&pic, can_id, &from, 2_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let correct_fee = get_transfer_fee(&pic, &can_id);

    // Test wrong fee
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: to.clone(),
        amount: 1_000_000u64.into(),
        fee: Some(12345u64.into()), // Wrong fee
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(matches!(
        result,
        Err(TransferError::BadFee { expected_fee: _ })
    ));

    // Test with correct fee
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to,
        amount: 1_000_000u64.into(),
        fee: Some(correct_fee),
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(result.is_ok());
}

#[test]
fn test_minting_account_transfers() {
    let (pic, can_id) = create_test_canister();

    let regular_account = Icrc1Account {
        owner: Principal::from_slice(&[11; 29]),
        subaccount: None,
    };

    // Get minting account
    let minting_account = query_check_and_decode!(
        pic,
        can_id,
        "icrc1_minting_account",
        encode_one(()).unwrap(),
        Option<Icrc1Account>
    )
    .unwrap();

    // Get current timestamp
    let ts = get_timestamp_ns(&pic, can_id);

    // Mint tokens to regular account
    mint_tokens_for_test(&pic, can_id, &regular_account, 2_000_000_000);

    // Test transfer to minting account (burn) with zero fee
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: minting_account.clone(),
        amount: 1_000_000u64.into(),
        fee: Some(0u64.into()), // Burn should have no fee
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        regular_account.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(result.is_ok());

    // Test burn with non-zero fee should fail
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: minting_account,
        amount: 1_000_000u64.into(),
        fee: Some(get_transfer_fee(&pic, &can_id)),
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        regular_account.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(matches!(
        result,
        Err(TransferError::BadFee { expected_fee: _ })
    ));
}

#[test]
fn test_subaccount_transfers() {
    let (pic, can_id) = create_test_canister();

    let owner = Principal::from_slice(&[12; 29]);
    let from = Icrc1Account {
        owner,
        subaccount: Some([1; 32]),
    };
    let to = Icrc1Account {
        owner,
        subaccount: Some([2; 32]),
    };

    // Mint to first subaccount
    mint_tokens_for_test(&pic, can_id, &from, 2_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Transfer between subaccounts
    let transfer_arg = TransferArg {
        from_subaccount: Some([1; 32]),
        to,
        amount: 1_000_000u64.into(),
        fee: Some(fee.clone()),
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(result.is_ok());

    // Verify balances
    let from_balance = get_account_balance(&pic, &can_id, &from);
    let to_balance = get_account_balance(&pic, &can_id, &to);
    assert_eq!(
        from_balance,
        Nat::from(2_000_000_000u64) - Nat::from(1_000_000u64) - fee
    );
    assert_eq!(to_balance, <u64 as Into<Nat>>::into(1_000_000u64));
}
