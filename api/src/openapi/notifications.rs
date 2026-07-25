//! Provider/user notification configuration, usage, and test endpoints.
//!
//! Extracted from `providers.rs` (#444 large-file split). These handlers all
//! carry the `ApiTags::Providers` tag and form a self-contained cluster with no
//! dependency on private helpers or local types defined in `providers.rs`.
//! Registration is unchanged from the consumer's perspective: `NotificationsApi`
//! is combined with the other `*Api` types in `openapi::create_combined_api`,
//! and every path, method, tag, and schema below is identical to the pre-split
//! API.

use super::common::{
    ApiResponse, ApiTags, NotificationConfigResponse, NotificationUsageResponse,
    TestNotificationRequest, TestNotificationResponse, UpdateNotificationConfigRequest,
};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{payload::Json, OpenApi};
use std::sync::Arc;

pub struct NotificationsApi;

#[OpenApi]
impl NotificationsApi {
    /// Get user notification configuration
    ///
    /// Returns notification preferences for the authenticated user
    #[oai(
        path = "/providers/me/notification-config",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_user_notification_config(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<NotificationConfigResponse>> {
        match db.get_user_notification_config(&auth.pubkey).await {
            Ok(Some(config)) => Json(ApiResponse {
                success: true,
                data: Some(NotificationConfigResponse {
                    notify_telegram: config.notify_telegram,
                    notify_email: config.notify_email,
                    notify_sms: config.notify_sms,
                    telegram_chat_id: config.telegram_chat_id,
                    notify_phone: config.notify_phone,
                    notify_email_address: config.notify_email_address,
                }),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: true,
                data: Some(NotificationConfigResponse {
                    notify_telegram: false,
                    notify_email: false,
                    notify_sms: false,
                    telegram_chat_id: None,
                    notify_phone: None,
                    notify_email_address: None,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update user notification configuration
    ///
    /// Updates notification preferences for the authenticated user
    #[oai(
        path = "/providers/me/notification-config",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_user_notification_config(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        req: Json<UpdateNotificationConfigRequest>,
    ) -> Json<ApiResponse<String>> {
        let config = crate::database::notification_config::UserNotificationConfig {
            user_pubkey: auth.pubkey.clone(),
            notify_telegram: req.notify_telegram,
            notify_email: req.notify_email,
            notify_sms: req.notify_sms,
            telegram_chat_id: req.telegram_chat_id.clone(),
            notify_phone: req.notify_phone.clone(),
            notify_email_address: req.notify_email_address.clone(),
        };

        match db.set_user_notification_config(&auth.pubkey, &config).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Notification configuration updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider notification usage
    ///
    /// Returns today's notification usage counts for the authenticated provider
    #[oai(
        path = "/providers/me/notification-usage",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_notification_usage(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<NotificationUsageResponse>> {
        let provider_id = hex::encode(&auth.pubkey);

        let telegram = db.get_notification_usage(&provider_id, "telegram").await;
        let sms = db.get_notification_usage(&provider_id, "sms").await;
        let email = db.get_notification_usage(&provider_id, "email").await;

        match (telegram, sms, email) {
            (Ok(tg), Ok(sm), Ok(em)) => Json(ApiResponse {
                success: true,
                data: Some(NotificationUsageResponse {
                    telegram_count: tg,
                    sms_count: sm,
                    email_count: em,
                    telegram_limit: crate::support_bot::notifications::TELEGRAM_DAILY_LIMIT,
                    sms_limit: crate::support_bot::notifications::SMS_DAILY_LIMIT,
                }),
                error: None,
            }),
            _ => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Failed to fetch usage data".to_string()),
            }),
        }
    }

    /// Test a notification channel
    ///
    /// Sends a test notification to the specified channel to verify configuration.
    /// Channels: "telegram", "email", "sms"
    #[oai(
        path = "/providers/me/notification-test",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn test_notification_channel(
        &self,
        db: Data<&Arc<Database>>,
        email_service: Data<&Option<Arc<email_utils::EmailService>>>,
        auth: ApiAuthenticatedUser,
        req: Json<TestNotificationRequest>,
    ) -> Json<ApiResponse<TestNotificationResponse>> {
        use crate::support_bot::test_notifications::send_test_notification;

        match send_test_notification(&db, email_service.as_ref(), &auth.pubkey, &req.channel).await
        {
            Ok(message) => Json(ApiResponse {
                success: true,
                data: Some(TestNotificationResponse {
                    sent: true,
                    message,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: true,
                data: Some(TestNotificationResponse {
                    sent: false,
                    message: format!("{:#}", e), // Full error chain
                }),
                error: None,
            }),
        }
    }

    /// Test the full escalation notification flow
    ///
    /// Creates a mock escalation event and dispatches notifications to all enabled channels.
    /// This tests the complete pipeline from Chatwoot escalation to notification delivery.
    #[oai(
        path = "/providers/me/notification-test/escalation",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn test_escalation_notification(
        &self,
        db: Data<&Arc<Database>>,
        email_service: Data<&Option<Arc<email_utils::EmailService>>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<TestNotificationResponse>> {
        use crate::support_bot::test_notifications::send_test_escalation;

        match send_test_escalation(&db, email_service.as_ref(), &auth.pubkey).await {
            Ok(message) => Json(ApiResponse {
                success: true,
                data: Some(TestNotificationResponse {
                    sent: true,
                    message,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: true,
                data: Some(TestNotificationResponse {
                    sent: false,
                    message: format!("{:#}", e), // Full error chain
                }),
                error: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::openapi::common::{
        NotificationConfigResponse, NotificationUsageResponse, TestNotificationResponse,
    };

    // ── NotificationConfigResponse ───────────────────────────────────────────

    #[test]
    fn test_notification_config_response_optional_fields_absent_when_none() {
        let config = NotificationConfigResponse {
            notify_telegram: false,
            notify_email: true,
            notify_sms: false,
            telegram_chat_id: None,
            notify_phone: None,
            notify_email_address: None,
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["notifyTelegram"], false);
        assert_eq!(json["notifyEmail"], true);
        // None fields serialise as null through serde (skip_serializing_if is poem-specific)
        assert!(
            json.get("telegramChatId").is_none_or(|v| v.is_null()),
            "telegramChatId should be absent or null"
        );
    }

    #[test]
    fn test_notification_config_response_with_all_fields() {
        let config = NotificationConfigResponse {
            notify_telegram: true,
            notify_email: true,
            notify_sms: true,
            telegram_chat_id: Some("123456789".to_string()),
            notify_phone: Some("+1555000".to_string()),
            notify_email_address: Some("a@b.com".to_string()),
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["telegramChatId"], "123456789");
        assert_eq!(json["notifyPhone"], "+1555000");
        assert_eq!(json["notifyEmailAddress"], "a@b.com");
    }

    // ── NotificationUsageResponse ────────────────────────────────────────────

    #[test]
    fn test_notification_usage_response_field_names() {
        let usage = NotificationUsageResponse {
            telegram_count: 5,
            sms_count: 2,
            email_count: 10,
            telegram_limit: 50,
            sms_limit: 10,
        };
        let json = serde_json::to_value(&usage).unwrap();
        assert_eq!(json["telegramCount"], 5_i64);
        assert_eq!(json["smsCount"], 2_i64);
        assert_eq!(json["emailCount"], 10_i64);
        assert_eq!(json["telegramLimit"], 50_i64);
        assert_eq!(json["smsLimit"], 10_i64);
    }

    // ── TestNotificationResponse ─────────────────────────────────────────────

    #[test]
    fn test_notification_test_response_sent_true() {
        let resp = TestNotificationResponse {
            sent: true,
            message: "Telegram message delivered".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["sent"], true);
        assert_eq!(json["message"], "Telegram message delivered");
    }

    #[test]
    fn test_notification_test_response_sent_false() {
        let resp = TestNotificationResponse {
            sent: false,
            message: "Bot token not configured".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["sent"], false);
        assert!(!json["message"].as_str().unwrap().is_empty());
    }
}
