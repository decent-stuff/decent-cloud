mod test_utils;
use candid::{Nat, Principal};
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc2::allowance::AllowanceArgs;
use icrc_ledger_types::icrc2::approve::{ApproveArgs, ApproveError};
use icrc_ledger_types::icrc2::transfer_from::{TransferFromArgs, TransferFromError};
use pocket_ic::WasmResult;
use test_utils::{create_test_account, TestContext};

#[test]
fn test_basic_approve() {
    let ctx = TestContext::new();
    let owner = create_test_account(1);
    let spender = create_test_account(2);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Approve spending
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
        ctx.pic,
        ctx.canister_id,
        "icrc2_allowance",
        candid::encode_one(allowance_args).unwrap(),
        icrc_ledger_types::icrc2::allowance::Allowance
    );

    assert_eq!(allowance.allowance, Nat::from(500_000_000u64));
    assert_eq!(allowance.expires_at, None);
}

#[test]
fn test_approve_with_expiration() {
    let ctx = TestContext::new();
    let owner = create_test_account(3);
    let spender = create_test_account(4);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();
    let expires_at = ts + 1_000_000_000; // 1 second in the future

    // Approve spending with expiration
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: Some(expires_at),
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
        ctx.pic,
        ctx.canister_id,
        "icrc2_allowance",
        candid::encode_one(allowance_args).unwrap(),
        icrc_ledger_types::icrc2::allowance::Allowance
    );

    assert_eq!(allowance.allowance, Nat::from(500_000_000u64));
    assert_eq!(allowance.expires_at, Some(expires_at));
}

#[test]
fn test_transfer_from() {
    let ctx = TestContext::new();
    let owner = create_test_account(5);
    let spender = create_test_account(6);
    let recipient = create_test_account(7);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Approve spending
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee.clone()),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Transfer from owner to recipient using spender's allowance
    let transfer_from_args = TransferFromArgs {
        spender_subaccount: None,
        from: owner,
        to: recipient,
        amount: 300_000_000u64.into(),
        fee: Some(fee.clone()),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let owner = create_test_account(8);
    let spender = create_test_account(9);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Approve with immediate expiration
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: Some(ts), // Expires immediately
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let owner = create_test_account(10);
    let spender = create_test_account(11);

    // Mint very small amount of tokens to owner (less than fee)
    ctx.mint_tokens_for_test(&owner, 100);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

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
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let owner = create_test_account(12);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Try to approve self
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: owner, // Same as owner
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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
    let ctx = TestContext::new();
    let owner = create_test_account(13);
    let spender = create_test_account(14);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // First approval
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 500_000_000u64.into(),
        expected_allowance: Some(0u64.into()), // Expect no existing allowance
        expires_at: None,
        fee: Some(fee.clone()),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Second approval with wrong expected allowance
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 300_000_000u64.into(),
        expected_allowance: Some(0u64.into()), // Wrong expectation
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
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

#[test]
fn test_transfer_from_with_expired_allowance() {
    let ctx = TestContext::new();
    let owner = create_test_account(15);
    let spender = create_test_account(16);
    let recipient = create_test_account(17);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();
    let expires_at = ts + 1_000_000_000; // 1 second in the future

    // Approve spending with expiration
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: Some(expires_at),
        fee: Some(fee.clone()),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Wait 2s for expiration
    ctx.ffwd_to_next_block(ts);
    ctx.ffwd_to_next_block(ts);

    // Try to transfer after expiration
    let transfer_from_args = TransferFromArgs {
        spender_subaccount: None,
        from: owner,
        to: recipient,
        amount: 300_000_000u64.into(),
        fee: None,
        memo: None,
        created_at_time: Some(ctx.get_timestamp_ns()),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        spender.owner,
        "icrc2_transfer_from",
        candid::encode_one(transfer_from_args).unwrap(),
        Result<Nat, TransferFromError>
    );
    assert!(matches!(
        result,
        Err(TransferFromError::InsufficientAllowance { allowance: _ })
    ));
}

#[test]
fn test_transfer_from_with_subaccount() {
    let ctx = TestContext::new();
    let owner = create_test_account(18);
    let spender = create_test_account(19);
    let recipient = create_test_account(20);
    let subaccount1 = [1u8; 32];
    let subaccount2 = [2u8; 32];

    // Mint tokens to owner, and transfer half to subaccount2
    ctx.mint_tokens_for_test(&owner, 2_000_000_000);
    ctx.transfer_funds(
        &owner,
        &Account {
            subaccount: Some(subaccount2),
            ..owner
        },
        1_000_000_000,
    )
    .unwrap();

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    let icrc2_approve_spending = |from_subaccount: Option<[u8; 32]>| -> Result<Nat, ApproveError> {
        let approve_args_default = ApproveArgs {
            from_subaccount,
            spender,
            amount: 500_000_000u64.into(),
            expected_allowance: None,
            expires_at: None,
            fee: Some(fee.clone()),
            memo: None,
            created_at_time: Some(ts),
        };

        update_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            owner.owner,
            "icrc2_approve",
            candid::encode_one(approve_args_default).unwrap(),
            Result<Nat, ApproveError>
        )
    };

    let icrc2_transfer = |spender_subaccount: Option<[u8; 32]>,
                          from_subaccount: Option<[u8; 32]>|
     -> Result<Nat, TransferFromError> {
        let transfer_from_args = TransferFromArgs {
            spender_subaccount,
            from: Account {
                subaccount: from_subaccount,
                ..owner
            },
            to: recipient,
            amount: 300_000_000u64.into(),
            fee: Some(fee.clone()),
            memo: None,
            created_at_time: Some(ts),
        };

        update_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            spender.owner,
            "icrc2_transfer_from",
            candid::encode_one(transfer_from_args).unwrap(),
            Result<Nat, TransferFromError>
        )
    };

    // Approve spending from main account, no subaccount
    let result = icrc2_approve_spending(None);
    assert!(result.is_ok());

    assert_eq!(
        icrc2_transfer(None, Some(subaccount1))
            .unwrap_err()
            .to_string(),
        "the spender account does not have sufficient allowance, current allowance is 0"
    );
    assert_eq!(
        icrc2_transfer(Some(subaccount1), Some(subaccount1))
            .unwrap_err()
            .to_string(),
        "the spender account does not have sufficient allowance, current allowance is 0"
    );

    // Now approve spending from subaccount2
    let result = icrc2_approve_spending(Some(subaccount2));
    println!("result: {:#?}", result.unwrap());

    // Spending from subaccount1 still fails
    assert_eq!(
        icrc2_transfer(None, Some(subaccount1))
            .unwrap_err()
            .to_string(),
        "the spender account does not have sufficient allowance, current allowance is 0"
    );
    assert_eq!(
        icrc2_transfer(Some(subaccount1), Some(subaccount1))
            .unwrap_err()
            .to_string(),
        "the spender account does not have sufficient allowance, current allowance is 0"
    );

    // Spender subaccount1 fails
    assert_eq!(
        icrc2_transfer(Some(subaccount1), Some(subaccount1))
            .unwrap_err()
            .to_string(),
        "the spender account does not have sufficient allowance, current allowance is 0"
    );
    // Spender subaccount2 fails
    assert_eq!(
        icrc2_transfer(Some(subaccount2), Some(subaccount1))
            .unwrap_err()
            .to_string(),
        "the spender account does not have sufficient allowance, current allowance is 0"
    );
    // Spender no subaccount FROM subaccount2 succeeds
    let result = icrc2_transfer(None, Some(subaccount2));
    println!("result: {:#?}", result.unwrap());
}

