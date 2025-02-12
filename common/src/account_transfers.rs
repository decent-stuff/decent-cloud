use crate::{
    account_balance_add, account_balance_get, account_balance_sub, amount_as_string,
    get_pubkey_from_principal, get_timestamp_ns, ledger_add_reputation_change,
    slice_to_32_bytes_array, AHashMap, DccIdentity, RecentCache, TokenAmountE9s, TransferError,
    DC_TOKEN_TRANSFER_FEE_E9S, LABEL_DC_TOKEN_TRANSFER, MINTING_ACCOUNT, MINTING_ACCOUNT_PRINCIPAL,
    TX_WINDOW,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use candid::{CandidType, Deserialize, Nat, Principal};
use data_encoding::BASE32;
#[cfg(target_arch = "wasm32")]
use ic_cdk::println;
use icrc_ledger_types::{
    icrc1::account::Account as Icrc1Account,
    icrc3::transactions::{Burn, Mint, Transaction, Transfer},
};
use ledger_map::{info, LedgerMap};
use once_cell::sync::OnceCell;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct RecentTx {
    tx_num: u64,
    timestamp: u64,
}

static FEES_SINK_ACCOUNTS: OnceCell<Arc<Mutex<Vec<IcrcCompatibleAccount>>>> = OnceCell::new();
static RECENT_TRANSACTIONS: OnceCell<Arc<Mutex<AHashMap<[u8; 32], RecentTx>>>> = OnceCell::new();

pub(crate) fn account_transfers_cache_init() {
    if FEES_SINK_ACCOUNTS.get().is_none() {
        FEES_SINK_ACCOUNTS
            .set(Arc::new(Mutex::new(Vec::new())))
            .unwrap();
    }
    if RECENT_TRANSACTIONS.get().is_none() {
        RECENT_TRANSACTIONS
            .set(Arc::new(Mutex::new(AHashMap::default())))
            .unwrap();
    }
}

fn fees_sink_accounts_lock() -> tokio::sync::MutexGuard<'static, Vec<IcrcCompatibleAccount>> {
    FEES_SINK_ACCOUNTS
        .get()
        .expect("FEES_SINK_ACCOUNTS not initialized")
        .blocking_lock()
}

fn recent_transactions_lock() -> tokio::sync::MutexGuard<'static, AHashMap<[u8; 32], RecentTx>> {
    RECENT_TRANSACTIONS
        .get()
        .expect("RECENT_TRANSACTIONS not initialized")
        .blocking_lock()
}

pub fn recent_transactions_cleanup() {
    let now = get_timestamp_ns();
    recent_transactions_lock()
        .retain(|_, recent_tx| recent_tx.timestamp > now.saturating_sub(TX_WINDOW));
}

fn recent_transaction_find(tx_hash: &[u8; 32]) -> u64 {
    recent_transactions_lock()
        .get(tx_hash)
        .map(|recent_tx| recent_tx.tx_num)
        .unwrap_or(0)
}

fn recent_transaction_add(tx_hash: &[u8; 32], tx_num: u64, timestamp: u64) {
    recent_transactions_lock().insert(*tx_hash, RecentTx { tx_num, timestamp });
}

