use crate::{
    AHashMap, IcrcCompatibleAccount, RecentCache, TokenAmountE9s, DC_TOKEN_TRANSFER_FEE_E9S,
    LABEL_DC_TOKEN_APPROVAL,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use candid::Nat;
use icrc_ledger_types::{
    icrc1::{account::Account, transfer::Memo},
    icrc2::allowance::Allowance,
    icrc3::transactions::{Approve, Transaction},
};
use ledger_map::{LedgerError, LedgerMap};
use sha2::Digest;

thread_local! {
    static APPROVALS: std::cell::RefCell<AHashMap<(Account, Account), Allowance>> = std::cell::RefCell::new(AHashMap::default());
}

pub fn approval_update(account: Account, spender: Account, allowance: Allowance) {
    APPROVALS.with(|approvals| {
        let mut approvals = approvals.borrow_mut();
        if allowance.allowance > 0u32 {
            approvals.insert((account, spender), allowance);
        } else {
            approvals.remove(&(account, spender));
        }
    })
}

pub fn approval_get(account: Account, spender: Account) -> Option<Allowance> {
    APPROVALS.with(|approvals| {
        let approvals = approvals.borrow();
        approvals.get(&(account, spender)).cloned()
    })
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct FundsTransferApprovalV1 {
    approver: IcrcCompatibleAccount,
    spender: IcrcCompatibleAccount,
    allowance: TokenAmountE9s,
    expires_at: Option<u64>,
    fee: TokenAmountE9s,
    memo: Vec<u8>,
    created_at_time: u64,
}

impl FundsTransferApprovalV1 {
    pub fn new(
        approver: IcrcCompatibleAccount,
        spender: IcrcCompatibleAccount,
        allowance: TokenAmountE9s,
        expires_at: Option<u64>,
        fee: TokenAmountE9s,
        memo: Vec<u8>,
        created_at_time: u64,
    ) -> Self {
        Self {
            approver,
            spender,
            allowance,
            expires_at,
            fee,
            memo,
            created_at_time,
        }
    }

    pub fn to_tx_id(&self) -> [u8; 32] {
        let mut hasher = sha2::Sha256::new();
        hasher.update(borsh::to_vec(self).unwrap());
        let result = hasher.finalize();
        let mut tx_id = [0u8; 32];
        tx_id.copy_from_slice(&result[..32]);
        tx_id
    }

    pub fn spender(&self) -> &IcrcCompatibleAccount {
        &self.spender
    }

    pub fn approver(&self) -> &IcrcCompatibleAccount {
        &self.approver
    }

    pub fn allowance(&self) -> Allowance {
        Allowance {
            allowance: self.allowance.into(),
            expires_at: self.expires_at,
        }
    }
}

impl std::fmt::Display for FundsTransferApprovalV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FundsTransferApprovalV1 {{ approver={} spender={} allowance={} expires_at={:?} fee={} memo={} }}",
            self.spender(),
            self.approver(),
            self.allowance,
            self.expires_at,
            self.fee,
            match String::try_from_slice(&self.memo) {
                Ok(memo) => memo,
                Err(_) => BASE64.encode(&self.memo),
            },
        )
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub enum FundsTransferApproval {
    V1(FundsTransferApprovalV1),
}

impl FundsTransferApproval {
    pub fn new(
        approver: IcrcCompatibleAccount,
        spender: IcrcCompatibleAccount,
        allowance: TokenAmountE9s,
        expires_at: Option<u64>,
        fee: TokenAmountE9s,
        memo: Vec<u8>,
        created_at_time: u64,
    ) -> Self {
        Self::V1(FundsTransferApprovalV1 {
            approver,
            spender,
            allowance,
            expires_at,
            fee,
            memo,
            created_at_time,
        })
    }

    pub fn to_tx_id(&self) -> [u8; 32] {
        match self {
            FundsTransferApproval::V1(v1) => v1.to_tx_id(),
        }
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, std::io::Error> {
        borsh::from_slice(bytes)
    }

    pub fn add_to_ledger(&self, ledger: &mut LedgerMap) -> Result<Nat, LedgerError> {
        ledger.upsert(
            LABEL_DC_TOKEN_APPROVAL,
            self.to_tx_id(),
            borsh::to_vec(self).unwrap(),
        )?;
        let new_tx_num = RecentCache::get_next_tx_num();
        RecentCache::add_entry(new_tx_num, self.into());
        Ok(new_tx_num.into())
    }

    pub fn approver(&self) -> &IcrcCompatibleAccount {
        match self {
            FundsTransferApproval::V1(v1) => v1.approver(),
        }
    }

    pub fn spender(&self) -> &IcrcCompatibleAccount {
        match self {
            FundsTransferApproval::V1(v1) => v1.spender(),
        }
    }

    pub fn allowance(&self) -> Allowance {
        match self {
            FundsTransferApproval::V1(v1) => v1.allowance(),
        }
    }

    pub fn memo(&self) -> &[u8] {
        match self {
            FundsTransferApproval::V1(v1) => &v1.memo,
        }
    }

    pub fn created_at_time(&self) -> u64 {
        match self {
            FundsTransferApproval::V1(v1) => v1.created_at_time,
        }
    }
}

impl std::fmt::Display for FundsTransferApproval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundsTransferApproval::V1(v1) => write!(f, "{}", v1),
        }
    }
}

impl From<&FundsTransferApproval> for Transaction {
    fn from(fta: &FundsTransferApproval) -> Self {
        let allowance = fta.allowance();
        Transaction {
            kind: "transfer".into(),
            mint: None,
            burn: None,
            transfer: None,
            approve: Some(Approve {
                from: fta.approver().into(),
                spender: fta.spender().into(),
                amount: allowance.allowance,
                expected_allowance: None,
                expires_at: allowance.expires_at,
                memo: if fta.memo().is_empty() {
                    None
                } else {
                    Some(Memo(fta.memo().to_vec().into()))
                },
                fee: Some(DC_TOKEN_TRANSFER_FEE_E9S.into()),
                created_at_time: Some(fta.created_at_time()),
            }),
            timestamp: fta.created_at_time(),
        }
    }
}
