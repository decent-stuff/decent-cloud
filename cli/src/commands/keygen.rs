use bip39::{Language, Mnemonic, MnemonicType};
use dcc_common::DccIdentity;
use log::info;
use std::io::{BufRead, BufReader, Read, Write};

use crate::argparse::KeygenArgs;

pub async fn handle_keygen_command(
    keygen_args: KeygenArgs,
    identity: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity = identity.ok_or_else(|| {
        "Identity must be specified for this command. Use --identity <name>".to_string()
    })?;
    let mnemonic = if keygen_args.generate {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        info!("Generated mnemonic: {}", mnemonic);
        mnemonic
    } else if keygen_args.mnemonic.is_some() {
        let mnemonic_string = keygen_args
            .mnemonic
            .clone()
            .unwrap_or_default()
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();
        if mnemonic_string.len() < 12 {
            let reader = BufReader::new(std::io::stdin());
            mnemonic_from_stdin(reader, std::io::stdout())?
        } else {
            mnemonic_from_strings(mnemonic_string)?
        }
    } else {
        return Err(
            "Missing mnemonic source: specify --mnemonic '<words>' to use an existing mnemonic phrase or --generate to create a new one".into(),
        );
    };

    let seed = bip39::Seed::new(&mnemonic, "");
    let dcc_identity = DccIdentity::new_from_seed(seed.as_bytes())?;
    info!("Generated identity: {}", dcc_identity);
    dcc_identity.save_to_dir(&identity)?;
    Ok(())
}

pub fn mnemonic_from_strings(words: Vec<String>) -> Result<Mnemonic, Box<dyn std::error::Error>> {
    let mnemonic_string = words.join(" ");
    Ok(Mnemonic::from_phrase(&mnemonic_string, Language::English)?)
}

pub fn mnemonic_from_stdin<R: Read, W: Write>(
    mut reader: BufReader<R>,
    mut writer: W,
) -> Result<Mnemonic, Box<dyn std::error::Error>> {
    let mut words = Vec::new();
    writeln!(
        writer,
        "Please enter your mnemonic phrase (12 words, one per line):"
    )?;
    for i in 0..12 {
        write!(writer, "Word {}: ", i + 1)?;
        writer.flush()?;
        let mut word = String::new();
        reader.read_line(&mut word)?;
        words.push(word.trim().to_string());
    }
    mnemonic_from_strings(words)
}
