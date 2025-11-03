mod test_utils;
use crate::test_utils::{
    test_contract_sign_reply, test_contract_sign_request, test_contracts_list_pending,
    test_get_id_reputation, test_icrc1_account_from_slice, test_ledger_entries,
    test_next_block_entries, test_offering_add, test_offering_search, test_provider_check_in,
    test_provider_register, test_user_register, TestContext,
};
use borsh::BorshDeserialize;
use candid::{encode_one, Nat, Principal};
use dcc_common::{
    reward_e9s_per_block_recalculate, ContractSignReply, ContractSignRequestPayload, DccIdentity,
    TokenAmountE9s, DC_TOKEN_DECIMALS, DC_TOKEN_DECIMALS_DIV, DC_TOKEN_NAME, DC_TOKEN_SYMBOL,
    DC_TOKEN_TOTAL_SUPPLY, DC_TOKEN_TRANSFER_FEE_E9S, MINTING_ACCOUNT_ICRC1,
};
use decent_cloud_canister::canister_backend::icrc1::Icrc1StandardRecord;
use decent_cloud_canister::DC_TOKEN_LOGO;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use provider_offering::ServerOffering;

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
        test_provider_register(&ctx, b"prov_past", 2 * DC_TOKEN_DECIMALS_DIV); // ignored, added only to get 1 block
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

#[test]
fn test_offerings() {
    let ctx = TestContext::new();
    let ts_ns = ctx.get_timestamp_ns();

    let (_prov_past, _reg_result) =
        test_provider_register(&ctx, b"prov_past", 2 * DC_TOKEN_DECIMALS_DIV); // ignored, added only to get 1 block
    ctx.ffwd_to_next_block(ts_ns);

    let prov1 = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV).0;
    ctx.ffwd_to_next_block(ts_ns);

    assert_eq!(test_offering_search(&ctx, "").len(), 0);

    // Create a test offering
    let offering = ServerOffering {
        offer_name: "Test Small VPS".to_string(),
        description: "A small VPS for testing".to_string(),
        unique_internal_identifier: "xxx-small".to_string(),
        product_page_url: "https://example.com/xxx-small".to_string(),
        currency: provider_offering::Currency::USD,
        monthly_price: 2.0,
        setup_fee: 0.0,
        visibility: provider_offering::Visibility::Visible,
        product_type: provider_offering::ProductType::VPS,
        virtualization_type: Some(provider_offering::VirtualizationType::KVM),
        billing_interval: provider_offering::BillingInterval::Monthly,
        stock: provider_offering::StockStatus::InStock,
        processor_brand: Some("Intel".to_string()),
        processor_amount: Some(1),
        processor_cores: Some(1),
        processor_speed: Some("2.5 GHz".to_string()),
        processor_name: Some("Intel Xeon".to_string()),
        memory_error_correction: None,
        memory_type: Some("DDR4".to_string()),
        memory_amount: Some("512 MB".to_string()),
        hdd_amount: 0,
        total_hdd_capacity: None,
        ssd_amount: 1,
        total_ssd_capacity: Some("2 GB".to_string()),
        unmetered: vec![],
        uplink_speed: Some("1 Gbps".to_string()),
        traffic: Some(1000),
        datacenter_country: "US".to_string(),
        datacenter_city: "New York".to_string(),
        datacenter_coordinates: Some((40.7128, -74.0060)),
        features: vec!["SSD Storage".to_string(), "IPv6".to_string()],
        operating_systems: vec!["Ubuntu".to_string(), "CentOS".to_string()],
        control_panel: Some("cPanel".to_string()),
        gpu_name: None,
        payment_methods: vec!["Credit Card".to_string(), "PayPal".to_string()],
    };

    test_offering_add(&ctx, &prov1, &offering).unwrap();

    let search_results = test_offering_search(&ctx, "");
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0].provider_pubkey,
        prov1.to_bytes_verifying()
    );
    assert_eq!(
        search_results[0].server_offerings[0].offer_name,
        offering.offer_name
    );
    assert_eq!(
        search_results[0].server_offerings[0].unique_internal_identifier,
        offering.unique_internal_identifier
    );

    ctx.ffwd_to_next_block(ts_ns);
    let search_results = test_offering_search(&ctx, "");
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0].provider_pubkey,
        prov1.to_bytes_verifying()
    );
    assert_eq!(
        search_results[0].server_offerings[0].offer_name,
        offering.offer_name
    );
    assert_eq!(
        search_results[0].server_offerings[0].unique_internal_identifier,
        offering.unique_internal_identifier
    );

    let search_results = test_offering_search(&ctx, "512 MB");
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0].provider_pubkey,
        prov1.to_bytes_verifying()
    );
    assert_eq!(
        search_results[0].server_offerings[0].offer_name,
        offering.offer_name
    );
    assert_eq!(
        search_results[0].server_offerings[0].unique_internal_identifier,
        offering.unique_internal_identifier
    );

    let search_results = test_offering_search(&ctx, "1GB");
    assert_eq!(search_results.len(), 0);

    // Test for contract signing
    let offering_id = offering.get_unique_instance_id().clone();
    assert_eq!(offering_id, "xxx-small");

    let u1 = test_user_register(&ctx, b"u1", 2 * DC_TOKEN_DECIMALS_DIV).0;

    assert_eq!(test_get_id_reputation(&ctx, &u1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &prov1), 0);

    // Test the rejection of a contract signing
    contract_req_sign_flow(&ctx, &prov1, &u1, &offering_id, "memo1".to_owned(), false);

    // Test the acceptance of a contract signing
    contract_req_sign_flow(&ctx, &prov1, &u1, &offering_id, "memo2".to_owned(), true);
    let prov1_rep = test_get_id_reputation(&ctx, &prov1);
    let u1_rep = test_get_id_reputation(&ctx, &u1);

    let pending_contracts = test_contracts_list_pending(&ctx, None);
    assert_eq!(pending_contracts.len(), 0);
    let pending_contracts = test_contracts_list_pending(&ctx, Some(prov1.to_bytes_verifying()));
    assert_eq!(pending_contracts.len(), 0);
    ctx.ffwd_to_next_block(ts_ns);

    let pending_contracts = test_contracts_list_pending(&ctx, None);
    assert_eq!(pending_contracts.len(), 0);
    let pending_contracts = test_contracts_list_pending(&ctx, Some(prov1.to_bytes_verifying()));
    assert_eq!(pending_contracts.len(), 0);

    assert_eq!(test_get_id_reputation(&ctx, &prov1), prov1_rep);
    assert_eq!(test_get_id_reputation(&ctx, &u1), u1_rep);
}

