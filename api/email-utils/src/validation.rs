use anyhow::{bail, Result};
use regex::Regex;
use std::sync::OnceLock;

static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();

fn email_regex() -> &'static Regex {
    EMAIL_REGEX
        .get_or_init(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap())
}

pub fn validate_email(email: &str) -> Result<()> {
    if email.trim().is_empty() {
        bail!("Email cannot be empty");
    }
    if email.len() > 255 {
        bail!("Email is too long (max 255 characters)");
    }
    if !email_regex().is_match(email) {
        bail!("Invalid email format");
    }
    Ok(())
}