pub fn ledger_funds_transfer(
    ledger: &mut LedgerMap,
    transfer: FundsTransfer,
) -> Result<Nat, TransferError> {
    let amount = transfer.amount();
    let transfer_bytes = transfer.to_bytes()?;
    let transfer_id = transfer.to_tx_id();

    // Check for duplicate transaction
    let duplicate_tx = recent_transaction_find(&transfer_id);
    if duplicate_tx > 0 {
        return Err(TransferError::Duplicate {
            duplicate_of_block: Nat::from(duplicate_tx),
        });
    }
    if !transfer.from().is_minting_account() {
        let balance_from = account_balance_get(transfer.from());
        let amount_withdraw_from = amount + transfer.fee().unwrap_or_default();
        if balance_from < amount_withdraw_from {
            return Err(TransferError::InsufficientFunds {
                account: transfer.from().clone().into(),
                current_balance: balance_from,
                requested_amount: amount_withdraw_from,
            });
        }
        account_balance_sub(transfer.from(), amount_withdraw_from).map_err(|e| {
            TransferError::GenericError {
                error_code: 10136u64.into(),
                message: e.to_string(),
            }
        })?;
    }
    ledger
        .upsert(LABEL_DC_TOKEN_TRANSFER, transfer_id, transfer_bytes)
        .map_err(|e| TransferError::GenericError {
            error_code: 10140u64.into(),
            message: e.to_string(),
        })?;
    account_balance_add(transfer.to(), amount).map_err(|e| TransferError::GenericError {
        error_code: 10144u64.into(),
        message: e.to_string(),
    })?;
    let new_tx_num = RecentCache::get_next_tx_num();
    recent_transaction_add(
        &transfer_id,
        new_tx_num,
        transfer.created_at_time().unwrap_or(get_timestamp_ns()),
    );

    RecentCache::add_entry(new_tx_num, transfer.into());
    Ok(Nat::from(new_tx_num))
}

pub fn charge_fees_to_account_and_bump_reputation(
    ledger: &mut LedgerMap,
    dcc_id: &DccIdentity,
    amount_e9s: TokenAmountE9s,
    memo: &str,
) -> Result<(), String> {
    if amount_e9s == 0 {
        return Ok(());
    }
    let from_icrc1_account = dcc_id.as_icrc_compatible_account();
    let balance_from_before = account_balance_get(&from_icrc1_account);
    match ledger_funds_transfer(
        ledger,
        // Burn 0 tokens, and transfer the entire amount_e9s to the fee accounts
        FundsTransfer::new(
            from_icrc1_account,
            MINTING_ACCOUNT,
            amount_e9s.into(),
            Some(fees_sink_accounts()),
            Some(get_timestamp_ns()),
            memo.as_bytes().to_vec(),
            0,
            balance_from_before.saturating_sub(amount_e9s),
            0,
        ),
    ) {
        Ok(_) => Ok(ledger_add_reputation_change(
            ledger,
            dcc_id,
            amount_e9s as i64,
        )?),
        Err(e) => {
            info!("Failed to charge fees: {}", e);
            Err(e.to_string())
        }
    }
}

pub fn charge_fees_to_account_no_bump_reputation(
    ledger: &mut LedgerMap,
    dcc_id_charge: &DccIdentity,
    amount_e9s: TokenAmountE9s,
    memo: &str,
) -> Result<(), String> {
    if amount_e9s == 0 {
        return Ok(());
    }
    let from_icrc1_account = dcc_id_charge.as_icrc_compatible_account();
    let balance_from_before = account_balance_get(&from_icrc1_account);
    match ledger_funds_transfer(
        ledger,
        // Burn 0 tokens, and transfer the entire amount_e9s to the fee accounts
        FundsTransfer::new(
            from_icrc1_account,
            MINTING_ACCOUNT,
            Some(amount_e9s),
            Some(fees_sink_accounts()),
            Some(get_timestamp_ns()),
            memo.as_bytes().to_vec(),
            0,
            balance_from_before.saturating_sub(amount_e9s),
            0,
        ),
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            info!("Failed to charge fees: {}", e);
            Err(e.to_string())
        }
    }
}

pub enum IncreaseReputation {
    None,
    Sender,
    Recipient,
}