fn contract_req_sign_flow(
    ctx: &TestContext,
    prov1: &DccIdentity,
    u1: &DccIdentity,
    offering_id: &str,
    memo: String,
    accept: bool,
) {
    if accept {
        println!("Testing an accept of a contract signing");
    } else {
        println!("Testing a rejection of a contract signing");
    }
    let prov1_balance_before = ctx.get_account_balance(&prov1.as_icrc_compatible_account().into());
    let prov1_rep_before = test_get_id_reputation(ctx, prov1);
    let u1_balance_before = ctx.get_account_balance(&u1.as_icrc_compatible_account().into());
    let u1_rep_before = test_get_id_reputation(ctx, u1);

    let contract_amount: TokenAmountE9s = 1_000_000_000;
    let contract_step_fee = contract_amount / 100; // 1% fee
    test_contract_sign_request(
        ctx,
        u1,
        &prov1.to_bytes_verifying(),
        offering_id,
        memo,
        contract_amount,
    )
    .unwrap();

    assert_eq!(
        ctx.get_account_balance(&u1.as_icrc_compatible_account().into()),
        u1_balance_before.clone() - contract_step_fee
    );
    assert_eq!(
        ctx.get_account_balance(&prov1.as_icrc_compatible_account().into()),
        prov1_balance_before
    );
    assert_eq!(test_get_id_reputation(ctx, prov1), prov1_rep_before);
    assert_eq!(
        test_get_id_reputation(ctx, u1),
        u1_rep_before + contract_step_fee
    );

    let pending_contracts = test_contracts_list_pending(ctx, None);
    assert_eq!(pending_contracts.len(), 1);

    let pending_contracts = test_contracts_list_pending(ctx, Some(prov1.to_bytes_verifying()));
    assert_eq!(pending_contracts.len(), 1);

    let (contract_id, contract_req_bytes) = pending_contracts[0].clone();

    // Verify that the returned contract ID can be correctly recalculated
    let contract_req = ContractSignRequestPayload::try_from_slice(&contract_req_bytes).unwrap();
    assert_eq!(contract_id, contract_req.calc_contract_id());

    let reply = ContractSignReply::new(
        prov1.to_bytes_verifying(),
        "test_memo_wrong",
        contract_id,
        accept,
        "Thank you for signing up",
        "Here are some details",
    );
    let res = test_contract_sign_reply(ctx, prov1, u1, &reply).unwrap();
    assert_eq!(res, "Contract signing reply submitted! Thank you. You have been charged 0.010000000 DC tokens as a fee, and your reputation has been bumped accordingly");

    if accept {
        assert_eq!(
            ctx.get_account_balance(&u1.as_icrc_compatible_account().into()),
            u1_balance_before - 2 * contract_step_fee - contract_amount
        );
        assert_eq!(
            ctx.get_account_balance(&prov1.as_icrc_compatible_account().into()),
            prov1_balance_before + contract_amount
        );
        assert_eq!(
            test_get_id_reputation(ctx, prov1),
            prov1_rep_before + contract_step_fee
        );
        assert_eq!(
            test_get_id_reputation(ctx, u1),
            u1_rep_before + contract_step_fee
        );
    } else {
        assert_eq!(
            ctx.get_account_balance(&u1.as_icrc_compatible_account().into()),
            u1_balance_before - contract_step_fee
        );
        assert_eq!(
            ctx.get_account_balance(&prov1.as_icrc_compatible_account().into()),
            prov1_balance_before - contract_step_fee
        );
        assert_eq!(test_get_id_reputation(ctx, prov1), prov1_rep_before);
        assert_eq!(test_get_id_reputation(ctx, u1), u1_rep_before);
    }
}

