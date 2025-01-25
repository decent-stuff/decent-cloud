use crate::{AHashMap, IcrcCompatibleAccount, TokenAmountE9s, LABEL_DC_TOKEN_APPROVAL};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use icrc_ledger_types::{icrc1::account::Account, icrc2::allowance::Allowance};
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
}

impl FundsTransferApprovalV1 {
    pub fn new(
        approver: IcrcCompatibleAccount,
        spender: IcrcCompatibleAccount,
        allowance: TokenAmountE9s,
        expires_at: Option<u64>,
        fee: TokenAmountE9s,
        memo: Vec<u8>,
    ) -> Self {
        Self {
            approver,
            spender,
            allowance,
            expires_at,
            fee,
            memo,
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
    ) -> Self {
        Self::V1(FundsTransferApprovalV1 {
            approver,
            spender,
            allowance,
            expires_at,
            fee,
            memo,
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

    pub fn add_to_ledger(&self, ledger: &mut LedgerMap) -> Result<(), LedgerError> {
        ledger.upsert(
            LABEL_DC_TOKEN_APPROVAL,
            self.to_tx_id(),
            borsh::to_vec(self).unwrap(),
        )
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
}

impl std::fmt::Display for FundsTransferApproval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundsTransferApproval::V1(v1) => write!(f, "{}", v1),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ledger_funds_transfer_approve(
    ledger: &mut LedgerMap,
    approver: &IcrcCompatibleAccount,
    spender: &IcrcCompatibleAccount,
    allowance: TokenAmountE9s,
    expires_at: Option<u64>,
    fee: TokenAmountE9s,
    memo: Vec<u8>,
) -> Result<(), LedgerError> {
    let approval = FundsTransferApproval::new(
        approver.clone(),
        spender.clone(),
        allowance,
        expires_at,
        fee,
        memo,
    );
    approval.add_to_ledger(ledger)
}