pub fn do_funds_transfer(
    ledger: &mut LedgerMap,
    from_dcc_id: &DccIdentity,
    to_dcc_id: &DccIdentity,
    transfer_amount_e9s: TokenAmountE9s,
    fees_amount_e9s: TokenAmountE9s,
    memo: &[u8],
    increase_reputation: IncreaseReputation,
) -> Result<String, String> {
    let from_icrc1_account = from_dcc_id.as_icrc_compatible_account();
    let to_icrc1_account = to_dcc_id.as_icrc_compatible_account();

    if transfer_amount_e9s == 0 {
        return Ok("Nothing to transfer".to_string());
    }
    let balance_from_before = account_balance_get(&from_icrc1_account);
    let balance_to_before = account_balance_get(&to_icrc1_account);
    if balance_from_before < transfer_amount_e9s + fees_amount_e9s {
        return Err(format!(
            "Not enough funds to transfer: {} < {} + {}",
            balance_from_before, transfer_amount_e9s, fees_amount_e9s
        ));
    }
    let balance_from_after = balance_from_before - transfer_amount_e9s - fees_amount_e9s;
    match ledger_funds_transfer(
        ledger,
        FundsTransfer::new(
            from_icrc1_account.clone(),
            to_icrc1_account.clone(),
            Some(fees_amount_e9s),
            Some(fees_sink_accounts()),
            Some(get_timestamp_ns()),
            memo.to_vec(),
            transfer_amount_e9s,
            balance_from_after,
            balance_to_before + transfer_amount_e9s,
        ),
    ) {
        Ok(_) => {
            let response = format!(
                "Transferred {} tokens from {} \t to account {}, and charged fees {} tokens",
                amount_as_string(transfer_amount_e9s),
                from_icrc1_account,
                to_icrc1_account,
                amount_as_string(fees_amount_e9s)
            );
            match increase_reputation {
                IncreaseReputation::None => (),
                IncreaseReputation::Sender => {
                    ledger_add_reputation_change(ledger, from_dcc_id, fees_amount_e9s as i64)?;
                }
                IncreaseReputation::Recipient => {
                    ledger_add_reputation_change(ledger, to_dcc_id, fees_amount_e9s as i64)?;
                }
            }
            Ok(response)
        }
        Err(e) => {
            info!("Failed to charge fees: {}", e);
            Err(e.to_string())
        }
    }
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IcrcCompatibleAccount {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

impl Default for IcrcCompatibleAccount {
    fn default() -> Self {
        IcrcCompatibleAccount {
            owner: Principal::from_slice(&[]),
            subaccount: None,
        }
    }
}

impl BorshSerialize for IcrcCompatibleAccount {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let owner_bytes = self.owner.as_slice();
        BorshSerialize::serialize(&owner_bytes.to_vec(), writer)?;
        BorshSerialize::serialize(&self.subaccount, writer)?;
        Ok(())
    }
}

impl BorshDeserialize for IcrcCompatibleAccount {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        // Deserialize the owner from Vec<u8>
        let owner_vec: Vec<u8> = BorshDeserialize::deserialize(buf)?;
        let owner = Principal::from_slice(&owner_vec);

        // Deserialize the subaccount
        let subaccount: Option<Vec<u8>> = BorshDeserialize::deserialize(buf)?;

        Ok(IcrcCompatibleAccount { owner, subaccount })
    }

    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // Deserialize the owner from Vec<u8>
        let owner_vec: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
        let owner = Principal::from_slice(&owner_vec);
        let subaccount = BorshDeserialize::deserialize_reader(reader)?;
        Ok(IcrcCompatibleAccount { owner, subaccount })
    }
}

#[allow(dead_code)]
impl IcrcCompatibleAccount {
    pub fn new(owner: Principal, subaccount: Option<Vec<u8>>) -> Self {
        IcrcCompatibleAccount { owner, subaccount }
    }

    pub const fn new_minting() -> Self {
        IcrcCompatibleAccount {
            owner: MINTING_ACCOUNT_PRINCIPAL,
            subaccount: None,
        }
    }

