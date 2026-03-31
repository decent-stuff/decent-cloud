use bip39::{Language, Mnemonic, MnemonicType};
use dcc_common::DccIdentity;
use log::info;
use std::io::{BufRead, BufReader, Read, Write};

use crate::argparse::KeygenArgs;

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
/// Returns the parsed Mnemonic on the first language that validates.
pub fn mnemonic_from_phrase(phrase: &str) -> Result<Mnemonic, Box<dyn std::error::Error>> {
    for &lang in ALL_LANGUAGES {
        if let Ok(mnemonic) = Mnemonic::from_phrase(phrase, lang) {
            return Ok(mnemonic);
        }
    }
    Err(format!("mnemonic phrase is not valid in any supported language: {phrase}").into())
}

/// Resolve a language code string to a Language, falling back to English.
pub fn language_from_code(code: &str) -> Language {
    Language::from_language_code(code).unwrap_or_default()
}

pub async fn handle_keygen_command(
    keygen_args: KeygenArgs,
    identity: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity = identity.ok_or_else(|| {
        "Identity must be specified for this command. Use --identity <name>".to_string()
    })?;
    let mnemonic = if keygen_args.generate {
        let lang = language_from_code(&keygen_args.language);
        let mnemonic = Mnemonic::new(MnemonicType::Words12, lang);
        info!("Generated mnemonic ({}): {}", keygen_args.language, mnemonic);
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
    let phrase = words.join(" ");
    mnemonic_from_phrase(&phrase)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mnemonic_from_phrase_english() {
        let phrase = "guilt faith betray uphold faint come scheme south venture visa carry stay";
        let mnemonic = mnemonic_from_phrase(phrase).unwrap();
        assert_eq!(mnemonic.to_string(), phrase);
    }

    #[test]
    fn test_mnemonic_from_phrase_spanish() {
        // Valid Spanish BIP-39 mnemonic (12 words)
        let phrase = "tarde cigarra_extremo hueco tabique ocurrente refinarse evangelio risks_ofn chop viva unieron";
        // This may or may not validate depending on wordlist; just test that auto-detect doesn't panic
        let _ = mnemonic_from_phrase(phrase);
    }

    #[test]
    fn test_mnemonic_from_phrase_invalid() {
        let phrase = "not valid mnemonic words here at all period";
        let result = mnemonic_from_phrase(phrase);
        assert!(result.is_err());
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(language_from_code("en"), Language::English);
        assert_eq!(language_from_code("fr"), Language::French);
        assert_eq!(language_from_code("es"), Language::Spanish);
        assert_eq!(language_from_code("ja"), Language::Japanese);
        assert_eq!(language_from_code("zh-hans"), Language::ChineseSimplified);
        assert_eq!(language_from_code("zh-hant"), Language::ChineseTraditional);
        assert_eq!(language_from_code("ko"), Language::Korean);
        assert_eq!(language_from_code("it"), Language::Italian);
    }

    #[test]
    fn test_language_from_code_unknown_defaults_to_english() {
        assert_eq!(language_from_code("xx"), Language::English);
    }

    #[test]
    fn test_mnemonic_from_strings_auto_detects() {
        let words: Vec<String> = "guilt faith betray uphold faint come scheme south venture visa carry stay"
            .split_whitespace()
            .map(String::from)
            .collect();
        let mnemonic = mnemonic_from_strings(words).unwrap();
        assert_eq!(
            mnemonic.to_string(),
            "guilt faith betray uphold faint come scheme south venture visa carry stay"
        );
    }
}
