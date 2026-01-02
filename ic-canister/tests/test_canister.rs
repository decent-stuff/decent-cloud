mod test_utils;
use crate::test_utils::{
    test_get_id_reputation, test_icrc1_account_from_slice, test_ledger_entries,
    test_provider_check_in, test_provider_register, test_user_register, TestContext,
};
use candid::{encode_one, Nat, Principal};
use dcc_common::{
    reward_e9s_per_block_recalculate, TokenAmountE9s, DC_TOKEN_DECIMALS, DC_TOKEN_DECIMALS_DIV,
    DC_TOKEN_NAME, DC_TOKEN_SYMBOL, DC_TOKEN_TOTAL_SUPPLY, DC_TOKEN_TRANSFER_FEE_E9S,
    MINTING_ACCOUNT_ICRC1,
};
use decent_cloud_canister::canister_backend::icrc1::Icrc1StandardRecord;
use decent_cloud_canister::DC_TOKEN_LOGO;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;

#[test]
fn test_get_set_timestamp() {
    let ctx = TestContext::new();
    let no_args = encode_one(()).expect("failed to encode");
    let timestamp = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "get_timestamp_ns",
        no_args.clone(),
        u64
    );

    assert!(timestamp > 1600000000000000000u64);

    let ts_1 = encode_one(2000000000000000000u64).unwrap();
    update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        Principal::anonymous(),
        "set_timestamp_ns",
        ts_1,
        ()
    );

    assert_eq!(ctx.get_timestamp_ns(), 2000000000000000000u64);
}

#[test]
fn test_icrc1_compatibility() {
    let ctx = TestContext::new();
    let no_args = encode_one(()).expect("failed to encode");

    assert_eq!(
        query_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            "icrc1_metadata",
            no_args.clone(),
            Vec<(String, MetadataValue)>
        ),
        vec![
            MetadataValue::entry("icrc1:decimals", DC_TOKEN_DECIMALS as u64),
            MetadataValue::entry("icrc1:name", DC_TOKEN_NAME.to_string()),
            MetadataValue::entry("icrc1:symbol", DC_TOKEN_SYMBOL.to_string()),
            MetadataValue::entry("icrc1:fee", DC_TOKEN_TRANSFER_FEE_E9S),
            MetadataValue::entry("icrc1:logo", DC_TOKEN_LOGO.to_string()),
        ]
    );

    assert_eq!(
        query_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            "icrc1_name",
            no_args.clone(),
            String
        ),
        DC_TOKEN_NAME.to_string()
    );

    assert_eq!(
        query_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            "icrc1_symbol",
            no_args.clone(),
            String
        ),
        DC_TOKEN_SYMBOL.to_string()
    );

    assert_eq!(
        query_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            "icrc1_decimals",
            no_args.clone(),
            u8
        ),
        DC_TOKEN_DECIMALS
    );

    assert_eq!(
        query_check_and_decode!(ctx.pic, ctx.canister_id, "icrc1_fee", no_args.clone(), Nat),
        DC_TOKEN_TRANSFER_FEE_E9S
    );

    assert_eq!(
        query_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            "icrc1_total_supply",
            no_args.clone(),
            Nat
        ),
        DC_TOKEN_TOTAL_SUPPLY
    );

    assert_eq!(
        query_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            "icrc1_minting_account",
            no_args.clone(),
            Option<icrc_ledger_types::icrc1::account::Account>
        ),
        Some(MINTING_ACCOUNT_ICRC1)
    );

    assert_eq!(
        query_check_and_decode!(
            ctx.pic,
            ctx.canister_id,
            "icrc1_supported_standards",
            no_args.clone(),
            Vec<Icrc1StandardRecord>
        ),
        vec![
            Icrc1StandardRecord {
                name: "ICRC-1".to_string(),
                url: "https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-1".to_string(),
            },
            Icrc1StandardRecord {
                name: "ICRC-2".to_string(),
                url: "https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-2".to_string(),
            }
        ]
    );
}

