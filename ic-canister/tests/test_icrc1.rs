mod test_utils;
use candid::{encode_one, Nat, Principal};
use dcc_common::{PERMITTED_DRIFT, TX_WINDOW};
use icrc_ledger_types::icrc1::transfer::{Memo, TransferArg, TransferError};
use pocket_ic::WasmResult;
use test_utils::{create_test_account, create_test_subaccount, TestContext};

#[test]
fn test_basic_transfer() {
    let ctx = TestContext::new();
    let from = create_test_account(1);
    let to = create_test_account(2);

    // Mint some tokens to the sender
    ctx.mint_tokens_for_test(&from, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Perform transfer
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to,
        amount: 500_000_000u64.into(),
        fee: Some(fee),
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );

    assert!(result.is_ok());

    // Check balances
    let from_balance = ctx.get_account_balance(&from);
    let to_balance = ctx.get_account_balance(&to);
    assert_eq!(from_balance, <u64 as Into<Nat>>::into(499_000_000u64)); // Original - amount - fee
    assert_eq!(to_balance, <u64 as Into<Nat>>::into(500_000_000u64));
}

#[test]
fn test_duplicate_transaction() {
    let ctx = TestContext::new();
    let from = create_test_account(3);
    let to = create_test_account(4);

    // Mint tokens
    ctx.mint_tokens_for_test(&from, 2_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    let transfer_arg = TransferArg {
        from_subaccount: None,
        to,
        amount: 500_000_000u64.into(),
        fee: Some(fee),
        created_at_time: Some(ts),
        memo: Some(Memo(vec![1, 2, 3].into())),
    };

    // First transfer should succeed
    let result1 = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg.clone()).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(result1.is_ok());

    // Same transfer should fail as duplicate
    let result2 = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let from = create_test_account(5);
    let to = create_test_account(6);

    // Mint tokens
    ctx.mint_tokens_for_test(&from, 2_000_000_000);

    // Get current timestamp and fee
    let now = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Test too old transaction
    let old_time = now - TX_WINDOW - PERMITTED_DRIFT - 1;
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to,
        amount: 500_000_000u64.into(),
        fee: Some(fee.clone()),
        created_at_time: Some(old_time),
        memo: None,
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
        to,
        amount: 500_000_000u64.into(),
        fee: Some(fee),
        created_at_time: Some(future_time),
        memo: None,
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let from = create_test_account(7);
    let to = create_test_account(8);

    // Mint small amount
    ctx.mint_tokens_for_test(&from, 1_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

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
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let metadata = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let standards = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc1_supported_standards",
        encode_one(()).unwrap(),
        Vec<decent_cloud_canister::canister_backend::icrc1::Icrc1StandardRecord>
    );
    assert!(standards.iter().any(|s| s.name == "ICRC-1"));
}

#[test]
fn test_minting_account() {
    let ctx = TestContext::new();
    let minting_account = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc1_minting_account",
        encode_one(()).unwrap(),
        Option<icrc_ledger_types::icrc1::account::Account>
    );
    assert!(minting_account.is_some());
}

#[test]
fn test_basic_info() {
    let ctx = TestContext::new();

    let name = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc1_name",
        encode_one(()).unwrap(),
        String
    );
    assert!(!name.is_empty());

    let symbol = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc1_symbol",
        encode_one(()).unwrap(),
        String
    );
    assert!(!symbol.is_empty());

    let decimals = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc1_decimals",
        encode_one(()).unwrap(),
        u8
    );
    assert_eq!(decimals, 9);

    let total_supply = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc1_total_supply",
        encode_one(()).unwrap(),
        Nat
    );
    assert!(total_supply > <u64 as Into<Nat>>::into(0u64));
}

#[test]
fn test_fee_handling() {
    let ctx = TestContext::new();
    let from = create_test_account(9);
    let to = create_test_account(10);

    // Mint tokens
    ctx.mint_tokens_for_test(&from, 2_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let correct_fee = ctx.get_transfer_fee();

    // Test wrong fee
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to,
        amount: 1_000_000u64.into(),
        fee: Some(12345u64.into()), // Wrong fee
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
        ctx.pic,
        ctx.canister_id,
        from.owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(result.is_ok());
}

#[test]
fn test_minting_account_transfers() {
    let ctx = TestContext::new();
    let regular_account = create_test_account(11);

    // Get minting account
    let minting_account = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc1_minting_account",
        encode_one(()).unwrap(),
        Option<icrc_ledger_types::icrc1::account::Account>
    )
    .unwrap();

    // Get current timestamp
    let ts = ctx.get_timestamp_ns();

    // Mint tokens to regular account
    ctx.mint_tokens_for_test(&regular_account, 2_000_000_000);

    // Test transfer to minting account (burn) with zero fee
    let transfer_arg = TransferArg {
        from_subaccount: None,
        to: minting_account,
        amount: 1_000_000u64.into(),
        fee: Some(0u64.into()), // Burn should have no fee
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
        fee: Some(ctx.get_transfer_fee()),
        created_at_time: Some(ts),
        memo: None,
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let owner = create_test_account(12).owner;
    let from = create_test_subaccount(owner, 1);
    let to = create_test_subaccount(owner, 2);

    // Mint to first subaccount
    ctx.mint_tokens_for_test(&from, 2_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

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
        ctx.pic,
        ctx.canister_id,
        owner,
        "icrc1_transfer",
        candid::encode_one(transfer_arg).unwrap(),
        Result<Nat, TransferError>
    );
    assert!(result.is_ok());

    // Verify balances
    let from_balance = ctx.get_account_balance(&from);
    let to_balance = ctx.get_account_balance(&to);
    assert_eq!(
        from_balance,
        Nat::from(2_000_000_000u64) - Nat::from(1_000_000u64) - fee
    );
    assert_eq!(to_balance, <u64 as Into<Nat>>::into(1_000_000u64));
}
