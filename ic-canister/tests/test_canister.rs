mod test_utils;
use crate::test_utils::{
    test_contract_sign_reply, test_contract_sign_request, test_contracts_list_pending,
    test_get_id_reputation, test_icrc1_account_from_slice, test_offering_add, test_offering_search,
    test_provider_check_in, test_provider_register, test_user_register, TestContext,
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

    let _ = test_provider_register(&ctx, b"prov_past", 2 * DC_TOKEN_DECIMALS_DIV); // ignored, added only to get 1 block
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

    let _ = test_provider_register(&ctx, b"prov_past", 2 * DC_TOKEN_DECIMALS_DIV); // ignored, added only to get 1 block
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