#[test]
fn test_balances_and_transfers() {
    let ctx = TestContext::new();

    let account_a = test_icrc1_account_from_slice(b"A");
    let account_b = test_icrc1_account_from_slice(b"B");

    assert_eq!(ctx.get_account_balance(&account_a), 0u64);
    assert_eq!(ctx.get_account_balance(&account_b), 0u64);

    // Mint 666 tokens on account_a
    let amount_mint = 666 * DC_TOKEN_DECIMALS_DIV;
    let amount_send = 111 * DC_TOKEN_DECIMALS_DIV;
    let response = ctx.mint_tokens_for_test(&account_a, amount_mint);
    println!("mint_tokens_for_test response: {:?}", response);

    assert_eq!(ctx.get_account_balance(&account_a), amount_mint);
    assert_eq!(ctx.get_account_balance(&account_b), 0u64);

    let response = ctx.transfer_funds(&account_a, &account_b, amount_send);
    assert!(response.is_ok());
    println!("icrc1_transfer response: {:?}", response);

    assert_eq!(
        ctx.get_account_balance(&account_a),
        amount_mint - amount_send - DC_TOKEN_TRANSFER_FEE_E9S
    );
    assert_eq!(ctx.get_account_balance(&account_b), amount_send);

    ctx.upgrade().expect("Canister upgrade failed");

    assert_eq!(
        ctx.get_account_balance(&account_a),
        amount_mint - amount_send - DC_TOKEN_TRANSFER_FEE_E9S
    );
    assert_eq!(ctx.get_account_balance(&account_b), amount_send);
}

#[test]
fn test_provider_registration_and_check_in() {
    let ctx = TestContext::new();
    let ts_ns = ctx.get_timestamp_ns();

    // Register one Provider and commit one block, to make sure there is something in the ledger.
    let (prov_past, _reg1) = test_provider_register(&ctx, b"prov_past", 0);
    assert_eq!(
        test_provider_check_in(&ctx, &prov_past).unwrap(),
        "Signature verified, check in successful. You have been charged 0.0 DC tokens".to_string()
    );
    ctx.commit();

    // prov_past now has 50 * 100 = 5000 tokens
    let amount: TokenAmountE9s = 5000u32 as TokenAmountE9s * DC_TOKEN_DECIMALS_DIV;
    assert_eq!(
        ctx.get_account_balance(&prov_past.as_icrc_compatible_account().into()),
        amount
    );

    // Since the ledger is not empty, Provider registration requires a payment of the registration fee
    let (prov1, reg1) = test_provider_register(&ctx, b"prov1", 0);
    assert_eq!(reg1.unwrap_err(), "InsufficientFunds: account oklaa-ptl4i-uqysq-lxgo4-ya4ki-7dt3a-53rry-f7s47-ovxl4-r3rnm-5qe has 0 e9s (0.0 DC tokens) and requested 500000000 e9s (0.500000000 DC tokens)".to_string());
    assert_eq!(
        ctx.get_account_balance(&prov1.as_icrc_compatible_account().into()),
        0u64
    );

    let (prov2, reg2) = test_provider_register(&ctx, b"prov2", 0);
    assert_eq!(reg2.unwrap_err(), "InsufficientFunds: account zrt5x-yw3i6-ez2tr-ua76a-qqbct-o2onk-vrbiw-36wsh-zzbyg-4tkbt-wae has 0 e9s (0.0 DC tokens) and requested 500000000 e9s (0.500000000 DC tokens)".to_string());
    ctx.commit();

    // Initial reputation is 0
    assert_eq!(test_get_id_reputation(&ctx, &prov1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &prov2), 0);

    let prov_past_acct = prov_past.as_icrc_compatible_account().into();
    let prov2_acct = prov2.as_icrc_compatible_account().into();
    let amount_send = 10 * DC_TOKEN_DECIMALS_DIV;
    let response = ctx.transfer_funds(&prov_past_acct, &prov2_acct, amount_send);

    assert!(response.is_ok());

    assert_eq!(
        ctx.get_account_balance(&prov_past.as_icrc_compatible_account().into()),
        amount - amount_send - DC_TOKEN_TRANSFER_FEE_E9S
    );
    assert_eq!(
        ctx.get_account_balance(&prov2.as_icrc_compatible_account().into()),
        amount_send
    );

    // Now prov1 still can't register
    let (prov1, reg1) = test_provider_register(&ctx, b"prov1", 0);
    assert_eq!(reg1.unwrap_err(), "InsufficientFunds: account oklaa-ptl4i-uqysq-lxgo4-ya4ki-7dt3a-53rry-f7s47-ovxl4-r3rnm-5qe has 0 e9s (0.0 DC tokens) and requested 500000000 e9s (0.500000000 DC tokens)".to_string());
    assert_eq!(
        ctx.get_account_balance(&prov1.as_icrc_compatible_account().into()),
        0u64
    );

    // But prov2 can, since it has enough funds
    let (prov2, reg2) = test_provider_register(&ctx, b"prov2", 0);
    assert_eq!(
        reg2.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );
    assert_eq!(
        ctx.get_account_balance(&prov2.as_icrc_compatible_account().into()),
        9500000000u64
    );

    ctx.upgrade().expect("Canister upgrade failed");
    assert_eq!(
        ctx.get_account_balance(&prov2.as_icrc_compatible_account().into()),
        9500000000u64
    );

    assert_eq!(
        ctx.get_account_balance(&prov1.as_icrc_compatible_account().into()),
        0u64
    );

    ctx.commit();
    // check in prov2
    assert_eq!(
        test_provider_check_in(&ctx, &prov2).unwrap(),
        "Signature verified, check in successful. You have been charged 0.500000000 DC tokens"
            .to_string()
    );
    ctx.ffwd_to_next_block(ts_ns);
    // Now prov2 got a reward of 50 tokens distributed to it
    // The balance is 50 (reward) + 10 (prov_past transfer) - 0.5 (reg fee) - 0.5 (check in) = 59000000000 e9s
    assert_eq!(
        ctx.get_account_balance(&prov2.as_icrc_compatible_account().into()),
        59000000000u64
    );

    ctx.upgrade().expect("Canister upgrade failed");
    assert_eq!(
        ctx.get_account_balance(&prov2.as_icrc_compatible_account().into()),
        59000000000u64
    );

    assert_eq!(
        ctx.get_account_balance(&prov1.as_icrc_compatible_account().into()),
        0u64
    );

    // Registration itself does not affect the reputation.
    reward_e9s_per_block_recalculate();
    assert_eq!(test_get_id_reputation(&ctx, &prov1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &prov2), 0);
}

