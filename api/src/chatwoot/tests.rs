use super::*;
use serial_test::serial;

// =============================================================================
// ChatwootClient (Account API) tests
// =============================================================================

#[test]
#[serial]
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

/// Test that invalid account ID string fails to parse.
#[test]
fn test_account_id_parse_invalid() {
    // Test that non-numeric account IDs fail to parse
    let account_str = "not_a_number";
    let parse_result: Result<u32, _> = account_str.parse();
    assert!(
        parse_result.is_err(),
        "Invalid account ID should fail to parse"
    );
}

#[test]
#[serial]
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
#[serial]
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
#[serial]
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

// =============================================================================
// Article Management tests
// =============================================================================

#[test]
fn test_articles_list_response_deserialize() {
    let json = r#"{
        "payload": [
            {
                "id": 123,
                "title": "Getting Started",
                "slug": "getting-started",
                "content": "Article content here"
            },
            {
                "id": 456,
                "title": "User Guide",
                "slug": "user-guide",
                "content": "Guide content"
            }
        ]
    }"#;

    #[derive(serde::Deserialize)]
    struct ListHelpCenterArticlesResponse {
        payload: Vec<crate::chatwoot::HelpCenterArticle>,
    }

    let response: ListHelpCenterArticlesResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.payload.len(), 2);
    assert_eq!(response.payload[0].id, 123);
    assert_eq!(response.payload[0].title, "Getting Started");
    assert_eq!(response.payload[0].slug, "getting-started");
    assert_eq!(response.payload[1].id, 456);
}

#[test]
fn test_create_article_request_serialize() {
    #[derive(serde::Serialize)]
    struct CreateArticleRequest<'a> {
        title: &'a str,
        slug: &'a str,
        content: &'a str,
        description: &'a str,
        status: i32,
        author_id: i64,
    }

    let request = CreateArticleRequest {
        title: "Test Article",
        slug: "test-article",
        content: "Article content",
        description: "Brief description",
        status: 1,
        author_id: 42,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains(r#""title":"Test Article""#));
    assert!(json.contains(r#""slug":"test-article""#));
    assert!(json.contains(r#""author_id":42"#));
    assert!(json.contains(r#""status":1"#));
}

#[test]
fn test_create_article_response_deserialize() {
    // API wraps response in "payload" field
    let json = r#"{"payload": {"id": 789, "title": "Test", "slug": "test", "content": "..."}}"#;

    #[derive(serde::Deserialize)]
    struct ArticlePayload {
        id: i64,
    }
    #[derive(serde::Deserialize)]
    struct CreateArticleResponse {
        payload: ArticlePayload,
    }

    let response: CreateArticleResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.payload.id, 789);
}

#[test]
fn test_update_article_request_serialize() {
    #[derive(serde::Serialize)]
    struct UpdateArticleRequest<'a> {
        title: &'a str,
        content: &'a str,
        description: &'a str,
        status: i32,
    }

    let request = UpdateArticleRequest {
        title: "Updated Title",
        content: "Updated content",
        description: "Updated description",
        status: 1,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains(r#""title":"Updated Title""#));
    // Should not contain slug field for updates
    assert!(!json.contains("slug"));
}

#[test]
fn test_profile_response_deserialize() {
    let json = r#"{"id": 42, "name": "Test User", "email": "test@example.com"}"#;

    #[derive(serde::Deserialize)]
    struct ProfileResponse {
        id: i64,
    }

    let response: ProfileResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.id, 42);
}
