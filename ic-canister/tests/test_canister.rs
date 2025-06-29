mod test_utils;
use crate::test_utils::{
    test_contract_sign_reply, test_contract_sign_request, test_contracts_list_pending,
    test_get_id_reputation, test_icrc1_account_from_slice, test_np_check_in, test_np_register,
    test_offering_add, test_offering_search, test_user_register, TestContext,
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
use np_offering::Offering;

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
fn test_np_registration_and_check_in() {
    let ctx = TestContext::new();
    let ts_ns = ctx.get_timestamp_ns();

    // Register one NP and commit one block, to make sure there is something in the ledger.
    let (np_past, _reg1) = test_np_register(&ctx, b"np_past", 0);
    assert_eq!(
        test_np_check_in(&ctx, &np_past).unwrap(),
        "Signature verified, check in successful. You have been charged 0.0 DC tokens".to_string()
    );
    ctx.commit();

    // np_past now has 50 * 100 = 5000 tokens
    let amount: TokenAmountE9s = 5000u32 as TokenAmountE9s * DC_TOKEN_DECIMALS_DIV;
    assert_eq!(
        ctx.get_account_balance(&np_past.as_icrc_compatible_account().into()),
        amount
    );

    // Since the ledger is not empty, NP registration requires a payment of the registration fee
    let (np1, reg1) = test_np_register(&ctx, b"np1", 0);
    assert_eq!(reg1.unwrap_err(), "InsufficientFunds: account w7shl-xsw5s-kduqo-kx77s-nxs35-4zdh3-3tpob-nr4yc-2c6zw-qeyzj-rqe has 0 e9s (0.0 DC tokens) and requested 500000000 e9s (0.500000000 DC tokens)".to_string());
    assert_eq!(
        ctx.get_account_balance(&np1.as_icrc_compatible_account().into()),
        0u64
    );

    let (np2, reg2) = test_np_register(&ctx, b"np2", 0);
    assert_eq!(reg2.unwrap_err(), "InsufficientFunds: account ejigd-cloes-e7n46-7uop4-cwkfh-ccuxk-ry2cf-adfeg-3ik3k-znob6-pae has 0 e9s (0.0 DC tokens) and requested 500000000 e9s (0.500000000 DC tokens)".to_string());
    ctx.commit();

    // Initial reputation is 0
    assert_eq!(test_get_id_reputation(&ctx, &np1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &np2), 0);

    let np_past_acct = np_past.as_icrc_compatible_account().into();
    let np2_acct = np2.as_icrc_compatible_account().into();
    let amount_send = 10 * DC_TOKEN_DECIMALS_DIV;
    let response = ctx.transfer_funds(&np_past_acct, &np2_acct, amount_send);

    assert!(response.is_ok());

    assert_eq!(
        ctx.get_account_balance(&np_past.as_icrc_compatible_account().into()),
        amount - amount_send - DC_TOKEN_TRANSFER_FEE_E9S
    );
    assert_eq!(
        ctx.get_account_balance(&np2.as_icrc_compatible_account().into()),
        amount_send
    );

    // Now np1 still can't register
    let (np1, reg1) = test_np_register(&ctx, b"np1", 0);
    assert_eq!(reg1.unwrap_err(), "InsufficientFunds: account w7shl-xsw5s-kduqo-kx77s-nxs35-4zdh3-3tpob-nr4yc-2c6zw-qeyzj-rqe has 0 e9s (0.0 DC tokens) and requested 500000000 e9s (0.500000000 DC tokens)".to_string());
    assert_eq!(
        ctx.get_account_balance(&np1.as_icrc_compatible_account().into()),
        0u64
    );

    // But np2 can, since it has enough funds
    let (np2, reg2) = test_np_register(&ctx, b"np2", 0);
    assert_eq!(
        reg2.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );
    assert_eq!(
        ctx.get_account_balance(&np2.as_icrc_compatible_account().into()),
        9500000000u64
    );

    ctx.upgrade().expect("Canister upgrade failed");
    assert_eq!(
        ctx.get_account_balance(&np2.as_icrc_compatible_account().into()),
        9500000000u64
    );

    assert_eq!(
        ctx.get_account_balance(&np1.as_icrc_compatible_account().into()),
        0u64
    );

    ctx.commit();
    // check in np2
    assert_eq!(
        test_np_check_in(&ctx, &np2).unwrap(),
        "Signature verified, check in successful. You have been charged 0.500000000 DC tokens"
            .to_string()
    );
    ctx.ffwd_to_next_block(ts_ns);
    // Now np2 got a reward of 50 tokens distributed to it
    // The balance is 50 (reward) + 10 (np_past transfer) - 0.5 (reg fee) - 0.5 (check in) = 59000000000 e9s
    assert_eq!(
        ctx.get_account_balance(&np2.as_icrc_compatible_account().into()),
        59000000000u64
    );

    ctx.upgrade().expect("Canister upgrade failed");
    assert_eq!(
        ctx.get_account_balance(&np2.as_icrc_compatible_account().into()),
        59000000000u64
    );

    assert_eq!(
        ctx.get_account_balance(&np1.as_icrc_compatible_account().into()),
        0u64
    );

    // Registration itself does not affect the reputation.
    reward_e9s_per_block_recalculate();
    assert_eq!(test_get_id_reputation(&ctx, &np1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &np2), 0);
}

#[test]
fn test_reputation() {
    let ctx = TestContext::new();
    let ts_ns = ctx.get_timestamp_ns();

    let _ = test_np_register(&ctx, b"np_past", 2 * DC_TOKEN_DECIMALS_DIV); // ignored, added only to get 1 block
    ctx.ffwd_to_next_block(ts_ns);

    let (np1, reg1) = test_np_register(&ctx, b"np1", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(
        reg1.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );
    let (np2, reg2) = test_np_register(&ctx, b"np2", 2 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(
        reg2.unwrap(),
        "Registration complete! Thank you. You have been charged 0.500000000 DC tokens".to_string()
    );
    let (np3, reg3) = test_np_register(&ctx, b"np3", 2 * DC_TOKEN_DECIMALS_DIV);
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

    assert_eq!(test_get_id_reputation(&ctx, &np1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &np2), 0);
    assert_eq!(test_get_id_reputation(&ctx, &np3), 0);

    assert_eq!(test_get_id_reputation(&ctx, &u1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &u2), 0);
}

#[test]
fn test_offerings() {
    let ctx = TestContext::new();
    let ts_ns = ctx.get_timestamp_ns();

    let _ = test_np_register(&ctx, b"np_past", 2 * DC_TOKEN_DECIMALS_DIV); // ignored, added only to get 1 block
    ctx.ffwd_to_next_block(ts_ns);

    let np1 = test_np_register(&ctx, b"np1", 2 * DC_TOKEN_DECIMALS_DIV).0;
    ctx.ffwd_to_next_block(ts_ns);

    assert_eq!(test_offering_search(&ctx, "").len(), 0);
    let offering = Offering::new_from_file("tests/data/np-offering-demo1.yaml").unwrap();
    test_offering_add(&ctx, &np1, &offering).unwrap();

    let search_results = test_offering_search(&ctx, "");
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0].0.to_bytes_verifying(),
        np1.to_bytes_verifying()
    );
    assert_eq!(
        search_results[0].1.as_json_string(),
        offering.as_json_string()
    );

    ctx.ffwd_to_next_block(ts_ns);
    let search_results = test_offering_search(&ctx, "");
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0].0.to_bytes_verifying(),
        np1.to_bytes_verifying()
    );
    assert_eq!(
        search_results[0].1.as_json_string(),
        offering.as_json_string()
    );

    let search_results = test_offering_search(&ctx, "memory >= 512MB");
    assert_eq!(search_results.len(), 1);
    assert_eq!(
        search_results[0].0.to_bytes_verifying(),
        np1.to_bytes_verifying()
    );
    assert_eq!(
        search_results[0].1.as_json_string(),
        offering.as_json_string()
    );

    let search_results = test_offering_search(&ctx, "memory < 512MB");
    assert_eq!(search_results.len(), 0);

    // Test for contract signing
    let offering_id = offering.matches_search("memory >= 512MB")[0].clone();
    assert_eq!(offering_id, "xxx-small");

    let u1 = test_user_register(&ctx, b"u1", 2 * DC_TOKEN_DECIMALS_DIV).0;

    assert_eq!(test_get_id_reputation(&ctx, &u1), 0);
    assert_eq!(test_get_id_reputation(&ctx, &np1), 0);

    // Test the rejection of a contract signing
    contract_req_sign_flow(&ctx, &np1, &u1, &offering_id, "memo1".to_owned(), false);

    // Test the acceptance of a contract signing
    contract_req_sign_flow(&ctx, &np1, &u1, &offering_id, "memo2".to_owned(), true);
    let np1_rep = test_get_id_reputation(&ctx, &np1);
    let u1_rep = test_get_id_reputation(&ctx, &u1);

    let pending_contracts = test_contracts_list_pending(&ctx, None);
    assert_eq!(pending_contracts.len(), 0);
    let pending_contracts = test_contracts_list_pending(&ctx, Some(np1.to_bytes_verifying()));
    assert_eq!(pending_contracts.len(), 0);
    ctx.ffwd_to_next_block(ts_ns);

    let pending_contracts = test_contracts_list_pending(&ctx, None);
    assert_eq!(pending_contracts.len(), 0);
    let pending_contracts = test_contracts_list_pending(&ctx, Some(np1.to_bytes_verifying()));
    assert_eq!(pending_contracts.len(), 0);

    assert_eq!(test_get_id_reputation(&ctx, &np1), np1_rep);
    assert_eq!(test_get_id_reputation(&ctx, &u1), u1_rep);
}

