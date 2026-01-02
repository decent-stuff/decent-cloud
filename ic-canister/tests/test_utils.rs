use candid::{encode_one, Encode, Nat, Principal};
use dcc_common::{DccIdentity, TokenAmountE9s, BLOCK_INTERVAL_SECS, FIRST_BLOCK_TIMESTAMP_NS};
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc1::transfer::{Memo, TransferArg, TransferError};
use once_cell::sync::Lazy;
use pocket_ic::PocketIc;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---- Common Macros ----

#[macro_export]
macro_rules! query_check_and_decode {
    ($pic:expr, $can:expr, $method_name:expr, $method_arg:expr, $decode_type:ty) => {{
        let reply = $pic.query_call(
            $can,
            Principal::anonymous(),
            $method_name,
            $method_arg.clone(),
        );
        let reply = match reply {
            Ok(reply) => reply,
            Err(err) => panic!("Received an error: {:?}", err),
        };

        candid::decode_one::<$decode_type>(&reply).expect("Failed to decode")
    }};
}

#[macro_export]
macro_rules! update_check_and_decode {
    ($pic:expr, $can:expr, $sender:expr, $method_name:expr, $method_arg:expr, $decode_type:ty) => {{
        let reply = $pic.update_call($can, $sender, $method_name, $method_arg.clone());
        let reply = match reply {
            Ok(reply) => reply,
            Err(err) => panic!("Received an error: {:?}", err),
        };

        candid::decode_one::<$decode_type>(&reply).expect("Failed to decode")
    }};
}

// ---- Utility Functions ----

pub fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .expect("Failed to execute 'cargo locate-project'. Make sure Cargo is installed and in PATH.");
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Failed to locate workspace directory.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
    let cargo_path = Path::new(std::str::from_utf8(&output.stdout).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

pub static CANISTER_WASM: Lazy<Vec<u8>> = Lazy::new(|| {
    let mut path = workspace_dir();
    let canister_dir = path.join("ic-canister");
    let output = Command::new("dfx")
        .arg("build")
        .current_dir(&canister_dir)
        .output()
        .expect("Failed to execute 'dfx build'. Make sure dfx is installed and in PATH.");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Failed to build canister at {}\nstdout: {}\nstderr: {}",
            canister_dir.display(),
            stdout,
            stderr
        );
    }

    path.push("target/wasm32-unknown-unknown/release/decent_cloud_canister.wasm");
    fs_err::read(path).unwrap()
});

// ---- Test Context ----

pub struct TestContext {
    pub pic: PocketIc,
    pub canister_id: Principal,
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl TestContext {
    pub fn new() -> Self {
        let pic = PocketIc::new();
        let canister_id = pic.create_canister();
        pic.add_cycles(canister_id, 20_000_000_000_000);

        pic.install_canister(
            canister_id,
            CANISTER_WASM.clone(),
            encode_one(true).expect("failed to encode"),
            None,
        );

        // Ensure deterministic timestamp
        let ts_ns = FIRST_BLOCK_TIMESTAMP_NS + 100 * BLOCK_INTERVAL_SECS * 1_000_000_000;
        let ts_1 = encode_one(ts_ns).unwrap();
        update_check_and_decode!(
            pic,
            canister_id,
            Principal::anonymous(),
            "set_timestamp_ns",
            ts_1,
            ()
        );

        Self { pic, canister_id }
    }

    pub fn get_timestamp_ns(&self) -> u64 {
        query_check_and_decode!(
            self.pic,
            self.canister_id,
            "get_timestamp_ns",
            encode_one(()).unwrap(),
            u64
        )
    }

    pub fn get_transfer_fee(&self) -> Nat {
        query_check_and_decode!(
            self.pic,
            self.canister_id,
            "icrc1_fee",
            encode_one(()).expect("failed to encode"),
            Nat
        )
    }

    pub fn mint_tokens_for_test(&self, acct: &Account, amount: u64) -> Nat {
        update_check_and_decode!(
            self.pic,
            self.canister_id,
            acct.owner,
            "mint_tokens_for_test",
            candid::encode_args((acct, amount, None::<Option<Memo>>)).unwrap(),
            Nat
        )
    }

    pub fn get_account_balance(&self, account: &Account) -> Nat {
        query_check_and_decode!(
            self.pic,
            self.canister_id,
            "icrc1_balance_of",
            encode_one(account).expect("failed to encode"),
            Nat
        )
    }

    pub fn transfer_funds(
        &self,
        send_from: &Account,
        send_to: &Account,
        amount_send: u64,
    ) -> Result<Nat, TransferError> {
        let transfer_args = TransferArg {
            from_subaccount: send_from.subaccount,
            to: *send_to,
            fee: Some(self.get_transfer_fee()),
            created_at_time: None,
            memo: None,
            amount: amount_send.into(),
        };

        update_check_and_decode!(
            self.pic,
            self.canister_id,
            send_from.owner,
            "icrc1_transfer",
            candid::encode_one(transfer_args).unwrap(),
            Result<Nat, TransferError>
        )
    }

    pub fn upgrade(&self) -> Result<(), pocket_ic::RejectResponse> {
        let no_args = encode_one(true).expect("failed to encode");
        self.pic
            .upgrade_canister(self.canister_id, CANISTER_WASM.clone(), no_args, None)
    }

    pub fn commit(&self) {
        let no_args: Vec<u8> = encode_one(()).expect("failed to encode");
        update_check_and_decode!(
            self.pic,
            self.canister_id,
            Principal::anonymous(),
            "run_periodic_task",
            no_args,
            ()
        )
    }

    pub fn ffwd_to_next_block(&self, mut ts_ns: u64) -> u64 {
        ts_ns += BLOCK_INTERVAL_SECS * 1_000_000_000;
        let ts_2 = encode_one(ts_ns).unwrap();
        update_check_and_decode!(
            self.pic,
            self.canister_id,
            Principal::anonymous(),
            "set_timestamp_ns",
            ts_2,
            ()
        );
        self.commit();
        ts_ns
    }
}

// ---- Account Management Functions ----

#[allow(dead_code)]
pub fn create_test_account(id: u8) -> Account {
    Account {
        owner: Principal::from_slice(&[id; 29]),
        subaccount: None,
    }
}

#[allow(dead_code)]
pub fn create_test_subaccount(owner: Principal, subaccount_id: u8) -> Account {
    Account {
        owner,
        subaccount: Some([subaccount_id; 32]),
    }
}

#[allow(dead_code)]
pub fn test_icrc1_account_from_slice(bytes: &[u8]) -> Account {
    Account {
        owner: Principal::from_slice(bytes),
        subaccount: None,
    }
}

// ---- Node Provider and User Management Functions ----

#[allow(dead_code)]
pub fn test_provider_register(
    ctx: &TestContext,
    seed: &[u8],
    initial_funds: TokenAmountE9s,
) -> (DccIdentity, Result<String, String>) {
    let dcc_identity = DccIdentity::new_from_seed(seed).unwrap();
    if initial_funds > 0 {
        ctx.mint_tokens_for_test(
            &dcc_identity.as_icrc_compatible_account().into(),
            initial_funds,
        );
    }
    let pubkey_bytes = dcc_identity.to_bytes_verifying();
    let pubkey_signature = dcc_identity.sign(&pubkey_bytes).unwrap();
    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        dcc_identity.to_ic_principal(),
        "provider_register",
        Encode!(&pubkey_bytes, &pubkey_signature.to_bytes()).unwrap(),
        Result<String, String>
    );
    (dcc_identity, result)
}

