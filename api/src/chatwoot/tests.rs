use super::*;

// =============================================================================
// ChatwootClient (Account API) tests
// =============================================================================

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

// =============================================================================
// ChatwootPlatformClient (Platform API) tests
// =============================================================================

#[test]
fn test_platform_client_from_env_missing_vars() {
    std::env::remove_var("CHATWOOT_PLATFORM_API_TOKEN");
    std::env::remove_var("CHATWOOT_BASE_URL");
    std::env::remove_var("CHATWOOT_ACCOUNT_ID");

    let result = ChatwootPlatformClient::from_env();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("CHATWOOT_BASE_URL"));
}

#[test]
fn test_platform_client_from_env_valid() {
    std::env::set_var("CHATWOOT_BASE_URL", "https://test.chatwoot.com");
    std::env::set_var("CHATWOOT_PLATFORM_API_TOKEN", "platform_token");
    std::env::set_var("CHATWOOT_ACCOUNT_ID", "1");

    let result = ChatwootPlatformClient::from_env();
    assert!(result.is_ok());

    let client = result.unwrap();
    assert_eq!(
        format!("{:?}", client),
        "ChatwootPlatformClient { base_url: \"https://test.chatwoot.com\", account_id: 1 }"
    );

    // Clean up
    std::env::remove_var("CHATWOOT_BASE_URL");
    std::env::remove_var("CHATWOOT_PLATFORM_API_TOKEN");
    std::env::remove_var("CHATWOOT_ACCOUNT_ID");
}

#[test]
fn test_platform_client_is_configured() {
    // Clear all
    std::env::remove_var("CHATWOOT_PLATFORM_API_TOKEN");
    std::env::remove_var("CHATWOOT_BASE_URL");
    std::env::remove_var("CHATWOOT_ACCOUNT_ID");

    assert!(!ChatwootPlatformClient::is_configured());

    // Set all required
    std::env::set_var("CHATWOOT_BASE_URL", "https://test.chatwoot.com");
    std::env::set_var("CHATWOOT_PLATFORM_API_TOKEN", "token");
    std::env::set_var("CHATWOOT_ACCOUNT_ID", "1");

    assert!(ChatwootPlatformClient::is_configured());

    // Clean up
    std::env::remove_var("CHATWOOT_BASE_URL");
    std::env::remove_var("CHATWOOT_PLATFORM_API_TOKEN");
    std::env::remove_var("CHATWOOT_ACCOUNT_ID");
}

// =============================================================================
// Portals tests
// =============================================================================

#[test]
fn test_portals_response_deserialize() {
    let json = r#"{
        "payload": [
            {
                "id": 1,
                "slug": "platform-overview",
                "archived": false,
                "name": "Platform Overview"
            },
            {
                "id": 2,
                "slug": "old-portal",
                "archived": true,
                "name": "Old Portal"
            }
        ]
    }"#;

    #[derive(serde::Deserialize)]
    struct PortalsResponse {
        payload: Vec<Portal>,
    }

    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Portal {
        slug: String,
        archived: bool,
    }

    let response: PortalsResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.payload.len(), 2);
    assert_eq!(response.payload[0].slug, "platform-overview");
    assert!(!response.payload[0].archived);
    assert_eq!(response.payload[1].slug, "old-portal");
    assert!(response.payload[1].archived);
}

#[test]
fn test_portals_response_empty() {
    let json = r#"{"payload": []}"#;

    #[derive(serde::Deserialize)]
    struct PortalsResponse {
        payload: Vec<serde_json::Value>,
    }

    let response: PortalsResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.payload.len(), 0);
}

// =============================================================================
// Help Center tests
// =============================================================================

#[test]
fn test_help_center_article_deserialize() {
    let json = r#"{
        "id": 42,
        "title": "How to get started",
        "content": "<p>Getting started is easy!</p>",
        "slug": "how-to-get-started"
    }"#;

    let article: crate::chatwoot::HelpCenterArticle = serde_json::from_str(json).unwrap();
    assert_eq!(article.id, 42);
    assert_eq!(article.title, "How to get started");
    assert_eq!(article.content, "<p>Getting started is easy!</p>");
    assert_eq!(article.slug, "how-to-get-started");
}

#[test]
fn test_help_center_article_serialize() {
    let article = crate::chatwoot::HelpCenterArticle {
        id: 123,
        title: "Test Article".to_string(),
        content: "<h1>Content</h1>".to_string(),
        slug: "test-article".to_string(),
    };

    let json = serde_json::to_string(&article).unwrap();
    assert!(json.contains(r#""id":123"#));
    assert!(json.contains(r#""title":"Test Article""#));
    assert!(json.contains(r#""slug":"test-article""#));
}

#[test]
fn test_help_center_article_clone_and_eq() {
    let article1 = crate::chatwoot::HelpCenterArticle {
        id: 1,
        title: "Article".to_string(),
        content: "Content".to_string(),
        slug: "article".to_string(),
    };

    let article2 = article1.clone();
    assert_eq!(article1, article2);

    let article3 = crate::chatwoot::HelpCenterArticle {
        id: 2,
        title: "Article".to_string(),
        content: "Content".to_string(),
        slug: "article".to_string(),
    };
    assert_ne!(article1, article3);
}
