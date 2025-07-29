use base64::prelude::*;
use dirs;
use fs_err::create_dir_all;
use fs_err::File;
use std::env;
use std::io::{Seek, Write};
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

// Configuration constants
const POCKET_IC_VERSION: &str = "9.0.3";

fn install_dfx_if_needed() {
    // Check if dfx is installed
    if which("dfx").is_err() {
        println!("cargo:warning=dfx not found, installing...");

        // Install dfx using the provided command
        let status = Command::new("sh")
            .args([
                "-ci",
                "$(curl -fsSL https://internetcomputer.org/install.sh)",
            ])
            .env("DFXVM_INIT_YES", "yes")
            .status();

        match status {
            Ok(exit_status) if exit_status.success() => {
                println!("cargo:warning=dfx installed successfully");
            }
            _ => {
                panic!("Failed to install dfx. Please install it manually by running: DFXVM_INIT_YES=yes sh -ci \"$(curl -fsSL https://internetcomputer.org/install.sh)\"\n\nAlternatively, you can install it using the package manager for your system. For example, on Ubuntu/Debian:\n\nsudo apt update\nsudo apt install curl\nDFXVM_INIT_YES=yes sh -ci \"$(curl -fsSL https://internetcomputer.org/install.sh)\"\n\nMake sure you have curl installed before running the installation command.");
            }
        }
    }
}

fn install_pocket_ic_if_needed() {
    // Check if pocket-ic is installed
    if which("pocket-ic").is_err() {
        println!("cargo:warning=pocket-ic not found, installing...");

        // Get the user's home directory
        let home_dir = dirs::home_dir().expect("Failed to get home directory");
        let bin_dir = home_dir.join("bin");

        // Create the bin directory if it doesn't exist
        if !bin_dir.exists() {
            create_dir_all(&bin_dir).expect("Failed to create ~/bin directory");
        }

        // Define the URL for pocket-ic download
        let pocket_ic_url = format!(
            "https://github.com/dfinity/pocketic/releases/download/{}/pocket-ic-x86_64-linux.gz",
            POCKET_IC_VERSION
        );

        // Download the gzipped file
        let response = reqwest::blocking::get(pocket_ic_url).expect("Failed to download pocket-ic");

        if !response.status().is_success() {
            panic!("Failed to download pocket-ic: HTTP {}", response.status());
        }

        // Create a temporary file to store the gzipped content
        let mut temp_file = tempfile::tempfile().expect("Failed to create temporary file");
        let content = response.bytes().expect("Failed to read response bytes");
        temp_file
            .write_all(&content)
            .expect("Failed to write to temporary file");

        // Decompress the gzipped content
        temp_file
            .seek(std::io::SeekFrom::Start(0))
            .expect("Failed to seek in temporary file");
        let mut decoder = flate2::read::GzDecoder::new(temp_file);

        // Write the decompressed binary to the target location
        let pocket_ic_path = bin_dir.join("pocket-ic");
        let mut output_file =
            File::create(&pocket_ic_path).expect("Failed to create pocket-ic binary");
        std::io::copy(&mut decoder, &mut output_file)
            .expect("Failed to decompress and write pocket-ic binary");

        // Make the binary executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs_err::metadata(&pocket_ic_path)
                .expect("Failed to get file metadata")
                .permissions();
            perms.set_mode(0o755);
            fs_err::set_permissions(&pocket_ic_path, perms)
                .expect("Failed to set file permissions");
        }

        println!(
            "cargo:warning=pocket-ic installed successfully to {}",
            pocket_ic_path.display()
        );
    }
}

pub fn main() {
    // Add ~/bin to PATH so that installed tools can be found
    if let Some(home_dir) = dirs::home_dir() {
        let bin_dir = home_dir.join("bin");
        if let Some(path) = std::env::var_os("PATH") {
            let new_path =
                std::env::join_paths(std::iter::once(bin_dir).chain(std::env::split_paths(&path)))
                    .unwrap_or(path);
            std::env::set_var("PATH", new_path);
        }
    }

    prepare_token_logo_env_var();
    install_dfx_if_needed();
    install_pocket_ic_if_needed();
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