#[allow(dead_code)]
pub fn test_user_register(
    ctx: &TestContext,
    seed: &[u8],
    initial_funds: TokenAmountE9s,
) -> (DccIdentity, Result<String, String>) {
    let dcc_identity = DccIdentity::new_from_seed(seed).unwrap();
    if initial_funds > 0 {
        ctx.mint_tokens_for_test(
            &dcc_identity.as_icrc_compatible_account().into(),
            initial_funds,
        );
    }
    let pubkey_bytes = dcc_identity.to_bytes_verifying();
    let pubkey_signature = dcc_identity.sign(&pubkey_bytes).unwrap();
    let result = update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        dcc_identity.to_ic_principal(),
        "user_register",
        Encode!(&pubkey_bytes, &pubkey_signature.to_bytes()).unwrap(),
        Result<String, String>
    );
    (dcc_identity, result)
}

#[allow(dead_code)]
pub fn test_get_id_reputation(ctx: &TestContext, dcc_id: &DccIdentity) -> u64 {
    let args = Encode!(&dcc_id.to_bytes_verifying()).unwrap();
    query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "get_identity_reputation",
        args,
        u64
    )
}

#[allow(dead_code)]
pub fn test_provider_check_in(
    ctx: &TestContext,
    dcc_identity: &DccIdentity,
) -> Result<String, String> {
    let no_args = encode_one(()).expect("failed to encode");
    let nonce_bytes = query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "get_check_in_nonce",
        no_args,
        Vec<u8>
    );

    let crypto_sig = dcc_identity.sign(&nonce_bytes).unwrap().to_bytes();

    update_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        dcc_identity.to_ic_principal(),
        "provider_check_in",
        Encode!(
            &dcc_identity.to_bytes_verifying(),
            &String::from("Just a test memo!"),
            &crypto_sig
        )
        .unwrap(),
        Result<String, String>
    )
}

// ---- Ledger Entries Functions ----

#[allow(dead_code)]
pub fn test_ledger_entries(
    ctx: &TestContext,
    label: Option<String>,
    cursor: Option<decent_cloud_canister::canister_backend::generic::ResumeCursor>,
    limit: Option<u32>,
    include_next_block: Option<bool>,
) -> decent_cloud_canister::canister_backend::generic::LedgerEntriesResult {
    query_check_and_decode!(
        ctx.pic,
        ctx.canister_id,
        "ledger_entries",
        Encode!(&label, &cursor, &limit, &include_next_block).unwrap(),
        decent_cloud_canister::canister_backend::generic::LedgerEntriesResult
    )
}