#[test]
fn test_reputation() {
    let ctx = TestContext::new();
    let ts_ns = ctx.get_timestamp_ns();

    let (_prov_past, _reg_result) =
        test_provider_register(&ctx, b"prov_past", 2 * DC_TOKEN_DECIMALS_DIV); // Return values intentionally ignored - this just adds a block
    ctx.ffwd_to_next_block(ts_ns);

    let (prov1, reg1) = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(
        reg1.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );
    let (prov2, reg2) = test_provider_register(&ctx, b"prov2", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(
        reg2.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );
    let (prov3, reg3) = test_provider_register(&ctx, b"prov3", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(
        reg3.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );

    let (u1, r_u1) = test_user_register(&ctx, b"u1", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(
        r_u1.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );
    let (u2, r_u2) = test_user_register(&ctx, b"u2", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(
        r_u2.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );

    ctx.ffwd_to_next_block(ts_ns);

    assert_eq!(test_get_id_reputation(&ctx, &prov1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &prov2), 0);
    assert_eq!(test_get_id_reputation(&ctx, &prov3), 0);

    assert_eq!(test_get_id_reputation(&ctx, &u1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &u2), 0);
}

// ---- Ledger Entries Tests ----

#[test]
fn test_ledger_entries_empty() {
    let ctx = TestContext::new();

    // Test with empty ledger (no committed entries)
    let result = test_ledger_entries(&ctx, None, None, None, None);

    assert_eq!(result.entries.len(), 0);
    assert!(!result.has_more);
    assert!(result.next_cursor.is_none());
}

#[test]
fn test_ledger_entries_with_committed_data() {
    let ctx = TestContext::new();

    // Register providers and commit
    let _ = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV); // Return values intentionally ignored - only checking pagination
    let _ = test_provider_register(&ctx, b"prov2", 2 * DC_TOKEN_DECIMALS_DIV); // Return values intentionally ignored - only checking pagination

    // Commit the block
    ctx.commit();

    // Query committed entries only (exclude_next_block = false or None)
    let result = test_ledger_entries(&ctx, None, None, None, Some(false));

    // Should have committed entries: at least 2 ProvRegister
    assert!(result.entries.len() >= 2);

    // Verify we have ProvRegister entries
    let prov_register_count = result
        .entries
        .iter()
        .filter(|e| e.label == "ProvRegister")
        .count();
    assert_eq!(prov_register_count, 2);

    let committed_len = result.entries.len();

    // Add a new provider (uncommitted)
    let _prov3 = test_provider_register(&ctx, b"prov3", 2 * DC_TOKEN_DECIMALS_DIV);

    // Query again without including next_block - should still have same count
    let result2 = test_ledger_entries(&ctx, None, None, None, Some(false));
    assert_eq!(result2.entries.len(), committed_len);
}

#[test]
fn test_ledger_entries_with_next_block_included() {
    let ctx = TestContext::new();

    // Register and commit
    let _prov1 = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);
    ctx.commit();

    // Add more without committing
    let _prov2 = test_provider_register(&ctx, b"prov2", 2 * DC_TOKEN_DECIMALS_DIV);
    let _user1 = test_user_register(&ctx, b"user1", 2 * DC_TOKEN_DECIMALS_DIV);

    // Query without next_block
    let result_committed = test_ledger_entries(&ctx, None, None, None, Some(false));
    let committed_len = result_committed.entries.len();
    assert!(committed_len >= 1); // At least the ProvRegister

    // Query with next_block included
    let result_all = test_ledger_entries(&ctx, None, None, None, Some(true));

    // Verify we have more entries with next_block included
    assert!(result_all.entries.len() > committed_len);

    // Should have at least 2 more entries (1 ProvRegister + 1 UserRegister in next_block)
    assert!(result_all.entries.len() >= committed_len + 2);
}

#[test]
fn test_ledger_entries_filter_by_label() {
    let ctx = TestContext::new();

    // Register providers and users, then commit
    let _prov1 = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);
    let _prov2 = test_provider_register(&ctx, b"prov2", 2 * DC_TOKEN_DECIMALS_DIV);
    let _user1 = test_user_register(&ctx, b"user1", 2 * DC_TOKEN_DECIMALS_DIV);
    ctx.commit();

    // Test filtering by ProvRegister label
    let result = test_ledger_entries(&ctx, Some("ProvRegister".to_string()), None, None, None);
    assert_eq!(result.entries.len(), 2);
    assert!(result.entries.iter().all(|e| e.label == "ProvRegister"));
    assert!(!result.has_more);

    // Test filtering by UserRegister label
    let result = test_ledger_entries(&ctx, Some("UserRegister".to_string()), None, None, None);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].label, "UserRegister");
    assert!(!result.has_more);

    // Test filtering by non-existent label
    let result = test_ledger_entries(&ctx, Some("NonExistent".to_string()), None, None, None);
    assert_eq!(result.entries.len(), 0);
    assert!(!result.has_more);
}

