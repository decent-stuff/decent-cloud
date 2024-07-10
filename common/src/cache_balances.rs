use crate::account_transfers::Account;
use crate::{AHashMap, DC_TOKEN_DECIMALS_DIV};
use candid::Nat;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use num_bigint::BigUint;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ACCOUNT_BALANCES: RefCell<AHashMap<Account, BigUint>> = RefCell::new(HashMap::default());
}

pub fn account_balance_get(account: &Account) -> Nat {
    ACCOUNT_BALANCES.with(|balances| {
        let balances = balances.borrow();
        match balances.get(account) {
            Some(balance) => Nat::from(balance.clone()),
            None => Nat::from(0u8),
        }
    })
}

pub fn account_balance_get_as_string(account: &Account) -> String {
    amount_as_string(&account_balance_get(account))
}

pub fn amount_as_string(amount: &Nat) -> String {
    let balance = amount.0.to_u64_digits();
    if balance.is_empty() {
        return "0.0".to_string();
    }
    assert_eq!(balance.len(), 1);
    amount_as_string_u64(balance[0])
}

pub fn amount_as_string_u64(amount: u64) -> String {
    format!(
        "{}.{}",
        amount / DC_TOKEN_DECIMALS_DIV,
        amount % DC_TOKEN_DECIMALS_DIV
    )
}

pub fn amount_as_string_u128(amount: u128) -> String {
    amount_as_string_u64(amount as u64)
}

fn _balance_sub(balance: &BigUint, amount: &BigUint) -> anyhow::Result<BigUint> {
    if balance < amount {
        anyhow::bail!("Account balance too low, cannot subtract balance")
    }
    Ok(balance - amount)
}

#[allow(dead_code)]
pub fn account_balance_sub(account: &Account, amount: &Nat) -> anyhow::Result<Nat> {
    ACCOUNT_BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        let balance = balances.entry(account.clone()).or_default();
        *balance = _balance_sub(balance, &amount.0)?;
        Ok(Nat::from(balance.clone()))
    })
}

pub fn account_balance_add(account: &Account, amount: &Nat) -> anyhow::Result<Nat> {
    ACCOUNT_BALANCES.with(|balances| {
        let mut balances = balances.borrow_mut();
        let balance = balances.entry(account.clone()).or_default();
        *balance += &amount.0;
        Ok(Nat::from(balance.clone()))
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

    fn mk_test_account(owner: u32, subaccount: Option<u32>) -> Account {
        Account::new(
            Principal::from_slice(&owner.to_le_bytes()),
            subaccount.map(|s| s.to_le_bytes().to_vec()),
        )
    }

    #[test]
    fn test_accounts_ops() {
        let account_1 = mk_test_account(1, None);
        assert_eq!(account_balance_get(&account_1), 0u32);
        assert_eq!(
            account_balance_add(&account_1, &100u32.into()).unwrap(),
            100u32
        );
        assert_eq!(account_balance_get(&account_1), 100u32);
        assert_eq!(
            account_balance_sub(&account_1, &50u32.into()).unwrap(),
            50u32
        );
        assert_eq!(
            account_balance_sub(&account_1, &100u32.into())
                .unwrap_err()
                .to_string(),
            "Account balance too low, cannot subtract balance"
        );
        assert_eq!(
            account_balance_sub(&account_1, &50u32.into()).unwrap(),
            0u32
        );
        assert_eq!(account_balance_sub(&account_1, &0u32.into()).unwrap(), 0u32);
        assert_eq!(
            account_balance_sub(&account_1, &1u32.into())
                .unwrap_err()
                .to_string(),
            "Account balance too low, cannot subtract balance"
        );
        assert_eq!(
            account_balance_add(&account_1, &100u32.into()).unwrap(),
            100u32
        );
        account_balances_clear();
        assert_eq!(account_balance_get(&account_1), 0u32);
    }
}
