use bip39::{Language, Mnemonic, MnemonicType, Seed};
use dcc_common::DccIdentity;
use ed25519_dalek::Signature;
use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};

#[allow(dead_code)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(io::stdin());
    let mnemonic = mnemonic_from_stdin(reader, io::stdout())?;
    let seed = Seed::new(&mnemonic, "");

    let dcc_identity = DccIdentity::new_from_seed(seed.as_bytes())?;

    // Sign a message
    let message: &[u8] = b"Hello, world!";
    let signature: Signature = dcc_identity.sign(message)?;

    let verifying_key = dcc_identity.verifying_key();
    println!("DccIdentity: {:?}", dcc_identity);
    println!("Public Key: {:?}", verifying_key.to_bytes());
    println!("Signature: {:?}", signature);

    match dcc_identity.verify(message, &signature) {
        Ok(()) => println!("Signature is valid."),
        Err(e) => println!("Signature is invalid: {}", e),
    }

    Ok(())
}

pub fn mnemonic_from_strings(input_phrase: Vec<String>) -> Result<Mnemonic, Box<dyn Error>> {
    match input_phrase.len() {
        12 => bip39::MnemonicType::Words12,
        24 => bip39::MnemonicType::Words24,
        _ => return Err(format!("mnemonic must be 12 or 24 words, got {:?}", input_phrase).into()),
    };
    let input_phrase = input_phrase.join(" ");
    let input_phrase = input_phrase.trim();
    // TODO: Add more languages
    Mnemonic::from_phrase(input_phrase, Language::English).map_err(Into::into)
}

pub fn mnemonic_from_stdin<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
) -> Result<Mnemonic, Box<dyn Error>> {
    let lang = Language::English;
    let mnemonic = Mnemonic::new(MnemonicType::Words12, lang);
    let mnemonic = loop {
        write!(writer, "Please enter mnemonic [{}]: ", mnemonic)?;
        writer.flush()?;
        let mut input = String::new();
        reader.read_line(&mut input)?;
        let input = input.trim();
        if input.is_empty() {
            break mnemonic;
        } else {
            match Mnemonic::validate(input, lang) {
                Ok(_) => break Mnemonic::from_phrase(input, lang)?,
                Err(err) => writeln!(writer, "{}", err)?,
            }
        }
    };
    writeln!(writer, "Mnemonic: {}", mnemonic)?;
    Ok(mnemonic)
}

#[cfg(test)]
mod tests {
    use dcc_common::ED25519_SIGN_CONTEXT;

    use super::*;
    use ed25519_dalek::{Digest, Sha512, Signer, Verifier};
    use std::io::Cursor;

    #[test]
    fn test_get_mnemonic_with_mock_input() -> Result<(), Box<dyn Error>> {
        let mock_input = "\n"; // Simulates the user just pressing Enter
        let mock_output = Vec::new();
        let reader = Cursor::new(mock_input);
        let writer = Cursor::new(mock_output);

        let mnemonic = mnemonic_from_stdin(reader, writer)?;

        // Validate the mnemonic
        assert!(mnemonic.to_string().split_whitespace().count() == 12);

        Ok(())
    }

    #[test]
    fn test_get_mnemonic_with_provided_input() -> Result<(), Box<dyn Error>> {
        let mock_input =
            "guilt faith betray uphold faint come scheme south venture visa carry stay\n";
        let mock_output = Vec::new();
        let reader = Cursor::new(mock_input);
        let writer = Cursor::new(mock_output);

        let mnemonic = mnemonic_from_stdin(reader, writer)?;

        // Validate the mnemonic
        assert!(mnemonic.to_string().split_whitespace().count() == 12);
        assert_eq!(
            mnemonic.to_string(),
            "guilt faith betray uphold faint come scheme south venture visa carry stay"
        );

        Ok(())
    }

    #[test]
    fn test_seed_from_mnemonic() {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let seed = Seed::new(&mnemonic, "");
        let seed_bip39 = bip39::Seed::new(&mnemonic, "");
        let expected_seed: [u8; 32] = seed_bip39.as_bytes()[..32].try_into().unwrap();
        assert_eq!(seed.as_bytes()[..32], expected_seed);
    }

    #[test]
    fn test_key_pair_generation() {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let seed = Seed::new(&mnemonic, "");
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(seed.as_bytes());
        let verifying_key = signing_key.verifying_key();
        assert!(!verifying_key.to_bytes().is_empty());
        assert!(!signing_key.to_bytes().is_empty());
    }

    #[test]
    fn test_message_signing_and_verification() {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let seed = Seed::new(&mnemonic, "");
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(seed.as_bytes());
        let verifying_key = signing_key.verifying_key();

        let message: &[u8] = b"Test Message";
        let signature = signing_key.sign(message);
        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_prehashed_signing_and_verification() {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let seed = Seed::new(&mnemonic, "");
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(seed.as_bytes());
        let verifying_key = signing_key.verifying_key();

        let message: &[u8] = b"Test Message";
        let mut prehashed = Sha512::new();
        prehashed.update(message);
        let signature = signing_key
            .sign_prehashed(prehashed.clone(), Some(ED25519_SIGN_CONTEXT))
            .unwrap();
        assert!(verifying_key
            .verify_prehashed(prehashed, Some(ED25519_SIGN_CONTEXT), &signature)
            .is_ok());
    }
}
