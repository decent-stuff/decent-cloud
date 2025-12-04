use super::common::{ApiResponse, ApiTags};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct MessagesApi;

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct MessagesResponse {
    pub messages: Vec<crate::database::messages::Message>,
    pub thread: crate::database::messages::MessageThread,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UnreadCountResponse {
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct InboxResponse {
    pub threads: Vec<crate::database::messages::MessageThread>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ProviderResponseMetrics {
    pub avg_response_time_hours: Option<f64>,
    pub response_rate_pct: f64,
    pub total_threads: i64,
    pub responded_threads: i64,
}

#[OpenApi]
impl MessagesApi {
    /// List messages for contract
    ///
    /// Returns all messages in the contract's message thread (requires authentication)
    #[oai(
        path = "/contracts/:id/messages",
        method = "get",
        tag = "ApiTags::Messages"
    )]
    async fn list_contract_messages(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<MessagesResponse>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Get contract to verify user is participant
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => contract,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Verify user is requester or provider
        let auth_pubkey_hex = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != auth_pubkey_hex
            && contract.provider_pubkey != auth_pubkey_hex
        {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: not a participant in this contract".to_string()),
            });
        }

        // Get or create thread
        let thread = match db.get_thread_by_contract(&contract_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                // Create thread if it doesn't exist
                match db
                    .create_thread(
                        &contract_id,
                        "Contract Discussion",
                        &contract.requester_pubkey,
                        &contract.provider_pubkey,
                    )
                    .await
                {
                    Ok(_) => match db.get_thread_by_contract(&contract_id).await {
                        Ok(Some(t)) => t,
                        _ => {
                            return Json(ApiResponse {
                                success: false,
                                data: None,
                                error: Some("Failed to retrieve created thread".to_string()),
                            })
                        }
                    },
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(format!("Failed to create thread: {}", e)),
                        })
                    }
                }
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Get messages
        let messages = match db
            .get_messages_for_thread(&thread.id, &auth_pubkey_hex)
            .await
        {
            Ok(msgs) => msgs,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        Json(ApiResponse {
            success: true,
            data: Some(MessagesResponse { messages, thread }),
            error: None,
        })
    }

    /// Send message in contract
    ///
    /// Creates a new message in the contract's thread (requires authentication)
    #[oai(
        path = "/contracts/:id/messages",
        method = "post",
        tag = "ApiTags::Messages"
    )]
    async fn send_contract_message(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<SendMessageRequest>,
    ) -> Json<ApiResponse<crate::database::messages::Message>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Get contract to verify user is participant
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => contract,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Verify user is requester or provider
        let auth_pubkey_hex = hex::encode(&auth.pubkey);
        let sender_role = if contract.requester_pubkey == auth_pubkey_hex {
            "requester"
        } else if contract.provider_pubkey == auth_pubkey_hex {
            "provider"
        } else {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: not a participant in this contract".to_string()),
            });
        };

        // Get or create thread
        let thread = match db.get_thread_by_contract(&contract_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                // Create thread if it doesn't exist
                match db
                    .create_thread(
                        &contract_id,
                        "Contract Discussion",
                        &contract.requester_pubkey,
                        &contract.provider_pubkey,
                    )
                    .await
                {
                    Ok(_) => match db.get_thread_by_contract(&contract_id).await {
                        Ok(Some(t)) => t,
                        _ => {
                            return Json(ApiResponse {
                                success: false,
                                data: None,
                                error: Some("Failed to retrieve created thread".to_string()),
                            })
                        }
                    },
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(format!("Failed to create thread: {}", e)),
                        })
                    }
                }
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Create message
        let message_id = match db
            .create_message(&thread.id, &auth_pubkey_hex, sender_role, &req.body)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to create message: {}", e)),
                })
            }
        };

        // Queue notification for recipient
        let recipient_pubkey = if sender_role == "requester" {
            &contract.provider_pubkey
        } else {
            &contract.requester_pubkey
        };

        if let Err(e) = db
            .queue_message_notification(&message_id, recipient_pubkey)
            .await
        {
            tracing::warn!("Failed to queue message notification: {}", e);
        }

        // Fetch created message to return
        let messages = match db
            .get_messages_for_thread(&thread.id, &auth_pubkey_hex)
            .await
        {
            Ok(msgs) => msgs,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        let message = messages
            .into_iter()
            .find(|m| m.id == message_id)
            .unwrap_or_else(|| crate::database::messages::Message {
                id: message_id.clone(),
                message_id: hex::encode(&message_id),
                thread_id: hex::encode(&thread.id),
                sender_pubkey: auth_pubkey_hex.clone(),
                sender_role: sender_role.to_string(),
                body: req.body.clone(),
                created_at_ns: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                is_read: false,
            });

        Json(ApiResponse {
            success: true,
            data: Some(message),
            error: None,
        })
    }

    /// Get thread metadata for contract
    ///
    /// Returns thread information for a contract (requires authentication)
    #[oai(
        path = "/contracts/:id/thread",
        method = "get",
        tag = "ApiTags::Messages"
    )]
    async fn get_contract_thread(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<crate::database::messages::MessageThread>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Get contract to verify user is participant
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => contract,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Verify user is requester or provider
        let auth_pubkey_hex = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != auth_pubkey_hex
            && contract.provider_pubkey != auth_pubkey_hex
        {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: not a participant in this contract".to_string()),
            });
        }

        // Get thread
        match db.get_thread_by_contract(&contract_id).await {
            Ok(Some(thread)) => Json(ApiResponse {
                success: true,
                data: Some(thread),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Thread not found for this contract".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Mark message as read
    ///
    /// Marks a specific message as read by the authenticated user
    #[oai(path = "/messages/:id/read", method = "put", tag = "ApiTags::Messages")]
    async fn mark_message_read(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<String>> {
        let message_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid message ID format".to_string()),
                })
            }
        };

        let auth_pubkey_hex = hex::encode(&auth.pubkey);
        match db.mark_message_read(&message_id, &auth_pubkey_hex).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Message marked as read".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get unread message count
    ///
    /// Returns total unread message count for authenticated user
    #[oai(
        path = "/messages/unread-count",
        method = "get",
        tag = "ApiTags::Messages"
    )]
    async fn get_unread_count(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<UnreadCountResponse>> {
        let auth_pubkey_hex = hex::encode(&auth.pubkey);
        match db.get_unread_count(&auth_pubkey_hex).await {
            Ok(count) => Json(ApiResponse {
                success: true,
                data: Some(UnreadCountResponse { count }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user inbox
    ///
    /// Returns all message threads for authenticated user
    #[oai(path = "/messages/inbox", method = "get", tag = "ApiTags::Messages")]
    async fn get_inbox(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<InboxResponse>> {
        let auth_pubkey_hex = hex::encode(&auth.pubkey);
        match db.get_threads_for_user(&auth_pubkey_hex).await {
            Ok(threads) => Json(ApiResponse {
                success: true,
                data: Some(InboxResponse { threads }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider message response metrics
    ///
    /// Returns message response time statistics for a provider (public endpoint).
    /// Measures how quickly a provider responds to customer messages in threads.
    /// For contract status response metrics, use `/providers/:pubkey/contract-response-metrics`.
    #[oai(
        path = "/providers/:pubkey/response-metrics",
        method = "get",
        tag = "ApiTags::Messages"
    )]
    async fn get_provider_message_response_metrics(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<ProviderResponseMetrics>> {
        let provider_pubkey = match hex::decode(&pubkey.0) {
            Ok(pk) => hex::encode(&pk),
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid provider pubkey format".to_string()),
                })
            }
        };

        // Get all threads for provider
        let threads = match db.get_threads_for_user(&provider_pubkey).await {
            Ok(t) => t,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        let total_threads = threads.len() as i64;
        let mut responded_threads = 0;
        let mut total_response_time_ns: i64 = 0;
        let mut response_count = 0;

        for thread in threads {
            // Get messages for thread
            let messages = match db
                .get_messages_for_thread(&thread.id, &provider_pubkey)
                .await
            {
                Ok(msgs) => msgs,
                Err(_) => continue,
            };

            // Find first requester message and first provider response
            let mut first_requester_msg_time: Option<i64> = None;
            let mut first_provider_response_time: Option<i64> = None;

            for msg in messages {
                if msg.sender_role == "requester" && first_requester_msg_time.is_none() {
                    first_requester_msg_time = Some(msg.created_at_ns);
                } else if msg.sender_role == "provider" && first_provider_response_time.is_none() {
                    first_provider_response_time = Some(msg.created_at_ns);
                }

                if first_requester_msg_time.is_some() && first_provider_response_time.is_some() {
                    break;
                }
            }

            if let (Some(req_time), Some(resp_time)) =
                (first_requester_msg_time, first_provider_response_time)
            {
                if resp_time > req_time {
                    responded_threads += 1;
                    total_response_time_ns += resp_time - req_time;
                    response_count += 1;
                }
            }
        }

        let avg_response_time_hours = if response_count > 0 {
            Some(
                (total_response_time_ns as f64)
                    / (response_count as f64)
                    / 1_000_000_000.0
                    / 3600.0,
            )
        } else {
            None
        };

        let response_rate_pct = if total_threads > 0 {
            (responded_threads as f64) / (total_threads as f64) * 100.0
        } else {
            0.0
        };

        Json(ApiResponse {
            success: true,
            data: Some(ProviderResponseMetrics {
                avg_response_time_hours,
                response_rate_pct,
                total_threads,
                responded_threads,
            }),
            error: None,
        })
    }
}
