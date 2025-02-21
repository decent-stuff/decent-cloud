use crate::account_transfers::IcrcCompatibleAccount;
use crate::{AHashMap, TokenAmountE9s, DC_TOKEN_DECIMALS_DIV};
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ACCOUNT_BALANCES: RefCell<AHashMap<IcrcCompatibleAccount, TokenAmountE9s>> = RefCell::new(HashMap::default());
}

pub fn account_balance_get(account: &IcrcCompatibleAccount) -> TokenAmountE9s {
    ACCOUNT_BALANCES.with(|balances| {
        let balances = balances.borrow();
        balances.get(account).copied().unwrap_or_default()
    })
}

pub fn account_balance_get_as_string(account: &IcrcCompatibleAccount) -> String {
    amount_as_string(account_balance_get(account))
}

pub fn amount_as_string(amount: TokenAmountE9s) -> String {
    if amount == 0 {
        return "0.0".to_string();
    }
    format!(
        "{}.{:0>9}",
        amount / DC_TOKEN_DECIMALS_DIV as TokenAmountE9s,
        amount % DC_TOKEN_DECIMALS_DIV as TokenAmountE9s
    )
}

#[allow(dead_code)]
pub fn account_balance_sub(
    account: &IcrcCompatibleAccount,
    amount: TokenAmountE9s,
) -> anyhow::Result<TokenAmountE9s> {
    ACCOUNT_BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        let balance = balances.entry(account.clone()).or_default();
        if *balance < amount {
            anyhow::bail!("Account balance too low, cannot subtract balance")
        };
        *balance = balance.saturating_sub(amount);
        Ok(*balance)
    })
}

pub fn account_balance_add(
    account: &IcrcCompatibleAccount,
    amount: TokenAmountE9s,
) -> anyhow::Result<TokenAmountE9s> {
    ACCOUNT_BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        let balance = balances.entry(account.clone()).or_default();
        *balance = balance.saturating_add(amount);
        Ok(*balance)
    })
}

#[allow(dead_code)]
pub fn account_balances_clear() {
    ACCOUNT_BALANCES.with(|balances| balances.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn mk_test_account(owner: u32, subaccount: Option<u32>) -> IcrcCompatibleAccount {
        IcrcCompatibleAccount::new(
            Principal::from_slice(&owner.to_le_bytes()),
            subaccount.map(|s| s.to_le_bytes().to_vec()),
        )
    }

    #[test]
    fn test_accounts_ops() {
        let account_1 = mk_test_account(1, None);
        assert_eq!(account_balance_get(&account_1), 0);
        assert_eq!(
            account_balance_add(&account_1, 100u32.into()).unwrap(),
            100u32 as TokenAmountE9s
        );
        assert_eq!(account_balance_get(&account_1), 100u32 as TokenAmountE9s);
        assert_eq!(
            account_balance_sub(&account_1, 50u32.into()).unwrap(),
            50u32 as TokenAmountE9s
        );
        assert_eq!(
            account_balance_sub(&account_1, 100u32.into())
                .unwrap_err()
                .to_string(),
            "Account balance too low, cannot subtract balance"
        );
        assert_eq!(
            account_balance_sub(&account_1, 50u32.into()).unwrap(),
            0u32 as TokenAmountE9s
        );
        assert_eq!(
            account_balance_sub(&account_1, 0u32.into()).unwrap(),
            0u32 as TokenAmountE9s
        );
        assert_eq!(
            account_balance_sub(&account_1, 1u32.into())
                .unwrap_err()
                .to_string(),
            "Account balance too low, cannot subtract balance"
        );
        assert_eq!(
            account_balance_add(&account_1, 100u32.into()).unwrap(),
            100u32 as TokenAmountE9s
        );
        account_balances_clear();
        assert_eq!(account_balance_get(&account_1), 0u32 as TokenAmountE9s);
    }
}
