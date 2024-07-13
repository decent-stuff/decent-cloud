use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

pub fn main() {
    if Ok("x86_64") == env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        let workspace_dir_path = workspace_dir();
        let canister_dir_path = workspace_dir_path.join("ic-canister");
        println!("Building canister... {}", canister_dir_path.display());

        // List of environment variables that may influence the build process
        let vars_to_unset = [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CC",
            "CXX",
            "LD",
            "AR",
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_TARGET",
        ];

        // Iterate over the list and remove each variable
        for var in vars_to_unset.iter() {
            std::env::remove_var(var);
        }
        if !Command::new("dfx")
            .args(["canister", "create", "--all"])
            .current_dir(&canister_dir_path)
            .status()
            .expect("Failed to create canister")
            .success()
        {
            panic!("failed to create canister")
        };

        if !Command::new("dfx")
            .args(["build"])
            .current_dir(canister_dir_path)
            .status()
            .expect("Failed to build canister")
            .success()
        {
            panic!("failed to build canister")
        };
    }

    let result = which::which("pocket-ic").expect(
        "Failed to find pocket-ic server binary. Please run `pixi run install-pocket-ic-server`",
    );
    println!("cargo:rustc-env=POCKET_IC_BIN={}", result.to_str().unwrap());
}
