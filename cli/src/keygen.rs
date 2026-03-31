use bip39::{Language, Mnemonic, MnemonicType, Seed};
use dcc_common::DccIdentity;
use ed25519_dalek::Signature;
use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};

/// All supported BIP-39 languages for auto-detection.
const ALL_LANGUAGES: &[Language] = &[
    Language::English,
    Language::ChineseSimplified,
    Language::ChineseTraditional,
    Language::French,
    Language::Italian,
    Language::Japanese,
    Language::Korean,
    Language::Spanish,
];

/// Try to parse a mnemonic phrase by testing all supported languages.
fn detect_mnemonic(phrase: &str) -> Result<Mnemonic, Box<dyn Error>> {
    for &lang in ALL_LANGUAGES {
        if let Ok(mnemonic) = Mnemonic::from_phrase(phrase, lang) {
            return Ok(mnemonic);
        }
    }
    Err(format!("mnemonic phrase is not valid in any supported language: {phrase}").into())
}

#[allow(dead_code)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BufReader::new(io::stdin());
    let mnemonic = mnemonic_from_stdin(reader, io::stdout())?;
    let seed = Seed::new(&mnemonic, "");

    let dcc_id = DccIdentity::new_from_seed(seed.as_bytes())?;

    // Sign a message
    let message: &[u8] = b"Hello, world!";
    let signature: Signature = dcc_id.sign(message)?;

    let verifying_key = dcc_id.verifying_key();
    println!("DccIdentity: {:?}", dcc_id);
    println!("Public Key: {:?}", verifying_key.to_bytes());
    println!("Signature: {:?}", signature);

    match dcc_id.verify(message, &signature) {
        Ok(()) => println!("Signature is valid."),
        Err(e) => println!("Signature is invalid: {:#}", e),
    }

    Ok(())
}

pub fn mnemonic_from_strings(input_phrase: Vec<String>) -> Result<Mnemonic, Box<dyn Error>> {
    match input_phrase.len() {
        12 | 15 | 18 | 21 | 24 => {}
        _ => {
            return Err(
                format!("mnemonic must be 12-24 words, got {:?}", input_phrase.len()).into(),
            )
        }
    }
    let phrase = input_phrase.join(" ");
    let phrase = phrase.trim();
    detect_mnemonic(phrase).map_err(Into::into)
}

pub fn mnemonic_from_stdin<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
) -> Result<Mnemonic, Box<dyn Error>> {
    let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
    let mnemonic = loop {
        write!(writer, "Please enter mnemonic [{}]: ", mnemonic)?;
        writer.flush()?;
        let mut input = String::new();
        reader.read_line(&mut input)?;
        let input = input.trim();
        if input.is_empty() {
            break mnemonic;
        } else {
            match detect_mnemonic(input) {
                Ok(m) => break m,
                Err(_) => writeln!(writer, "Invalid mnemonic in all supported languages")?,
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
        let mock_input = "\n";
        let reader = Cursor::new(mock_input);
        let writer = Cursor::new(Vec::new());

        let mnemonic = mnemonic_from_stdin(reader, writer)?;
        assert_eq!(mnemonic.to_string().split_whitespace().count(), 12);
        Ok(())
    }

    #[test]
    fn test_get_mnemonic_with_provided_input() -> Result<(), Box<dyn Error>> {
        let mock_input =
            "guilt faith betray uphold faint come scheme south venture visa carry stay\n";
        let reader = Cursor::new(mock_input);
        let writer = Cursor::new(Vec::new());

        let mnemonic = mnemonic_from_stdin(reader, writer)?;
        assert_eq!(mnemonic.to_string().split_whitespace().count(), 12);
        assert_eq!(
            mnemonic.to_string(),
            "guilt faith betray uphold faint come scheme south venture visa carry stay"
        );
        Ok(())
    }

    #[test]
    fn test_detect_mnemonic_english() {
        let phrase = "guilt faith betray uphold faint come scheme south venture visa carry stay";
        let mnemonic = detect_mnemonic(phrase).unwrap();
        assert_eq!(mnemonic.to_string(), phrase);
    }

    #[test]
    fn test_detect_mnemonic_invalid() {
        let phrase = "not valid mnemonic words here at all period done";
        let result = detect_mnemonic(phrase);
        assert!(result.is_err());
    }

    #[test]
    fn test_mnemonic_from_strings_auto_detects() -> Result<(), Box<dyn Error>> {
        let words: Vec<String> =
            "guilt faith betray uphold faint come scheme south venture visa carry stay"
                .split_whitespace()
                .map(String::from)
                .collect();
        let mnemonic = mnemonic_from_strings(words)?;
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
        let expected_seed: [u8; 32] = seed_bip39.as_bytes()[..32]
            .try_into()
            .expect("BIP39 seed should be at least 32 bytes");
        assert_eq!(seed.as_bytes()[..32], expected_seed);
    }

    #[test]
    fn test_key_pair_generation() {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let seed = Seed::new(&mnemonic, "");
        let signing_key =
            DccIdentity::generate_ed25519_signing_key_from_seed(seed.as_bytes()).unwrap();
        let verifying_key = signing_key.verifying_key();
        assert!(!verifying_key.to_bytes().is_empty());
        assert!(!signing_key.to_bytes().is_empty());
    }

    #[test]
    fn test_message_signing_and_verification() {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let seed = Seed::new(&mnemonic, "");
        let signing_key =
            DccIdentity::generate_ed25519_signing_key_from_seed(seed.as_bytes()).unwrap();
        let verifying_key = signing_key.verifying_key();

        let message: &[u8] = b"Test Message";
        let signature = signing_key.sign(message);
        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_prehashed_signing_and_verification() {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        let seed = Seed::new(&mnemonic, "");
        let signing_key =
            DccIdentity::generate_ed25519_signing_key_from_seed(seed.as_bytes()).unwrap();
        let verifying_key = signing_key.verifying_key();

        let message: &[u8] = b"Test Message";
        let mut prehashed = Sha512::new();
        prehashed.update(message);
        let signature = signing_key
            .sign_prehashed(prehashed.clone(), Some(ED25519_SIGN_CONTEXT))
            .expect("Prehashed signing should not fail with valid context");
        assert!(verifying_key
            .verify_prehashed(prehashed, Some(ED25519_SIGN_CONTEXT), &signature)
            .is_ok());
    }
}
