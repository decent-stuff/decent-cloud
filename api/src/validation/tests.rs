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

#[test]
fn test_validate_account_username_valid() {
    // Valid usernames from design spec
    assert_eq!(validate_account_username("alice").unwrap(), "alice");
    assert_eq!(validate_account_username("bob123").unwrap(), "bob123");
    assert_eq!(
        validate_account_username("charlie-delta").unwrap(),
        "charlie-delta"
    );
    assert_eq!(validate_account_username("user_99").unwrap(), "user_99");
    assert_eq!(
        validate_account_username("alice.smith").unwrap(),
        "alice.smith"
    );
    assert_eq!(
        validate_account_username("user@example.com").unwrap(),
        "user@example.com"
    );
    assert_eq!(validate_account_username("dev@org").unwrap(), "dev@org");

    // Uppercase should be preserved
    assert_eq!(validate_account_username("ALICE").unwrap(), "ALICE");
    assert_eq!(validate_account_username("Bob123").unwrap(), "Bob123");
    assert_eq!(validate_account_username("MixedCase").unwrap(), "MixedCase");

    // Whitespace should be trimmed
    assert_eq!(validate_account_username("  alice  ").unwrap(), "alice");
    assert_eq!(validate_account_username("  Alice  ").unwrap(), "Alice");
}

#[test]
fn test_validate_account_username_too_short() {
    assert!(validate_account_username("ab").is_err());
    assert!(validate_account_username("a").is_err());
    assert!(validate_account_username("").is_err());
}

#[test]
fn test_validate_account_username_too_long() {
    let long_username = "a".repeat(65);
    assert!(validate_account_username(&long_username).is_err());
}

#[test]
fn test_validate_account_username_invalid_format() {
    // Cannot start with special character
    assert!(validate_account_username("-alice").is_err());
    assert!(validate_account_username(".alice").is_err());
    assert!(validate_account_username("_alice").is_err());
    assert!(validate_account_username("@alice").is_err());

    // Cannot end with special character
    assert!(validate_account_username("alice-").is_err());
    assert!(validate_account_username("alice.").is_err());
    assert!(validate_account_username("alice_").is_err());
    assert!(validate_account_username("alice@").is_err());

    // Cannot contain invalid characters
    assert!(validate_account_username("alice!bob").is_err());
    assert!(validate_account_username("alice bob").is_err());
    assert!(validate_account_username("alice#bob").is_err());

    // Uppercase should be allowed
    assert!(validate_account_username("Alice").is_ok());
    assert!(validate_account_username("BobSmith").is_ok());
}

#[test]
fn test_validate_account_username_reserved() {
    // All reserved usernames from design spec
    assert!(validate_account_username("admin").is_err());
    assert!(validate_account_username("api").is_err());
    assert!(validate_account_username("system").is_err());
    assert!(validate_account_username("root").is_err());
    assert!(validate_account_username("support").is_err());
    assert!(validate_account_username("moderator").is_err());
    assert!(validate_account_username("administrator").is_err());
    assert!(validate_account_username("test").is_err());
    assert!(validate_account_username("null").is_err());
    assert!(validate_account_username("undefined").is_err());
    assert!(validate_account_username("decent").is_err());
    assert!(validate_account_username("cloud").is_err());

    // Reserved usernames should be case-insensitive
    assert!(validate_account_username("ADMIN").is_err());
    assert!(validate_account_username("Admin").is_err());
}
