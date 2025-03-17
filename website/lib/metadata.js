export const idlFactory = ({ IDL }) => {
  const GetBlocksResult = IDL.Rec();
  const ICRC3Value = IDL.Rec();
  const Value = IDL.Rec();
  const ResultString = IDL.Variant({ Ok: IDL.Text, Err: IDL.Text });
  const ResultData = IDL.Variant({
    Ok: IDL.Tuple(IDL.Text, IDL.Vec(IDL.Nat8)),
    Err: IDL.Text,
  });
  const BlockIndex = IDL.Nat;
  const GetBlocksArgs = IDL.Record({
    start: BlockIndex,
    length: IDL.Nat,
  });
  ICRC3Value.fill(
    IDL.Variant({
      Int: IDL.Int,
      Map: IDL.Vec(IDL.Tuple(IDL.Text, ICRC3Value)),
      Nat: IDL.Nat,
      Blob: IDL.Vec(IDL.Nat8),
      Text: IDL.Text,
      Array: IDL.Vec(ICRC3Value),
    })
  );
  GetBlocksResult.fill(
    IDL.Record({
      log_length: IDL.Nat,
      blocks: IDL.Vec(IDL.Record({ id: IDL.Nat, block: ICRC3Value })),
      archived_blocks: IDL.Vec(
        IDL.Record({
          args: IDL.Vec(GetBlocksArgs),
          callback: IDL.Func(
            [IDL.Vec(GetBlocksArgs)],
            [GetBlocksResult],
            ["query"]
          ),
        })
      ),
    })
  );
  const DataCertificate = IDL.Record({
    certificate: IDL.Opt(IDL.Vec(IDL.Nat8)),
    hash_tree: IDL.Vec(IDL.Nat8),
  });
  const TxIndex = IDL.Nat;
  const GetTransactionsRequest = IDL.Record({
    start: TxIndex,
    length: IDL.Nat,
  });
  const Subaccount = IDL.Vec(IDL.Nat8);
  const Account = IDL.Record({
    owner: IDL.Principal,
    subaccount: IDL.Opt(Subaccount),
  });
  const Timestamp = IDL.Nat64;
  const Burn = IDL.Record({
    from: Account,
    memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
    created_at_time: IDL.Opt(Timestamp),
    amount: IDL.Nat,
    spender: IDL.Opt(Account),
  });
  const Mint = IDL.Record({
    to: Account,
    memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
    created_at_time: IDL.Opt(Timestamp),
    amount: IDL.Nat,
  });
  const Approve = IDL.Record({
    fee: IDL.Opt(IDL.Nat),
    from: Account,
    memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
    created_at_time: IDL.Opt(Timestamp),
    amount: IDL.Nat,
    expected_allowance: IDL.Opt(IDL.Nat),
    expires_at: IDL.Opt(Timestamp),
    spender: Account,
  });
  const Transfer = IDL.Record({
    to: Account,
    fee: IDL.Opt(IDL.Nat),
    from: Account,
    memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
    created_at_time: IDL.Opt(Timestamp),
    amount: IDL.Nat,
    spender: IDL.Opt(Account),
  });
  const Transaction = IDL.Record({
    burn: IDL.Opt(Burn),
    kind: IDL.Text,
    mint: IDL.Opt(Mint),
    approve: IDL.Opt(Approve),
    timestamp: Timestamp,
    transfer: IDL.Opt(Transfer),
  });
  const TransactionRange = IDL.Record({
    transactions: IDL.Vec(Transaction),
  });
  const QueryArchiveFn = IDL.Func(
    [GetTransactionsRequest],
    [TransactionRange],
    ["query"]
  );
  const GetTransactionsResponse = IDL.Record({
    first_index: TxIndex,
    log_length: IDL.Nat,
    transactions: IDL.Vec(Transaction),
    archived_transactions: IDL.Vec(
      IDL.Record({
        callback: QueryArchiveFn,
        start: TxIndex,
        length: IDL.Nat,
      })
    ),
  });
  const HeaderField = IDL.Tuple(IDL.Text, IDL.Text);
  const HttpRequest = IDL.Record({
    url: IDL.Text,
    method: IDL.Text,
    body: IDL.Vec(IDL.Nat8),
    headers: IDL.Vec(HeaderField),
    certificate_version: IDL.Opt(IDL.Nat16),
  });
  const HttpResponse = IDL.Record({
    body: IDL.Vec(IDL.Nat8),
    headers: IDL.Vec(HeaderField),
    upgrade: IDL.Opt(IDL.Bool),
    status_code: IDL.Nat16,
  });
  const MetadataValue = IDL.Variant({
    Int: IDL.Int,
    Nat: IDL.Nat,
    Blob: IDL.Vec(IDL.Nat8),
    Text: IDL.Text,
  });
  const Icrc1TransferArgs = IDL.Record({
    to: Account,
    fee: IDL.Opt(IDL.Nat),
    memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
    from_subaccount: IDL.Opt(Subaccount),
    created_at_time: IDL.Opt(Timestamp),
    amount: IDL.Nat,
  });
  const TransferError = IDL.Variant({
    GenericError: IDL.Record({
      message: IDL.Text,
      error_code: IDL.Nat,
    }),
    TemporarilyUnavailable: IDL.Null,
    BadBurn: IDL.Record({ min_burn_amount: IDL.Nat }),
    Duplicate: IDL.Record({ duplicate_of: IDL.Nat }),
    BadFee: IDL.Record({ expected_fee: IDL.Nat }),
    CreatedInFuture: IDL.Record({ ledger_time: Timestamp }),
    TooOld: IDL.Null,
    InsufficientFunds: IDL.Record({ balance: IDL.Nat }),
  });
  const AllowanceArgs = IDL.Record({
    account: Account,
    spender: Account,
  });
  const Allowance = IDL.Record({
    allowance: IDL.Nat,
    expires_at: IDL.Opt(Timestamp),
  });
  const ApproveArgs = IDL.Record({
    fee: IDL.Opt(IDL.Nat),
    memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
    from_subaccount: IDL.Opt(IDL.Vec(IDL.Nat8)),
    created_at_time: IDL.Opt(IDL.Nat64),
    amount: IDL.Nat,
    expected_allowance: IDL.Opt(IDL.Nat),
    expires_at: IDL.Opt(IDL.Nat64),
    spender: Account,
  });
  const ApproveError = IDL.Variant({
    GenericError: IDL.Record({
      message: IDL.Text,
      error_code: IDL.Nat,
    }),
    TemporarilyUnavailable: IDL.Null,
    Duplicate: IDL.Record({ duplicate_of: IDL.Nat }),
    BadFee: IDL.Record({ expected_fee: IDL.Nat }),
    AllowanceChanged: IDL.Record({ current_allowance: IDL.Nat }),
    CreatedInFuture: IDL.Record({ ledger_time: Timestamp }),
    TooOld: IDL.Null,
    Expired: IDL.Record({ ledger_time: Timestamp }),
    InsufficientFunds: IDL.Record({ balance: IDL.Nat }),
  });
  const TransferFromArgs = IDL.Record({
    to: Account,
    fee: IDL.Opt(IDL.Nat),
    spender_subaccount: IDL.Opt(IDL.Vec(IDL.Nat8)),
    from: Account,
    memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
    created_at_time: IDL.Opt(IDL.Nat64),
    amount: IDL.Nat,
  });
  const TransferFromError = IDL.Variant({
    GenericError: IDL.Record({
      message: IDL.Text,
      error_code: IDL.Nat,
    }),
    TemporarilyUnavailable: IDL.Null,
    InsufficientAllowance: IDL.Record({ allowance: IDL.Nat }),
    BadBurn: IDL.Record({ min_burn_amount: IDL.Nat }),
    Duplicate: IDL.Record({ duplicate_of: IDL.Nat }),
    BadFee: IDL.Record({ expected_fee: IDL.Nat }),
    CreatedInFuture: IDL.Record({ ledger_time: IDL.Nat64 }),
    TooOld: IDL.Null,
    InsufficientFunds: IDL.Record({ balance: IDL.Nat }),
  });
  const Icrc3GetArchivesArgs = IDL.Record({ from: IDL.Opt(IDL.Principal) });
  const Icrc3GetArchivesResult = IDL.Vec(
    IDL.Record({
      end: IDL.Nat,
      canister_id: IDL.Principal,
      start: IDL.Nat,
    })
  );
  Value.fill(
    IDL.Variant({
      Int: IDL.Int,
      Map: IDL.Vec(IDL.Tuple(IDL.Text, Value)),
      Nat: IDL.Nat,
      Nat64: IDL.Nat64,
      Blob: IDL.Vec(IDL.Nat8),
      Text: IDL.Text,
      Array: IDL.Vec(Value),
    })
  );
  const Block = Value;
  const BlockRange = IDL.Record({ blocks: IDL.Vec(Block) });
  const QueryBlockArchiveFn = IDL.Func(
    [GetBlocksArgs],
    [BlockRange],
    ["query"]
  );
  const Icrc3GetBlocksResponse = IDL.Record({
    certificate: IDL.Opt(IDL.Vec(IDL.Nat8)),
    first_index: BlockIndex,
    blocks: IDL.Vec(Block),
    chain_length: IDL.Nat64,
    archived_blocks: IDL.Vec(
      IDL.Record({
        callback: QueryBlockArchiveFn,
        start: BlockIndex,
        length: IDL.Nat,
      })
    ),
  });
  const Icrc3DataCertificate = IDL.Record({
    certificate: IDL.Vec(IDL.Nat8),
    hash_tree: IDL.Vec(IDL.Nat8),
  });
  const OfferingEntry = IDL.Record({
    offering_compressed: IDL.Vec(IDL.Nat8),
    np_pub_key: IDL.Vec(IDL.Nat8),
  });
  return IDL.Service({
    contract_sign_reply: IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8)],
      [ResultString],
      []
    ),
    contract_sign_request: IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8)],
      [ResultString],
      []
    ),
    contracts_list_pending: IDL.Func(
      [IDL.Opt(IDL.Vec(IDL.Nat8))],
      [IDL.Vec(IDL.Tuple(IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8)))],
      ["query"]
    ),
    data_fetch: IDL.Func(
      [IDL.Opt(IDL.Text), IDL.Opt(IDL.Vec(IDL.Nat8))],
      [ResultData],
      ["query"]
    ),
    data_push: IDL.Func([IDL.Text, IDL.Vec(IDL.Nat8)], [ResultString], []),
    data_push_auth: IDL.Func([], [ResultString], []),
    get_blocks: IDL.Func([GetBlocksArgs], [GetBlocksResult], ["query"]),
    get_check_in_nonce: IDL.Func([], [IDL.Vec(IDL.Nat8)], ["query"]),
    get_data_certificate: IDL.Func([], [DataCertificate], ["query"]),
    get_identity_reputation: IDL.Func(
      [IDL.Vec(IDL.Nat8)],
      [IDL.Nat64],
      ["query"]
    ),
    get_logs_debug: IDL.Func([], [ResultString], ["query"]),
    get_logs_error: IDL.Func([], [ResultString], ["query"]),
    get_logs_info: IDL.Func([], [ResultString], ["query"]),
    get_logs_warn: IDL.Func([], [ResultString], ["query"]),
    get_registration_fee: IDL.Func([], [IDL.Nat64], ["query"]),
    get_transactions: IDL.Func(
      [GetTransactionsRequest],
      [GetTransactionsResponse],
      ["query"]
    ),
    http_request: IDL.Func([HttpRequest], [HttpResponse], ["query"]),
    icrc1_balance_of: IDL.Func([Account], [IDL.Nat], ["query"]),
    icrc1_decimals: IDL.Func([], [IDL.Nat8], ["query"]),
    icrc1_fee: IDL.Func([], [IDL.Nat], ["query"]),
    icrc1_metadata: IDL.Func(
      [],
      [IDL.Vec(IDL.Tuple(IDL.Text, MetadataValue))],
      ["query"]
    ),
    icrc1_minting_account: IDL.Func([], [IDL.Opt(Account)], ["query"]),
    icrc1_name: IDL.Func([], [IDL.Text], ["query"]),
    icrc1_supported_standards: IDL.Func(
      [],
      [IDL.Vec(IDL.Record({ url: IDL.Text, name: IDL.Text }))],
      ["query"]
    ),
    icrc1_symbol: IDL.Func([], [IDL.Text], ["query"]),
    icrc1_total_supply: IDL.Func([], [IDL.Nat], ["query"]),
    icrc1_transfer: IDL.Func(
      [Icrc1TransferArgs],
      [IDL.Variant({ Ok: IDL.Nat, Err: TransferError })],
      []
    ),
    icrc2_allowance: IDL.Func([AllowanceArgs], [Allowance], ["query"]),
    icrc2_approve: IDL.Func(
      [ApproveArgs],
      [IDL.Variant({ Ok: IDL.Nat, Err: ApproveError })],
      []
    ),
    icrc2_transfer_from: IDL.Func(
      [TransferFromArgs],
      [IDL.Variant({ Ok: IDL.Nat, Err: TransferFromError })],
      []
    ),
    icrc3_get_archives: IDL.Func(
      [Icrc3GetArchivesArgs],
      [Icrc3GetArchivesResult],
      ["query"]
    ),
    icrc3_get_blocks: IDL.Func(
      [IDL.Vec(GetBlocksArgs)],
      [Icrc3GetBlocksResponse],
      ["query"]
    ),
    icrc3_get_tip_certificate: IDL.Func(
      [],
      [IDL.Opt(Icrc3DataCertificate)],
      ["query"]
    ),
    icrc3_supported_block_types: IDL.Func(
      [],
      [IDL.Vec(IDL.Record({ url: IDL.Text, block_type: IDL.Text }))],
      ["query"]
    ),
    metadata: IDL.Func(
      [],
      [IDL.Vec(IDL.Tuple(IDL.Text, MetadataValue))],
      ["query"]
    ),
    node_provider_check_in: IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Text, IDL.Vec(IDL.Nat8)],
      [ResultString],
      []
    ),
    node_provider_get_profile_by_principal: IDL.Func(
      [IDL.Principal],
      [IDL.Opt(IDL.Text)],
      ["query"]
    ),
    node_provider_get_profile_by_pubkey_bytes: IDL.Func(
      [IDL.Vec(IDL.Nat8)],
      [IDL.Opt(IDL.Text)],
      ["query"]
    ),
    node_provider_list_checked_in: IDL.Func([], [ResultString], ["query"]),
    node_provider_register: IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8)],
      [ResultString],
      []
    ),
    node_provider_update_offering: IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8)],
      [ResultString],
      []
    ),
    node_provider_update_profile: IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8)],
      [ResultString],
      []
    ),
    offering_search: IDL.Func([IDL.Text], [IDL.Vec(OfferingEntry)], ["query"]),
    user_register: IDL.Func(
      [IDL.Vec(IDL.Nat8), IDL.Vec(IDL.Nat8)],
      [ResultString],
      []
    ),
  });
};
export const init = () => {
  return [];
};
