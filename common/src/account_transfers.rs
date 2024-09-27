use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use candid::{CandidType, Deserialize, Nat, Principal};
use data_encoding::BASE32;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use icrc_ledger_types::{
    icrc1::account::Account as Icrc1Account,
    icrc3::transactions::{Burn, Mint, Transaction, Transfer},
};
use ledger_map::{info, LedgerMap};
use num_bigint::BigUint;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    iter::Sum,
    ops::{Add, AddAssign},
};

use crate::{
    account_balance_add, account_balance_get, account_balance_sub, get_timestamp_ns,
    ledger_add_reputation_change, slice_to_32_bytes_array, Balance, DccIdentity, TransferError,
    LABEL_DC_TOKEN_TRANSFER, MINTING_ACCOUNT, MINTING_ACCOUNT_PRINCIPAL,
};

thread_local! {
    static FEES_SINK_ACCOUNTS: RefCell<Option<Vec<IcrcCompatibleAccount>>> = const { RefCell::new(None) };
}

pub fn ledger_funds_transfer(
    ledger: &mut LedgerMap,
    transfer: FundsTransfer,
) -> Result<(), TransferError> {
    info!("{}", transfer);
    let amount = transfer.amount();
    let transfer_bytes = transfer.to_bytes()?;
    let transfer_id = transfer.to_tx_id();
    if !transfer.from().is_minting_account() {
        let balance_from = account_balance_get(transfer.from());
        let amount_withdraw_from = amount + transfer.fee().unwrap_or_default();
        if balance_from < amount_withdraw_from {
            return Err(TransferError::InsufficientFunds {
                account: transfer.from().clone().into(),
                current_balance: balance_from.into(),
                requested_amount: amount_withdraw_from.into(),
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
    Ok(())
}

pub fn charge_fees_to_account_and_bump_reputation(
    ledger: &mut LedgerMap,
    dcc_identity: &DccIdentity,
    amount_e9s: Balance,
) -> Result<(), String> {
    if amount_e9s == 0 {
        return Ok(());
    }
    let balance_from_after =
        account_balance_get(&dcc_identity.as_icrc_compatible_account()) - amount_e9s;
    match ledger_funds_transfer(
        ledger,
        // Burn 0 tokens, and transfer the entire amount_e9s to the fee accounts
        FundsTransfer::new(
            dcc_identity.as_icrc_compatible_account(),
            MINTING_ACCOUNT,
            amount_e9s.into(),
            Some(fees_sink_accounts()),
            Some(get_timestamp_ns()),
            vec![],
            0,
            balance_from_after,
            0,
        ),
    ) {
        Ok(_) => Ok(ledger_add_reputation_change(
            ledger,
            dcc_identity,
            amount_e9s.min(i64::MAX as Balance) as i64,
        )?),
        Err(e) => {
            info!("Failed to charge fees: {}", e);
            Err(e.to_string())
        }
    }
}

pub fn charge_fees_to_account_no_bump_reputation(
    ledger: &mut LedgerMap,
    dcc_identity: &DccIdentity,
    amount_e9s: Balance,
) -> Result<(), String> {
    if amount_e9s == 0 {
        return Ok(());
    }
    let balance_from_after =
        account_balance_get(&dcc_identity.as_icrc_compatible_account()) - amount_e9s;
    match ledger_funds_transfer(
        ledger,
        // Burn 0 tokens, and transfer the entire amount_e9s to the fee accounts
        FundsTransfer::new(
            dcc_identity.as_icrc_compatible_account(),
            MINTING_ACCOUNT,
            Some(amount_e9s),
            Some(fees_sink_accounts()),
            Some(get_timestamp_ns()),
            vec![],
            0,
            balance_from_after,
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
    FEES_SINK_ACCOUNTS.with(|fees_sink_accounts| {
        *fees_sink_accounts.borrow_mut() = accounts;
    });
}

pub fn fees_sink_accounts() -> Vec<IcrcCompatibleAccount> {
    FEES_SINK_ACCOUNTS.with(
        |fees_sink_accounts| match fees_sink_accounts.borrow().as_ref() {
            Some(accounts) => accounts.clone(),
            None => vec![MINTING_ACCOUNT.clone()],
        },
    )
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

#[derive(CandidType, Deserialize, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct NumTokens(pub Nat);

impl From<u128> for NumTokens {
    fn from(value: u128) -> Self {
        NumTokens(Nat::from(value))
    }
}

impl From<u64> for NumTokens {
    fn from(value: u64) -> Self {
        NumTokens(Nat::from(value))
    }
}

impl From<u32> for NumTokens {
    fn from(value: u32) -> Self {
        NumTokens(Nat::from(value))
    }
}

impl From<Nat> for NumTokens {
    fn from(value: Nat) -> Self {
        NumTokens(value)
    }
}

impl From<BigUint> for NumTokens {
    fn from(value: BigUint) -> Self {
        NumTokens(Nat::from(value))
    }
}

impl From<NumTokens> for Nat {
    fn from(value: NumTokens) -> Self {
        value.0
    }
}

impl std::fmt::Display for NumTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Sum<NumTokens> for NumTokens {
    fn sum<I: Iterator<Item = NumTokens>>(iter: I) -> Self {
        iter.map(|n| n.0 .0).sum::<BigUint>().into()
    }
}

impl<'a> Sum<&'a NumTokens> for NumTokens {
    fn sum<I: Iterator<Item = &'a NumTokens>>(iter: I) -> Self {
        iter.map(|n| &n.0 .0).sum::<BigUint>().into()
    }
}

impl Add<NumTokens> for NumTokens {
    type Output = NumTokens;
    fn add(self, rhs: NumTokens) -> NumTokens {
        (self.0 .0 + rhs.0 .0).into()
    }
}

impl AddAssign<NumTokens> for NumTokens {
    fn add_assign(&mut self, rhs: NumTokens) {
        self.0 .0 += rhs.0 .0;
    }
}

impl BorshSerialize for NumTokens {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        BorshSerialize::serialize(&self.0 .0.to_bytes_le(), writer)
    }
}

impl BorshDeserialize for NumTokens {
    fn deserialize(buf: &mut &[u8]) -> Result<Self, std::io::Error> {
        let bytes: Vec<u8> = BorshDeserialize::deserialize(buf)?;
        Ok(NumTokens(Nat::from(BigUint::from_bytes_le(&bytes))))
    }

    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
        Ok(NumTokens(Nat::from(BigUint::from_bytes_le(&bytes))))
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct FundsTransferV1 {
    pub from: IcrcCompatibleAccount,
    pub to: IcrcCompatibleAccount,
    pub fee: Option<Balance>,
    pub fees_accounts: Option<Vec<IcrcCompatibleAccount>>,
    pub created_at_time: Option<u64>,
    pub memo: Vec<u8>,
    pub amount: Balance,
    pub balance_from_after: Balance,
    pub balance_to_after: Balance,
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
        fee: Option<Balance>,
        fees_accounts: Option<Vec<IcrcCompatibleAccount>>,
        created_at_time: Option<u64>,
        memo: Vec<u8>,
        amount: Balance,
        balance_from_after: Balance,
        balance_to_after: Balance,
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

    pub fn fee(&self) -> Option<Balance> {
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

    pub fn amount(&self) -> Balance {
        match self {
            FundsTransfer::V1(ft) => ft.amount,
        }
    }

    pub fn balance_from_after(&self) -> Balance {
        match self {
            FundsTransfer::V1(ft) => ft.balance_from_after,
        }
    }

    pub fn balance_to_after(&self) -> Balance {
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
        hasher.update(self.to_bytes().unwrap());
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
                    amount: ft.amount().into(),
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
                kind: "xfer".into(),
                mint: None,
                burn: None,
                transfer: Some(Transfer {
                    from: ft.from().into(),
                    to: ft.to().into(),
                    spender: None,
                    amount: ft.amount().into(),
                    fee: None,
                    memo: None,
                    created_at_time: ft.created_at_time(),
                }),
                approve: None,
                timestamp: ft.created_at_time().unwrap_or_default(),
            }
        }
    }
}