    pub fn is_minting_account(&self) -> bool {
        self.owner == MINTING_ACCOUNT_PRINCIPAL
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, std::io::Error> {
        borsh::from_slice(bytes)
    }

    pub fn to_dcc_identity(&self) -> Option<DccIdentity> {
        let owner_bytes = get_pubkey_from_principal(self.owner);
        if owner_bytes.is_empty() {
            return None;
        }
        DccIdentity::new_verifying_from_bytes(&owner_bytes).ok()
    }
}

impl From<&str> for IcrcCompatibleAccount {
    fn from(owner: &str) -> Self {
        IcrcCompatibleAccount {
            owner: Principal::from_text(owner).unwrap(),
            subaccount: None,
        }
    }
}

impl From<&String> for IcrcCompatibleAccount {
    fn from(owner: &String) -> Self {
        IcrcCompatibleAccount::from(owner.as_str())
    }
}

impl From<icrc_ledger_types::icrc1::account::Account> for IcrcCompatibleAccount {
    fn from(account: icrc_ledger_types::icrc1::account::Account) -> Self {
        IcrcCompatibleAccount {
            owner: account.owner,
            subaccount: account.subaccount.map(|s| s.to_vec()),
        }
    }
}

impl From<IcrcCompatibleAccount> for Icrc1Account {
    fn from(account: IcrcCompatibleAccount) -> Self {
        Icrc1Account {
            owner: account.owner,
            subaccount: account
                .subaccount
                .map(|s| *slice_to_32_bytes_array(&s).unwrap()),
        }
    }
}

impl From<&IcrcCompatibleAccount> for Icrc1Account {
    fn from(account: &IcrcCompatibleAccount) -> Self {
        Icrc1Account {
            owner: account.owner,
            subaccount: account
                .subaccount
                .as_ref()
                .map(|s| *slice_to_32_bytes_array(s).unwrap()),
        }
    }
}

pub fn set_fees_sink_accounts(accounts: Option<Vec<IcrcCompatibleAccount>>) {
    *fees_sink_accounts_lock() = accounts.unwrap_or(vec![MINTING_ACCOUNT.clone()]).clone();
}

pub fn fees_sink_accounts() -> Vec<IcrcCompatibleAccount> {
    fees_sink_accounts_lock().clone()
}

fn full_account_checksum(owner: &[u8], subaccount: &[u8]) -> String {
    let mut crc32hasher = crc32fast::Hasher::new();
    crc32hasher.update(owner);
    crc32hasher.update(subaccount);
    let checksum = crc32hasher.finalize().to_be_bytes();
    BASE32.encode(&checksum).to_lowercase()
}

impl std::fmt::Display for IcrcCompatibleAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-1/TextualEncoding.md#textual-encoding-of-icrc-1-accounts
        match &self.subaccount {
            None => write!(f, "{}", self.owner),
            Some(subaccount) if subaccount == &[0; 32] => write!(f, "{}", self.owner),
            Some(subaccount) => {
                let checksum = full_account_checksum(self.owner.as_slice(), subaccount.as_slice());
                let hex_subaccount = hex::encode(subaccount.as_slice());
                let hex_subaccount = hex_subaccount.trim_start_matches('0');
                write!(f, "{}-{}.{}", self.owner, checksum, hex_subaccount)
            }
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct FundsTransferV1 {
    pub from: IcrcCompatibleAccount,
    pub to: IcrcCompatibleAccount,
    pub fee: Option<TokenAmountE9s>,
    pub fees_accounts: Option<Vec<IcrcCompatibleAccount>>,
    pub created_at_time: Option<u64>,
    pub memo: Vec<u8>,
    pub amount: TokenAmountE9s,
    pub balance_from_after: TokenAmountE9s,
    pub balance_to_after: TokenAmountE9s,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub enum FundsTransfer {
    V1(FundsTransferV1),
}

#[allow(dead_code)]
impl FundsTransfer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        from: IcrcCompatibleAccount,
        to: IcrcCompatibleAccount,
        fee: Option<TokenAmountE9s>,
        fees_accounts: Option<Vec<IcrcCompatibleAccount>>,
        created_at_time: Option<u64>,
        memo: Vec<u8>,
        amount: TokenAmountE9s,
        balance_from_after: TokenAmountE9s,
        balance_to_after: TokenAmountE9s,
    ) -> Self {
        FundsTransfer::V1(FundsTransferV1 {
            from,
            to,
            fee,
            fees_accounts,
            created_at_time,
            memo,
            amount,
            balance_from_after,
            balance_to_after,
        })
    }

    pub fn from(&self) -> &IcrcCompatibleAccount {
        match self {
            FundsTransfer::V1(ft) => &ft.from,
        }
    }

    pub fn to(&self) -> &IcrcCompatibleAccount {
        match self {
            FundsTransfer::V1(ft) => &ft.to,
        }
    }

    pub fn fee(&self) -> Option<TokenAmountE9s> {
        match self {
            FundsTransfer::V1(ft) => ft.fee,
        }
    }

    pub fn fee_accounts(&self) -> Option<&[IcrcCompatibleAccount]> {
        match self {
            FundsTransfer::V1(ft) => ft.fees_accounts.as_deref(),
        }
    }

    pub fn created_at_time(&self) -> Option<u64> {
        match self {
            FundsTransfer::V1(ft) => ft.created_at_time,
        }
    }

    pub fn memo(&self) -> &[u8] {
        match self {
            FundsTransfer::V1(ft) => &ft.memo,
        }
    }

    pub fn amount(&self) -> TokenAmountE9s {
        match self {
            FundsTransfer::V1(ft) => ft.amount,
        }
    }

    pub fn balance_from_after(&self) -> TokenAmountE9s {
        match self {
            FundsTransfer::V1(ft) => ft.balance_from_after,
        }
    }

    pub fn balance_to_after(&self) -> TokenAmountE9s {
        match self {
            FundsTransfer::V1(ft) => ft.balance_to_after,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, std::io::Error> {
        borsh::from_slice(bytes)
    }

    pub fn to_tx_id(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.from().owner.as_slice());
        hasher.update(self.from().subaccount.clone().unwrap_or([0; 32].to_vec()));
        hasher.update(self.to().owner.as_slice());
        hasher.update(self.to().subaccount.clone().unwrap_or([0; 32].to_vec()));
        hasher.update(self.amount().to_be_bytes());
        if let Some(fee) = &self.fee() {
            hasher.update(fee.to_be_bytes());
        }
        if !self.memo().is_empty() {
            hasher.update(self.memo());
        }
        if let Some(created_at_time) = self.created_at_time() {
            hasher.update(created_at_time.to_be_bytes());
        }
        hasher.finalize().into()
    }
}

impl std::fmt::Display for FundsTransfer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FundsTransfer {{ from={} to={} fee={:?} fee_accounts=[{}] created_at_time={:?} memo={} amount={} }}",
            self.from(),
            self.to(),
            self.fee(),
            self.fee_accounts().unwrap_or_default().iter().map(|f| f.to_string()).collect::<Vec<String>>().join(", "),
            self.created_at_time(),
            match String::try_from_slice(self.memo()) {
                Ok(memo) => memo,
                Err(_) => BASE64.encode(self.memo()),
            },
            self.amount()
        )
    }
}

