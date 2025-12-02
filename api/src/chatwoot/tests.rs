use super::*;

#[test]
fn test_chatwoot_client_from_env_missing_vars() {
    // Clear env vars
    std::env::remove_var("CHATWOOT_BASE_URL");
    std::env::remove_var("CHATWOOT_API_TOKEN");
    std::env::remove_var("CHATWOOT_ACCOUNT_ID");

    let result = ChatwootClient::from_env();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("CHATWOOT_BASE_URL"));
}

#[test]
fn test_chatwoot_client_from_env_invalid_account_id() {
    std::env::set_var("CHATWOOT_BASE_URL", "https://test.chatwoot.com");
    std::env::set_var("CHATWOOT_API_TOKEN", "test_token");
    std::env::set_var("CHATWOOT_ACCOUNT_ID", "not_a_number");

    let result = ChatwootClient::from_env();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a number"));

    // Clean up
    std::env::remove_var("CHATWOOT_BASE_URL");
    std::env::remove_var("CHATWOOT_API_TOKEN");
    std::env::remove_var("CHATWOOT_ACCOUNT_ID");
}

#[test]
fn test_chatwoot_client_from_env_valid() {
    std::env::set_var("CHATWOOT_BASE_URL", "https://test.chatwoot.com");
    std::env::set_var("CHATWOOT_API_TOKEN", "test_token");
    std::env::set_var("CHATWOOT_ACCOUNT_ID", "1");

    let result = ChatwootClient::from_env();
    assert!(result.is_ok());

    let client = result.unwrap();
    assert_eq!(
        format!("{:?}", client),
        "ChatwootClient { base_url: \"https://test.chatwoot.com\", account_id: 1 }"
    );

    // Clean up
    std::env::remove_var("CHATWOOT_BASE_URL");
    std::env::remove_var("CHATWOOT_API_TOKEN");
    std::env::remove_var("CHATWOOT_ACCOUNT_ID");
}
