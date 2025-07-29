use base64::prelude::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

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

fn prepare_token_logo_env_var() {
    let input_path_svg = Path::new("token-logo.svg");

    if input_path_svg.exists() {
        // Read and encode the SVG file.
        let svg_content =
            fs_err::read_to_string(input_path_svg).expect("Failed to read the SVG file");
        let base64_encoded = BASE64_STANDARD.encode(svg_content);

        println!(
            "cargo:rustc-env=DC_TOKEN_LOGO_BASE64=data:image/svg+xml;base64,{}",
            base64_encoded
        );
        return;
    } else {
        let input_path_png = Path::new("token-logo.png");

        if input_path_png.exists() {
            // Read and encode the PNG file.
            let png_content = fs_err::read(input_path_png).expect("Failed to read the PNG file");
            let base64_encoded = BASE64_STANDARD.encode(png_content);
            println!(
                "cargo:rustc-env=DC_TOKEN_LOGO_BASE64=data:image/png;base64,{}",
                base64_encoded
            );
            return;
        }
    }
    panic!("Failed to find token-logo.svg or token-logo.png");
}


fn install_dfx_if_needed() {
    // Check if dfx is installed
    if which("dfx").is_err() {
        println!("cargo:warning=dfx not found, installing...");
        
        // Install dfx using the provided command
        let status = Command::new("sh")
            .args(["-ci", "$(curl -fsSL https://internetcomputer.org/install.sh)"])
            .env("DFXVM_INIT_YES", "yes")
            .status();
            
        match status {
            Ok(exit_status) if exit_status.success() => {
                println!("cargo:warning=dfx installed successfully");
            }
            _ => {
                panic!("Failed to install dfx. Please install it manually by running: DFXVM_INIT_YES=yes sh -ci \"$(curl -fsSL https://internetcomputer.org/install.sh)\"");
            }
        }
    }
}

pub fn main() {
    prepare_token_logo_env_var();
    install_dfx_if_needed();
    // Only build the canister on x86_64 Linux
    if Ok("linux") != env::var("CARGO_CFG_TARGET_OS").as_deref() {
        return;
    }

    if Ok("x86_64") != env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        return;
    }

    if matches!(env::var("PROFILE").as_deref(), Ok("release")) {
        return;
    }

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
        .args(["build"])
        .current_dir(canister_dir_path)
        .status()
        .expect("Failed to build canister")
        .success()
    {
        panic!("failed to build canister")
    };

    let result = which::which("pocket-ic").expect(
        "Failed to find pocket-ic server binary. Please run `pixi run install-pocket-ic-server`",
    );
    println!("cargo:rustc-env=POCKET_IC_BIN={}", result.to_str().unwrap());
}