#[test]
fn test_next_block_entries_empty() {
    let ctx = TestContext::new();

    // Test with empty next block (no entries)
    let result = test_next_block_entries(&ctx, None, None, None);

    assert_eq!(result.entries.len(), 0);
    assert_eq!(result.total_count, 0);
    assert!(!result.has_more);
}

#[test]
fn test_next_block_entries_with_single_provider_registration() {
    let ctx = TestContext::new();

    // Check empty state first
    let empty_result = test_next_block_entries(&ctx, None, None, None);
    assert_eq!(empty_result.entries.len(), 0);
    assert_eq!(empty_result.total_count, 0);

    // Register a provider
    let (provider, _reg_result) =
        test_provider_register(&ctx, b"test_prov", 2 * DC_TOKEN_DECIMALS_DIV);

    // Test next block entries
    let result = test_next_block_entries(&ctx, None, None, None);

    // Should have 2 entries: DCTokenTransfer and ProvRegister
    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.total_count, 2);
    assert!(!result.has_more);

    // Find entries by label
    let prov_reg_entry = result
        .entries
        .iter()
        .find(|e| e.label == "ProvRegister")
        .expect("Should find ProvRegister entry");

    let token_transfer_entry = result
        .entries
        .iter()
        .find(|e| e.label == "DCTokenTransfer")
        .expect("Should find DCTokenTransfer entry");

    // Verify ProvRegister entry
    assert_eq!(prov_reg_entry.key, provider.to_bytes_verifying());
    assert!(!prov_reg_entry.value.is_empty());

    // Verify DCTokenTransfer entry
    assert!(!token_transfer_entry.value.is_empty());
}

#[test]
fn test_next_block_entries_with_multiple_entries() {
    let ctx = TestContext::new();
    let _ts_ns = ctx.get_timestamp_ns();

    // Register multiple providers
    let (prov1, _) = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);
    let (_prov2, _reg_result) = test_provider_register(&ctx, b"prov2", 2 * DC_TOKEN_DECIMALS_DIV);
    let (_user1, _reg_result) = test_user_register(&ctx, b"user1", 2 * DC_TOKEN_DECIMALS_DIV);

    // Check in a provider
    test_provider_check_in(&ctx, &prov1).unwrap();

    // Test next block entries
    let result = test_next_block_entries(&ctx, None, None, None);

    // Should have 7 entries: 3 DCTokenTransfer + 2 ProvRegister + 1 UserRegister + 1 ProvCheckIn
    assert_eq!(result.entries.len(), 7);
    assert_eq!(result.total_count, 7);
    assert!(!result.has_more);

    // Verify entries are in chronological order
    let labels: Vec<String> = result.entries.iter().map(|e| e.label.clone()).collect();
    assert!(labels.contains(&"ProvRegister".to_string()));
    assert!(labels.contains(&"UserRegister".to_string()));
    assert!(labels.contains(&"ProvCheckIn".to_string()));
    assert!(labels.contains(&"DCTokenTransfer".to_string()));

    // Should have exactly 2 ProvRegister entries
    let prov_reg_count = labels
        .iter()
        .filter(|l| l.as_str() == "ProvRegister")
        .count();
    assert_eq!(prov_reg_count, 2);

    // Should have exactly 3 DCTokenTransfer entries
    let token_transfer_count = labels
        .iter()
        .filter(|l| l.as_str() == "DCTokenTransfer")
        .count();
    assert_eq!(token_transfer_count, 3);
}

