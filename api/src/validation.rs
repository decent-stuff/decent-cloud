use anyhow::{bail, Result};
use regex::Regex;
use std::sync::OnceLock;

static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
static URL_REGEX: OnceLock<Regex> = OnceLock::new();

fn email_regex() -> &'static Regex {
    EMAIL_REGEX.get_or_init(|| {
        Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
    })
}

fn url_regex() -> &'static Regex {
    URL_REGEX.get_or_init(|| {
        Regex::new(r"^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/.*)?$").unwrap()
    })
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

pub fn validate_url(url: &str) -> Result<()> {
    if url.trim().is_empty() {
        bail!("URL cannot be empty");
    }
    if url.len() > 2048 {
        bail!("URL is too long (max 2048 characters)");
    }
    if !url_regex().is_match(url) {
        bail!("Invalid URL format (must start with http:// or https://)");
    }
    Ok(())
}

pub fn validate_public_key(key_type: &str, key_data: &str) -> Result<()> {
    if key_data.trim().is_empty() {
        bail!("Public key data cannot be empty");
    }
    if key_data.len() > 10000 {
        bail!("Public key is too long (max 10000 characters)");
    }

    match key_type {
        "ssh-ed25519" | "ssh-rsa" => {
            let parts: Vec<&str> = key_data.split_whitespace().collect();
            if parts.is_empty() {
                bail!("Invalid SSH key format");
            }
            if !parts[0].starts_with("ssh-") {
                bail!("SSH key must start with key type (e.g., ssh-ed25519, ssh-rsa)");
            }
        }
        "gpg" => {
            if !key_data.contains("BEGIN PGP PUBLIC KEY BLOCK") {
                bail!("GPG key must contain PGP public key block");
            }
        }
        _ => {
            // For other key types, just check it's not empty (already done above)
        }
    }
    Ok(())
}

pub fn validate_contact_type(contact_type: &str) -> Result<()> {
    const VALID_TYPES: &[&str] = &["email", "phone", "telegram", "discord", "signal"];
    if !VALID_TYPES.contains(&contact_type) {
        bail!(
            "Invalid contact type. Must be one of: {}",
            VALID_TYPES.join(", ")
        );
    }
    Ok(())
}

pub fn validate_contact_value(contact_type: &str, contact_value: &str) -> Result<()> {
    if contact_value.trim().is_empty() {
        bail!("Contact value cannot be empty");
    }
    if contact_value.len() > 255 {
        bail!("Contact value is too long (max 255 characters)");
    }

    match contact_type {
        "email" => validate_email(contact_value)?,
        "phone" => {
            // Basic phone validation - allow digits, spaces, +, -, ()
            if !contact_value
                .chars()
                .all(|c| c.is_ascii_digit() || " +()-".contains(c))
            {
                bail!("Invalid phone number format");
            }
            let digit_count = contact_value.chars().filter(|c| c.is_ascii_digit()).count();
            if !(7..=15).contains(&digit_count) {
                bail!("Phone number must have 7-15 digits");
            }
        }
        _ => {
            // For telegram, discord, signal - just check length and non-empty (already done)
        }
    }
    Ok(())
}

pub fn validate_social_platform(platform: &str) -> Result<()> {
    const VALID_PLATFORMS: &[&str] = &["twitter", "github", "discord", "linkedin", "reddit"];
    if !VALID_PLATFORMS.contains(&platform) {
        bail!(
            "Invalid social platform. Must be one of: {}",
            VALID_PLATFORMS.join(", ")
        );
    }
    Ok(())
}

pub fn validate_social_username(username: &str) -> Result<()> {
    if username.trim().is_empty() {
        bail!("Username cannot be empty");
    }
    if username.len() > 100 {
        bail!("Username is too long (max 100 characters)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user+tag@sub.example.com").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://sub.example.com/path").is_ok());
        assert!(validate_url("invalid").is_err());
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("").is_err());
    }

    #[test]
    fn test_validate_public_key() {
        assert!(
            validate_public_key("ssh-ed25519", "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI...").is_ok()
        );
        assert!(validate_public_key("ssh-rsa", "ssh-rsa AAAAB3NzaC1yc2EA...").is_ok());
        assert!(validate_public_key("ssh-ed25519", "invalid").is_err());
        assert!(validate_public_key("ssh-ed25519", "").is_err());
    }

    #[test]
    fn test_validate_contact_value() {
        assert!(validate_contact_value("email", "user@example.com").is_ok());
        assert!(validate_contact_value("phone", "+1 (555) 123-4567").is_ok());
        assert!(validate_contact_value("phone", "5551234567").is_ok());
        assert!(validate_contact_value("telegram", "@username").is_ok());
        assert!(validate_contact_value("email", "invalid").is_err());
        assert!(validate_contact_value("phone", "abc").is_err());
        assert!(validate_contact_value("phone", "123").is_err()); // too short
    }

    #[test]
    fn test_validate_social_username() {
        assert!(validate_social_username("validuser").is_ok());
        assert!(validate_social_username("user_123").is_ok());
        assert!(validate_social_username("").is_err());
    }
}
