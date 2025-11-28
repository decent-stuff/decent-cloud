use super::*;
use crate::database::email::EmailQueueEntry;

#[test]
fn test_parse_email_address_with_name() {
    let (email, name) = parse_email_address("Test User <test@example.com>").unwrap();
    assert_eq!(email, "test@example.com");
    assert_eq!(name, "Test User");
}

#[test]
fn test_parse_email_address_without_name() {
    let (email, name) = parse_email_address("test@example.com").unwrap();
    assert_eq!(email, "test@example.com");
    assert_eq!(name, "test@example.com");
}

#[test]
fn test_parse_email_address_with_whitespace() {
    let (email, name) = parse_email_address("  Test User  <  test@example.com  >  ").unwrap();
    assert_eq!(email, "test@example.com");
    assert_eq!(name, "Test User");
}

#[test]
fn test_parse_email_address_invalid() {
    let result = parse_email_address("Test User <test@example.com");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_email_service_creation() {
    let service = EmailService::new("test-api-key".to_string(), None, None, None);
    assert_eq!(service.api_key, "test-api-key");
}

#[tokio::test]
async fn test_email_service_creation_with_dkim() {
    let service = EmailService::new(
        "test-api-key".to_string(),
        Some("example.com".to_string()),
        Some("selector".to_string()),
        Some("base64key".to_string()),
    );
    assert_eq!(service.dkim_domain, Some("example.com".to_string()));
    assert_eq!(service.dkim_selector, Some("selector".to_string()));
}

#[tokio::test]
async fn test_send_queued_email_requires_valid_api_key() {
    let service = EmailService::new("invalid-key".to_string(), None, None, None);

    let email = EmailQueueEntry {
        id: vec![0u8; 16],
        to_addr: "Test User <test@example.com>".to_string(),
        from_addr: "Sender <sender@example.com>".to_string(),
        subject: "Test Subject".to_string(),
        body: "Test body".to_string(),
        is_html: 0,
        status: "pending".to_string(),
        attempts: 0,
        max_attempts: 3,
        last_error: None,
        created_at: 0,
        last_attempted_at: None,
        sent_at: None,
    };

    let result = service.send_queued_email(&email).await;

    // Should fail with invalid API key
    assert!(result.is_err());
}

#[test]
fn test_email_request_serialization_without_dkim() {
    let request = EmailRequest {
        personalizations: vec![EmailPersonalization {
            to: vec![EmailAddress {
                email: "test@example.com".to_string(),
                name: "Test User".to_string(),
            }],
            dkim_domain: None,
            dkim_selector: None,
            dkim_private_key: None,
        }],
        from: EmailAddress {
            email: "sender@example.com".to_string(),
            name: "Sender".to_string(),
        },
        subject: "Test Subject".to_string(),
        content: vec![EmailContent {
            content_type: "text/plain".to_string(),
            value: "Test body".to_string(),
        }],
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("test@example.com"));
    assert!(json.contains("Test Subject"));
    assert!(json.contains("text/plain"));
    // DKIM fields should be omitted when None
    assert!(!json.contains("dkim_domain"));
    assert!(!json.contains("dkim_selector"));
    assert!(!json.contains("dkim_private_key"));
}

#[test]
fn test_email_request_serialization_with_dkim() {
    let request = EmailRequest {
        personalizations: vec![EmailPersonalization {
            to: vec![EmailAddress {
                email: "test@example.com".to_string(),
                name: "Test User".to_string(),
            }],
            dkim_domain: Some("example.com".to_string()),
            dkim_selector: Some("selector1".to_string()),
            dkim_private_key: Some("base64key".to_string()),
        }],
        from: EmailAddress {
            email: "sender@example.com".to_string(),
            name: "Sender".to_string(),
        },
        subject: "Test Subject".to_string(),
        content: vec![EmailContent {
            content_type: "text/plain".to_string(),
            value: "Test body".to_string(),
        }],
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("dkim_domain"));
    assert!(json.contains("example.com"));
    assert!(json.contains("dkim_selector"));
    assert!(json.contains("selector1"));
    assert!(json.contains("dkim_private_key"));
    assert!(json.contains("base64key"));
}