#[test]
fn test_next_block_entries_filter_by_label() {
    let ctx = TestContext::new();

    // Register providers and users
    let (_prov1, _reg_result) = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);
    let (_user1, _reg_result) = test_user_register(&ctx, b"user1", 2 * DC_TOKEN_DECIMALS_DIV);

    // Test filtering by ProvRegister label
    let result = test_next_block_entries(&ctx, Some("ProvRegister".to_string()), None, None);

    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.total_count, 1);
    assert!(!result.has_more);

    let entry = &result.entries[0];
    assert_eq!(entry.label, "ProvRegister");

    // Test filtering by UserRegister label
    let result = test_next_block_entries(&ctx, Some("UserRegister".to_string()), None, None);

    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.total_count, 1);
    assert!(!result.has_more);

    let entry = &result.entries[0];
    assert_eq!(entry.label, "UserRegister");

    // Test filtering by non-existent label
    let result = test_next_block_entries(&ctx, Some("NonExistent".to_string()), None, None);

    assert_eq!(result.entries.len(), 0);
    assert_eq!(result.total_count, 0);
    assert!(!result.has_more);
}

#[test]
fn test_next_block_entries_pagination() {
    let ctx = TestContext::new();

    // Register multiple providers to create enough entries for pagination testing
    for i in 0..5 {
        let seed = format!("prov{}", i);
        let (_provider, _reg_result) =
            test_provider_register(&ctx, seed.as_bytes(), 2 * DC_TOKEN_DECIMALS_DIV);
    }

    // Test pagination with limit 2
    let result1 = test_next_block_entries(&ctx, None, Some(0), Some(2));
    assert_eq!(result1.entries.len(), 2);
    assert_eq!(result1.total_count, 10); // 5 ProvRegister + 5 DCTokenTransfer
    assert!(result1.has_more);

    // Get second page
    let result2 = test_next_block_entries(&ctx, None, Some(2), Some(2));
    assert_eq!(result2.entries.len(), 2);
    assert_eq!(result2.total_count, 10);
    assert!(result2.has_more);

    // Get third page
    let result3 = test_next_block_entries(&ctx, None, Some(4), Some(2));
    assert_eq!(result3.entries.len(), 2);
    assert_eq!(result3.total_count, 10);
    assert!(result3.has_more);

    // Get fourth page
    let result4 = test_next_block_entries(&ctx, None, Some(6), Some(2));
    assert_eq!(result4.entries.len(), 2);
    assert_eq!(result4.total_count, 10);
    assert!(result4.has_more);

    // Get fifth page
    let result5 = test_next_block_entries(&ctx, None, Some(8), Some(2));
    assert_eq!(result5.entries.len(), 2);
    assert_eq!(result5.total_count, 10);
    assert!(!result5.has_more);

    // Verify no overlap between pages
    let page1_keys: Vec<Vec<u8>> = result1.entries.iter().map(|e| e.key.clone()).collect();
    let page2_keys: Vec<Vec<u8>> = result2.entries.iter().map(|e| e.key.clone()).collect();
    let page3_keys: Vec<Vec<u8>> = result3.entries.iter().map(|e| e.key.clone()).collect();
    let page4_keys: Vec<Vec<u8>> = result4.entries.iter().map(|e| e.key.clone()).collect();
    let page5_keys: Vec<Vec<u8>> = result5.entries.iter().map(|e| e.key.clone()).collect();

    assert_eq!(page1_keys.len(), 2);
    assert_eq!(page2_keys.len(), 2);
    assert_eq!(page3_keys.len(), 2);
    assert_eq!(page4_keys.len(), 2);
    assert_eq!(page5_keys.len(), 2);

    // Ensure no duplicate keys across pages
    let all_keys = page1_keys
        .iter()
        .chain(page2_keys.iter())
        .chain(page3_keys.iter())
        .chain(page4_keys.iter())
        .chain(page5_keys.iter());
    let unique_keys: std::collections::HashSet<_> = all_keys.cloned().collect();
    assert_eq!(unique_keys.len(), 10);
}

