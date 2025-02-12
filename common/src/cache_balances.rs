use crate::account_transfers::IcrcCompatibleAccount;
use crate::{AHashMap, TokenAmountE9s, DC_TOKEN_DECIMALS_DIV};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::Mutex;

static ACCOUNT_BALANCES: OnceCell<Arc<Mutex<AHashMap<IcrcCompatibleAccount, TokenAmountE9s>>>> =
    OnceCell::new();

pub(crate) fn account_balances_cache_init() {
    if ACCOUNT_BALANCES.get().is_none() {
        ACCOUNT_BALANCES
            .set(Arc::new(Mutex::new(AHashMap::default())))
            .unwrap();
    }
}

fn account_balances_lock(
) -> tokio::sync::MutexGuard<'static, AHashMap<IcrcCompatibleAccount, TokenAmountE9s>> {
    ACCOUNT_BALANCES
        .get()
        .expect("Account balance cache not initialized")
        .blocking_lock()
}

pub fn account_balance_get(account: &IcrcCompatibleAccount) -> TokenAmountE9s {
    account_balances_lock()
        .get(account)
        .copied()
        .unwrap_or_default()
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
    let mut balances = account_balances_lock();

    let balance = balances.entry(account.clone()).or_default();
    if *balance < amount {
        anyhow::bail!("Account balance too low, cannot subtract balance")
    };
    *balance = balance.saturating_sub(amount);
    Ok(*balance)
}

pub fn account_balance_add(
    account: &IcrcCompatibleAccount,
    amount: TokenAmountE9s,
) -> anyhow::Result<TokenAmountE9s> {
    let mut balances = account_balances_lock();

    let balance = balances.entry(account.clone()).or_default();
    *balance = balance.saturating_add(amount);
    Ok(*balance)
}

#[allow(dead_code)]
pub fn account_balances_clear() {
    account_balances_lock().clear();
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
        account_balances_cache_init();
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