#[test]
fn test_zero_amount_approval() {
    let ctx = TestContext::new();
    let owner = create_test_account(21);
    let spender = create_test_account(22);

    // Mint tokens to owner (enough for fee)
    ctx.mint_tokens_for_test(&owner, 1_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Approve zero amount
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 0u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Check allowance is zero
    let allowance_args = AllowanceArgs {
        account: owner,
        spender,
    };

    let allowance = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc2_allowance",
        candid::encode_one(allowance_args).unwrap(),
        icrc_ledger_types::icrc2::allowance::Allowance
    );

    assert_eq!(allowance.allowance, Nat::from(0u64));
}

#[test]
fn test_approve_with_memo() {
    let ctx = TestContext::new();
    let owner = create_test_account(23);
    let spender = create_test_account(24);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Approve with memo
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender,
        amount: 500_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee),
        memo: Some(icrc_ledger_types::icrc1::transfer::Memo(
            vec![1, 2, 3, 4].into(),
        )),
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());
}

#[test]
fn test_multiple_approvals() {
    let ctx = TestContext::new();
    let owner = create_test_account(25);
    let spender1 = create_test_account(26);
    let spender2 = create_test_account(27);

    // Mint tokens to owner
    ctx.mint_tokens_for_test(&owner, 1_000_000_000);

    // Get current timestamp and fee
    let ts = ctx.get_timestamp_ns();
    let fee = ctx.get_transfer_fee();

    // Approve first spender
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: spender1,
        amount: 300_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee.clone()),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Approve second spender
    let approve_args = ApproveArgs {
        from_subaccount: None,
        spender: spender2,
        amount: 200_000_000u64.into(),
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee),
        memo: None,
        created_at_time: Some(ts),
    };

    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        owner.owner,
        "icrc2_approve",
        candid::encode_one(approve_args).unwrap(),
        Result<Nat, ApproveError>
    );
    assert!(result.is_ok());

    // Check both allowances
    let allowance_args = AllowanceArgs {
        account: owner,
        spender: spender1,
    };

    let allowance1 = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc2_allowance",
        candid::encode_one(allowance_args).unwrap(),
        icrc_ledger_types::icrc2::allowance::Allowance
    );

    let allowance_args = AllowanceArgs {
        account: owner,
        spender: spender2,
    };

    let allowance2 = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "icrc2_allowance",
        candid::encode_one(allowance_args).unwrap(),
        icrc_ledger_types::icrc2::allowance::Allowance
    );

    assert_eq!(allowance1.allowance, Nat::from(300_000_000u64));
    assert_eq!(allowance2.allowance, Nat::from(200_000_000u64));
}
