use candid::{encode_one, Nat, Principal};
use dcc_common::{BLOCK_INTERVAL_SECS, FIRST_BLOCK_TIMESTAMP_NS};
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc1::transfer::Memo;
use icrc_ledger_types::icrc2::allowance::AllowanceArgs;
use icrc_ledger_types::icrc2::approve::{ApproveArgs, ApproveError};
use icrc_ledger_types::icrc2::transfer_from::{TransferFromArgs, TransferFromError};
use once_cell::sync::Lazy;
use pocket_ic::{PocketIc, WasmResult};
use std::path::{Path, PathBuf};
use std::process::Command;

// Reuse test infrastructure from ICRC-1 tests
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

fn mint_tokens_for_test(pic: &PocketIc, can_id: Principal, acct: &Account, amount: u64) -> Nat {
    update_check_and_decode!(
        pic,
        can_id,
        acct.owner,
        "mint_tokens_for_test",
        candid::encode_args((acct, amount, None::<Option<Memo>>)).unwrap(),
        Nat
    )
}

fn get_timestamp_ns(pic: &PocketIc, can: Principal) -> u64 {
    query_check_and_decode!(pic, can, "get_timestamp_ns", encode_one(()).unwrap(), u64)
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
fn test_basic_approve() {
    let (pic, can_id) = create_test_canister();

    let owner = Account {
        owner: Principal::from_slice(&[1; 29]),
        subaccount: None,
    };
    let spender = Account {
        owner: Principal::from_slice(&[2; 29]),
        subaccount: None,
    };

    // Mint tokens to owner
    mint_tokens_for_test(&pic, can_id, &owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Approve spending
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: spender.clone(),
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Check allowance
    let allowance_args = AllowanceArgs {
        account: owner.clone(),
        spender: spender.clone(),
    };

    let allowance = query_check_and_decode!(
        pic,
        can_id,
        "icrc2_allowance",
        candid::encode_one(allowance_args).unwrap(),
        icrc_ledger_types::icrc2::allowance::Allowance
    );

    assert_eq!(allowance.allowance, Nat::from(500_000_000u64));
    assert_eq!(allowance.expires_at, None);
}

#[test]
fn test_approve_with_expiration() {
    let (pic, can_id) = create_test_canister();

    let owner = Account {
        owner: Principal::from_slice(&[3; 29]),
        subaccount: None,
    };
    let spender = Account {
        owner: Principal::from_slice(&[4; 29]),
        subaccount: None,
    };

    // Mint tokens to owner
    mint_tokens_for_test(&pic, can_id, &owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);
    let expires_at = ts + 1_000_000_000; // 1 second in the future

    // Approve spending with expiration
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: spender.clone(),
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: Some(expires_at),
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Check allowance
    let allowance_args = AllowanceArgs {
        account: owner,
        spender,
    };

    let allowance = query_check_and_decode!(
        pic,
        can_id,
        "icrc2_allowance",
        candid::encode_one(allowance_args).unwrap(),
        icrc_ledger_types::icrc2::allowance::Allowance
    );

    assert_eq!(allowance.allowance, Nat::from(500_000_000u64));
    assert_eq!(allowance.expires_at, Some(expires_at));
}

#[test]
fn test_transfer_from() {
    let (pic, can_id) = create_test_canister();

    let owner = Account {
        owner: Principal::from_slice(&[5; 29]),
        subaccount: None,
    };
    let spender = Account {
        owner: Principal::from_slice(&[6; 29]),
        subaccount: None,
    };
    let recipient = Account {
        owner: Principal::from_slice(&[7; 29]),
        subaccount: None,
    };

    // Mint tokens to owner
    mint_tokens_for_test(&pic, can_id, &owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Approve spending
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: spender.clone(),
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee.clone()),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Transfer from owner to recipient using spender's allowance
    let transfer_from_args = TransferFromArgs {
        spender_subaccount: None,
        from: owner.clone(),
        to: recipient.clone(),
        amount: 300_000_000u64.into(),
        fee: Some(fee.clone()),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        spender.owner,
        "icrc2_transfer_from",
        candid::encode_one(transfer_from_args).unwrap(),
        Result<Nat, TransferFromError>
    );
    assert!(result.is_ok());

    // Check remaining allowance
    let allowance_args = AllowanceArgs {
        account: owner,
        spender,
    };

    let allowance = query_check_and_decode!(
        pic,
        can_id,
        "icrc2_allowance",
        candid::encode_one(allowance_args).unwrap(),
        icrc_ledger_types::icrc2::allowance::Allowance
    );

    // Remaining allowance should be initial amount minus transfer amount and fee
    assert_eq!(
        allowance.allowance,
        Nat::from(500_000_000u64) - Nat::from(300_000_000u64) - fee
    );
}

#[test]
fn test_expired_allowance() {
    let (pic, can_id) = create_test_canister();

    let owner = Account {
        owner: Principal::from_slice(&[8; 29]),
        subaccount: None,
    };
    let spender = Account {
        owner: Principal::from_slice(&[9; 29]),
        subaccount: None,
    };

    // Mint tokens to owner
    mint_tokens_for_test(&pic, can_id, &owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Approve with immediate expiration
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: spender.clone(),
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: Some(ts), // Expires immediately
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(matches!(
        result,
        Err(ApproveError::Expired { ledger_time: _ })
    ));
}

#[test]
fn test_insufficient_funds_for_approval() {
    let (pic, can_id) = create_test_canister();

    let owner = Account {
        owner: Principal::from_slice(&[10; 29]),
        subaccount: None,
    };
    let spender = Account {
        owner: Principal::from_slice(&[11; 29]),
        subaccount: None,
    };

    // Mint very small amount of tokens to owner (less than fee)
    mint_tokens_for_test(&pic, can_id, &owner, 100);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Try to approve
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 50u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(matches!(
        result,
        Err(ApproveError::InsufficientFunds { balance: _ })
    ));
}

#[test]
fn test_self_approval_prevention() {
    let (pic, can_id) = create_test_canister();

    let owner = Account {
        owner: Principal::from_slice(&[12; 29]),
        subaccount: None,
    };

    // Mint tokens to owner
    mint_tokens_for_test(&pic, can_id, &owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // Try to approve self
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: owner.clone(), // Same as owner
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(matches!(
        result,
        Err(ApproveError::GenericError {
            error_code: _,
            message: _
        })
    ));
}

#[test]
fn test_expected_allowance() {
    let (pic, can_id) = create_test_canister();

    let owner = Account {
        owner: Principal::from_slice(&[13; 29]),
        subaccount: None,
    };
    let spender = Account {
        owner: Principal::from_slice(&[14; 29]),
        subaccount: None,
    };

    // Mint tokens to owner
    mint_tokens_for_test(&pic, can_id, &owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = get_timestamp_ns(&pic, can_id);
    let fee = get_transfer_fee(&pic, &can_id);

    // First approval
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: spender.clone(),
        amount: 500_000_000u64.into(),
        expected_allowance: Some(0u64.into()), // Expect no existing allowance
        expires_at: None,
        fee: Some(fee.clone()),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Second approval with wrong expected allowance
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: spender.clone(),
        amount: 300_000_000u64.into(),
        expected_allowance: Some(0u64.into()), // Wrong expectation
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        pic,
        can_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(matches!(
        result,
        Err(ApproveError::AllowanceChanged {
            current_allowance: _
        })
    ));
}