impl From<FundsTransfer> for Transaction {
    fn from(ft: FundsTransfer) -> Self {
        if ft.from().is_minting_account() {
            Transaction {
                kind: "mint".into(),
                mint: Some(Mint {
                    amount: ft.amount().into(),
                    to: ft.to().into(),
                    memo: None,
                    created_at_time: ft.created_at_time(),
                }),
                burn: None,
                transfer: None,
                approve: None,
                timestamp: ft.created_at_time().unwrap_or_default(),
            }
        } else if ft.to().is_minting_account() {
            Transaction {
                kind: "burn".into(),
                mint: None,
                burn: Some(Burn {
                    amount: (ft.amount() + ft.fee().unwrap_or_default()).into(),
                    from: ft.from().into(),
                    spender: None,
                    memo: None,
                    created_at_time: ft.created_at_time(),
                }),
                transfer: None,
                approve: None,
                timestamp: ft.created_at_time().unwrap_or_default(),
            }
        } else {
            Transaction {
                kind: "transfer".into(),
                mint: None,
                burn: None,
                transfer: Some(Transfer {
                    from: ft.from().into(),
                    to: ft.to().into(),
                    spender: None,
                    amount: ft.amount().into(),
                    fee: Some(DC_TOKEN_TRANSFER_FEE_E9S.into()),
                    memo: None,
                    created_at_time: ft.created_at_time(),
                }),
                approve: None,
                timestamp: ft.created_at_time().unwrap_or_default(),
            }
        }
    }
}
