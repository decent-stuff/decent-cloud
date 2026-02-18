use super::*;
use client::{ChatwootClient, ChatwootPlatformClient};
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

// =============================================================================
// HTTP mock tests - ChatwootPlatformClient
// =============================================================================

#[tokio::test]
async fn test_create_user_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/platform/api/v1/users")
        .match_header("api_access_token", "test-platform-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 77, "email": "new@example.com"}"#)
        .create_async()
        .await;

    let client =
        ChatwootPlatformClient::new_for_test(server.url(), "test-platform-token".into(), 1);
    let user = client
        .create_user("new@example.com", "New User", "s3cret")
        .await
        .unwrap();

    assert_eq!(user.id, 77);
    assert_eq!(user.email, "new@example.com");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_create_user_api_error() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/platform/api/v1/users")
        .with_status(422)
        .with_body(r#"{"error": "Email has already been taken"}"#)
        .create_async()
        .await;

    let client =
        ChatwootPlatformClient::new_for_test(server.url(), "test-platform-token".into(), 1);
    let err = client
        .create_user("dup@example.com", "Dup", "pass")
        .await
        .unwrap_err();

    assert!(err.to_string().contains("422"), "Expected 422 in error: {err}");
    assert!(
        err.to_string().contains("Email has already been taken"),
        "Expected body in error: {err}"
    );
    mock.assert_async().await;
}

#[tokio::test]
async fn test_configure_agent_bot_creates_new() {
    let mut server = mockito::Server::new_async().await;

    // GET returns empty list (no existing bot)
    let list_mock = server
        .mock("GET", "/platform/api/v1/agent_bots")
        .match_header("api_access_token", "plat-tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    // POST creates the bot
    let create_mock = server
        .mock("POST", "/platform/api/v1/agent_bots")
        .match_header("api_access_token", "plat-tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 5, "name": "dc-bot", "account_id": 1}"#)
        .create_async()
        .await;

    let client = ChatwootPlatformClient::new_for_test(server.url(), "plat-tok".into(), 1);
    let bot_id = client
        .configure_agent_bot("dc-bot", "https://example.com/webhook")
        .await
        .unwrap();

    assert_eq!(bot_id, 5);
    list_mock.assert_async().await;
    create_mock.assert_async().await;
}

#[tokio::test]
async fn test_configure_agent_bot_updates_existing() {
    let mut server = mockito::Server::new_async().await;

    // GET returns a matching bot for this account
    let list_mock = server
        .mock("GET", "/platform/api/v1/agent_bots")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"id": 9, "name": "dc-bot", "account_id": 1}]"#)
        .create_async()
        .await;

    // PATCH updates the bot
    let update_mock = server
        .mock("PATCH", "/platform/api/v1/agent_bots/9")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 9, "name": "dc-bot", "account_id": 1}"#)
        .create_async()
        .await;

    let client = ChatwootPlatformClient::new_for_test(server.url(), "plat-tok".into(), 1);
    let bot_id = client
        .configure_agent_bot("dc-bot", "https://new-url.com/webhook")
        .await
        .unwrap();

    assert_eq!(bot_id, 9);
    list_mock.assert_async().await;
    update_mock.assert_async().await;
}

// =============================================================================
// HTTP mock tests - ChatwootClient
// =============================================================================

#[tokio::test]
async fn test_list_inboxes_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/accounts/1/inboxes")
        .match_header("api_access_token", "tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"payload": [{"id": 10}, {"id": 20}]}"#)
        .create_async()
        .await;

    let client = ChatwootClient::new_for_test(server.url(), "tok".into(), 1);
    let ids = client.list_inboxes().await.unwrap();

    assert_eq!(ids, vec![10, 20]);
    mock.assert_async().await;
}

#[tokio::test]
async fn test_list_inboxes_api_error() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/accounts/1/inboxes")
        .with_status(500)
        .with_body("Internal Server Error")
        .create_async()
        .await;

    let client = ChatwootClient::new_for_test(server.url(), "tok".into(), 1);
    let err = client.list_inboxes().await.unwrap_err();

    assert!(err.to_string().contains("500"), "Expected 500 in: {err}");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_find_or_create_inbox_existing() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/accounts/1/inboxes")
        .match_header("api_access_token", "tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"payload": [
                {"id": 3, "name": "Other", "channel_type": "api"},
                {"id": 7, "name": "Provider-X", "channel_type": "api"}
            ]}"#,
        )
        .create_async()
        .await;

    let client = ChatwootClient::new_for_test(server.url(), "tok".into(), 1);
    let inbox = client.find_or_create_inbox("Provider-X").await.unwrap();

    assert_eq!(inbox.id, 7);
    assert_eq!(inbox.name, "Provider-X");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_send_message_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/api/v1/accounts/1/conversations/42/messages")
        .match_header("api_access_token", "tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 999}"#)
        .create_async()
        .await;

    let client = ChatwootClient::new_for_test(server.url(), "tok".into(), 1);
    client
        .send_message(42, "Hello from test")
        .await
        .unwrap();

    mock.assert_async().await;
}

#[tokio::test]
async fn test_fetch_conversation_messages_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/accounts/1/conversations/10/messages")
        .match_header("api_access_token", "tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"payload": [
                {"content": "Hi there", "message_type": 0},
                {"content": "Hello!", "message_type": 1},
                {"content": null, "message_type": 0},
                {"content": "  ", "message_type": 1}
            ]}"#,
        )
        .create_async()
        .await;

    let client = ChatwootClient::new_for_test(server.url(), "tok".into(), 1);
    let messages = client.fetch_conversation_messages(10).await.unwrap();

    // null and whitespace-only messages are filtered out
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], ("customer".to_string(), "Hi there".to_string()));
    assert_eq!(messages[1], ("bot".to_string(), "Hello!".to_string()));
    mock.assert_async().await;
}

#[tokio::test]
async fn test_list_articles_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/accounts/1/portals/my-portal/articles")
        .match_header("api_access_token", "tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"payload": [
                {"id": 1, "title": "First", "content": "<p>One</p>", "slug": "first"},
                {"id": 2, "title": "Second", "content": "<p>Two</p>", "slug": "second"}
            ]}"#,
        )
        .create_async()
        .await;

    let client = ChatwootClient::new_for_test(server.url(), "tok".into(), 1);
    let articles = client.list_articles("my-portal").await.unwrap();

    assert_eq!(articles.len(), 2);
    assert_eq!(articles[0].id, 1);
    assert_eq!(articles[0].title, "First");
    assert_eq!(articles[0].slug, "first");
    assert_eq!(articles[1].id, 2);
    mock.assert_async().await;
}

#[tokio::test]
async fn test_list_portals_filters_archived() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/accounts/1/portals")
        .match_header("api_access_token", "tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"payload": [
                {"slug": "active-portal", "archived": false},
                {"slug": "dead-portal", "archived": true}
            ]}"#,
        )
        .create_async()
        .await;

    let client = ChatwootClient::new_for_test(server.url(), "tok".into(), 1);
    let slugs = client.list_portals().await.unwrap();

    assert_eq!(slugs, vec!["active-portal"]);
    mock.assert_async().await;
}

#[tokio::test]
async fn test_update_conversation_status_success() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock(
            "POST",
            "/api/v1/accounts/1/conversations/55/toggle_status",
        )
        .match_header("api_access_token", "tok")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status": "open"}"#)
        .create_async()
        .await;

    let client = ChatwootClient::new_for_test(server.url(), "tok".into(), 1);
    client
        .update_conversation_status(55, "open")
        .await
        .unwrap();

    mock.assert_async().await;
}