#[test]
fn test_next_block_entries_pagination_edge_cases() {
    let ctx = TestContext::new();

    // Test with offset beyond available entries
    let result = test_next_block_entries(&ctx, None, Some(100), Some(10));
    assert_eq!(result.entries.len(), 0);
    assert_eq!(result.total_count, 0);
    assert!(!result.has_more);

    // Register one provider
    let (_provider, _reg_result) =
        test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);

    // Test with offset exactly at the end
    let result = test_next_block_entries(&ctx, None, Some(2), Some(10));
    assert_eq!(result.entries.len(), 0);
    assert_eq!(result.total_count, 2);
    assert!(!result.has_more);

    // Test with offset beyond the end
    let result = test_next_block_entries(&ctx, None, Some(10), Some(10));
    assert_eq!(result.entries.len(), 0);
    assert_eq!(result.total_count, 2);
    assert!(!result.has_more);
}

#[test]
fn test_next_block_entries_after_commit() {
    let ctx = TestContext::new();

    // Register a provider
    let (_provider, _reg_result) =
        test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);

    // Verify entry is in next block
    let result = test_next_block_entries(&ctx, None, None, None);
    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.total_count, 2);

    // Commit the block
    ctx.commit();

    // Verify next block is now empty after commit
    let result = test_next_block_entries(&ctx, None, None, None);
    assert_eq!(result.entries.len(), 0);
    assert_eq!(result.total_count, 0);
    assert!(!result.has_more);
}

#[test]
fn test_next_block_entries_with_large_dataset() {
    let ctx = TestContext::new();

    // Create a larger dataset (15 entries) to test pagination
    for i in 0..15 {
        let seed = format!("prov{}", i);
        let (_provider, _reg_result) =
            test_provider_register(&ctx, seed.as_bytes(), 2 * DC_TOKEN_DECIMALS_DIV);
    }

    // Test with default limit (100)
    let result = test_next_block_entries(&ctx, None, None, None);
    assert_eq!(result.entries.len(), 30); // 15 ProvRegister + 15 DCTokenTransfer
    assert_eq!(result.total_count, 30);
    assert!(!result.has_more);

    // Test with custom limit (5)
    let mut total_retrieved = 0;
    let mut offset = 0;
    let limit = 5;

    loop {
        let result = test_next_block_entries(&ctx, None, Some(offset), Some(limit));
        total_retrieved += result.entries.len();

        if !result.has_more {
            break;
        }

        offset += limit;

        // Safety check to prevent infinite loop
        assert!(offset < 100, "Pagination loop detected");
    }

    assert_eq!(total_retrieved, 30);
    assert_eq!(total_retrieved, result.total_count as usize);
}

// ---- Ledger Entries Tests ----

#[test]
fn test_ledger_entries_empty() {
    let ctx = TestContext::new();

    // Test with empty ledger (no committed entries)
    let result = test_ledger_entries(&ctx, None, None, None, None);

    assert_eq!(result.entries.len(), 0);
    assert_eq!(result.total_count, 0);
    assert!(!result.has_more);
}

#[test]
fn test_ledger_entries_with_committed_data() {
    let ctx = TestContext::new();

    // Register providers and commit
    let (prov1, _) = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);
    let (prov2, _) = test_provider_register(&ctx, b"prov2", 2 * DC_TOKEN_DECIMALS_DIV);

    // Commit the block
    ctx.commit();

    // Query committed entries only (exclude_next_block = false or None)
    let result = test_ledger_entries(&ctx, None, None, None, Some(false));

    // Should have committed entries: at least 2 ProvRegister
    assert!(result.entries.len() >= 2);
    assert!(result.total_count >= 2);

    // Verify we have ProvRegister entries
    let prov_register_count = result
        .entries
        .iter()
        .filter(|e| e.label == "ProvRegister")
        .count();
    assert_eq!(prov_register_count, 2);

    let committed_count = result.total_count;

    // Add a new provider (uncommitted)
    let _prov3 = test_provider_register(&ctx, b"prov3", 2 * DC_TOKEN_DECIMALS_DIV);

    // Query again without including next_block - should still have same count
    let result2 = test_ledger_entries(&ctx, None, None, None, Some(false));
    assert_eq!(result2.total_count, committed_count);
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
    let committed_count = result_committed.total_count;
    assert!(committed_count >= 1); // At least the ProvRegister

    // Query with next_block included
    let result_all = test_ledger_entries(&ctx, None, None, None, Some(true));

    // Verify we have more entries with next_block included
    assert!(result_all.total_count > result_committed.total_count);

    // Should have at least 2 more entries (1 ProvRegister + 1 UserRegister in next_block)
    assert!(result_all.total_count >= committed_count + 2);
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
    assert_eq!(result.total_count, 2);
    assert!(result.entries.iter().all(|e| e.label == "ProvRegister"));

    // Test filtering by UserRegister label
    let result = test_ledger_entries(&ctx, Some("UserRegister".to_string()), None, None, None);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.total_count, 1);
    assert_eq!(result.entries[0].label, "UserRegister");

    // Test filtering by non-existent label
    let result = test_ledger_entries(&ctx, Some("NonExistent".to_string()), None, None, None);
    assert_eq!(result.entries.len(), 0);
    assert_eq!(result.total_count, 0);
}