fn contract_req_sign_flow(
    ctx: &TestContext,
    np1: &DccIdentity,
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
    let np1_balance_before = ctx.get_account_balance(&np1.as_icrc_compatible_account().into());
    let np1_rep_before = test_get_id_reputation(ctx, np1);
    let u1_balance_before = ctx.get_account_balance(&u1.as_icrc_compatible_account().into());
    let u1_rep_before = test_get_id_reputation(ctx, u1);

    let contract_amount: TokenAmountE9s = 1_000_000_000;
    let contract_step_fee = contract_amount / 100; // 1% fee
    test_contract_sign_request(
        ctx,
        u1,
        &np1.to_bytes_verifying(),
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
        ctx.get_account_balance(&np1.as_icrc_compatible_account().into()),
        np1_balance_before
    );
    assert_eq!(test_get_id_reputation(ctx, np1), np1_rep_before);
    assert_eq!(
        test_get_id_reputation(ctx, u1),
        u1_rep_before + contract_step_fee
    );

    let pending_contracts = test_contracts_list_pending(ctx, None);
    assert_eq!(pending_contracts.len(), 1);

    let pending_contracts = test_contracts_list_pending(ctx, Some(np1.to_bytes_verifying()));
    assert_eq!(pending_contracts.len(), 1);

    let (contract_id, contract_req_bytes) = pending_contracts[0].clone();

    // Verify that the returned contract ID can be correctly recalculated
    let contract_req = ContractSignRequestPayload::try_from_slice(&contract_req_bytes).unwrap();
    assert_eq!(contract_id, contract_req.calc_contract_id());

    let reply = ContractSignReply::new(
        np1.to_bytes_verifying(),
        "test_memo_wrong",
        contract_id,
        accept,
        "Thank you for signing up",
        "Here are some details",
    );
    let res = test_contract_sign_reply(ctx, np1, u1, &reply).unwrap();
    assert_eq!(res, "Contract signing reply submitted! Thank you. You have been charged 0.010000000 DC tokens as a fee, and your reputation has been bumped accordingly");

    if accept {
        assert_eq!(
            ctx.get_account_balance(&u1.as_icrc_compatible_account().into()),
            u1_balance_before - 2 * contract_step_fee - contract_amount
        );
        assert_eq!(
            ctx.get_account_balance(&np1.as_icrc_compatible_account().into()),
            np1_balance_before + contract_amount
        );
        assert_eq!(
            test_get_id_reputation(ctx, np1),
            np1_rep_before + contract_step_fee
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
            ctx.get_account_balance(&np1.as_icrc_compatible_account().into()),
            np1_balance_before - contract_step_fee
        );
        assert_eq!(test_get_id_reputation(ctx, np1), np1_rep_before);
        assert_eq!(test_get_id_reputation(ctx, u1), u1_rep_before);
    }
}
