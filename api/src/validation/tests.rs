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
    assert!(validate_public_key("ssh-ed25519", "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI...").is_ok());
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