#[test]
fn test_ledger_entries_pagination() {
    let ctx = TestContext::new();

    // Register multiple providers to create enough entries
    for i in 0..5 {
        let seed = format!("prov{}", i);
        test_provider_register(&ctx, seed.as_bytes(), 2 * DC_TOKEN_DECIMALS_DIV);
    }
    ctx.commit();

    // Get total count first
    let result_all = test_ledger_entries(&ctx, None, None, None, None);
    let total_entries = result_all.total_count;
    assert!(total_entries >= 5); // At least 5 ProvRegister entries

    // Test pagination with limit 2
    let result1 = test_ledger_entries(&ctx, None, Some(0), Some(2), None);
    assert_eq!(result1.entries.len(), 2);
    assert_eq!(result1.total_count, total_entries);
    assert_eq!(result1.has_more, total_entries > 2);

    if total_entries > 2 {
        // Get second page
        let result2 = test_ledger_entries(&ctx, None, Some(2), Some(2), None);
        assert!(result2.entries.len() <= 2);
        assert_eq!(result2.total_count, total_entries);

        // Verify no overlap
        let keys1: Vec<_> = result1.entries.iter().map(|e| &e.key).collect();
        let keys2: Vec<_> = result2.entries.iter().map(|e| &e.key).collect();
        for k2 in keys2 {
            assert!(!keys1.contains(&k2), "Found overlap in paginated results");
        }
    }
}

#[test]
fn test_ledger_entries_pagination_with_filter() {
    let ctx = TestContext::new();

    // Register 10 providers
    for i in 0..10 {
        let seed = format!("prov{}", i);
        test_provider_register(&ctx, seed.as_bytes(), 2 * DC_TOKEN_DECIMALS_DIV);
    }
    ctx.commit();

    // Filter by ProvRegister and paginate
    let result1 = test_ledger_entries(
        &ctx,
        Some("ProvRegister".to_string()),
        Some(0),
        Some(5),
        None,
    );
    assert_eq!(result1.entries.len(), 5);
    assert_eq!(result1.total_count, 10);
    assert!(result1.has_more);
    assert!(result1.entries.iter().all(|e| e.label == "ProvRegister"));

    let result2 = test_ledger_entries(
        &ctx,
        Some("ProvRegister".to_string()),
        Some(5),
        Some(5),
        None,
    );
    assert_eq!(result2.entries.len(), 5);
    assert_eq!(result2.total_count, 10);
    assert!(!result2.has_more);
    assert!(result2.entries.iter().all(|e| e.label == "ProvRegister"));
}

#[test]
fn test_ledger_entries_comparison_with_next_block_entries() {
    let ctx = TestContext::new();

    // Add some committed data
    let _prov1 = test_provider_register(&ctx, b"prov1", 2 * DC_TOKEN_DECIMALS_DIV);
    ctx.commit();

    // Add some uncommitted data
    let _prov2 = test_provider_register(&ctx, b"prov2", 2 * DC_TOKEN_DECIMALS_DIV);

    // ledger_entries with include_next_block=false should match committed data
    let ledger_committed = test_ledger_entries(&ctx, None, None, None, Some(false));
    let committed_count = ledger_committed.total_count;

    // next_block_entries should only show uncommitted
    let next_block = test_next_block_entries(&ctx, None, None, None);
    let next_block_count = next_block.total_count;

    // ledger_entries with include_next_block=true should be sum of both
    let ledger_all = test_ledger_entries(&ctx, None, None, None, Some(true));

    assert!(committed_count >= 1); // At least ProvRegister
    assert!(next_block_count >= 1); // At least ProvRegister in next_block
    assert_eq!(ledger_all.total_count, committed_count + next_block_count);
}