#[test]
fn test_ledger_entries_pagination() {
    let ctx = TestContext::new();

    // Register multiple providers to create enough entries
    for i in 0..5 {
        let seed = format!("prov{}", i);
        let _ = test_provider_register(&ctx, seed.as_bytes(), 2 * DC_TOKEN_DECIMALS_DIV); // Return value intentionally ignored - only checking pagination
    }
    ctx.commit();

    // Test cursor-based pagination with limit 2
    let result1 = test_ledger_entries(&ctx, None, None, Some(2), None);
    assert_eq!(result1.entries.len(), 2);
    assert!(result1.has_more);
    assert!(result1.next_cursor.is_some());

    // Get second page using the cursor from first page
    let result2 = test_ledger_entries(&ctx, None, result1.next_cursor.clone(), Some(2), None);
    assert!(result2.entries.len() <= 2);
    assert!(!result2.entries.is_empty()); // Should have at least some entries

    // Verify no overlap between pages
    let keys1: Vec<_> = result1.entries.iter().map(|e| &e.key).collect();
    let keys2: Vec<_> = result2.entries.iter().map(|e| &e.key).collect();
    for k2 in keys2 {
        assert!(!keys1.contains(&k2), "Found overlap in paginated results");
    }
}

#[test]
fn test_ledger_entries_pagination_with_filter() {
    let ctx = TestContext::new();

    // Register 10 providers
    for i in 0..10 {
        let seed = format!("prov{}", i);
        let _ = test_provider_register(&ctx, seed.as_bytes(), 2 * DC_TOKEN_DECIMALS_DIV); // Return value intentionally ignored - only checking pagination
    }
    ctx.commit();

    // Filter by ProvRegister and paginate with cursor
    let result1 = test_ledger_entries(&ctx, Some("ProvRegister".to_string()), None, Some(5), None);
    assert_eq!(result1.entries.len(), 5);
    assert!(result1.has_more);
    assert!(result1.next_cursor.is_some());
    assert!(result1.entries.iter().all(|e| e.label == "ProvRegister"));

    // Get second page using cursor
    let result2 = test_ledger_entries(
        &ctx,
        Some("ProvRegister".to_string()),
        result1.next_cursor.clone(),
        Some(5),
        None,
    );
    assert_eq!(result2.entries.len(), 5);
    assert!(!result2.has_more);
    assert!(result2.entries.iter().all(|e| e.label == "ProvRegister"));
}
