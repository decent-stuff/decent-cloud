//! Contract telemetry & history endpoints (usage, feedback, recipe log, events).
//!
//! Extracted from `contracts.rs` as part of #444 large-file splits. These six
//! handlers are all `/contracts/:id/{usage,feedback,recipe-log,events}`
//! sub-resources — the per-contract read/write data surface that is separate
//! from the contract lifecycle core (create/extend/cancel/checkout). They are
//! fully decoupled from `ContractsApi`: they depend only on `Database` methods
//! and the shared DTOs in `openapi::common` / `database::{contracts,stats}`,
//! with zero references to `contracts.rs`-private helpers. Behavior is
//! identical — every path/method/tag/schema is unchanged (verified via
//! byte-identical `/api/v1/openapi` spec, guarded by `spec_snapshot`).

use super::common::{decode_hex_path, ApiResponse, ApiTags, RecordUsageRequest};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct ContractTelemetryApi;

#[OpenApi]
impl ContractTelemetryApi {
    /// Record usage event for a contract
    ///
    /// Records a usage event (heartbeat, session start/end) for billing purposes.
    /// User must be the provider or an authorized agent.
    #[oai(
        path = "/contracts/:id/usage",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn record_usage(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<RecordUsageRequest>,
    ) -> Json<ApiResponse<i64>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: verify user is the provider
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: only provider can record usage".to_string()),
            });
        }

        // Record the usage event
        match db
            .record_usage_event(
                &contract_id,
                &req.event_type,
                req.units_delta,
                req.heartbeat_at,
                req.source.as_deref(),
                req.metadata.as_deref(),
            )
            .await
        {
            Ok(event_id) => Json(ApiResponse {
                success: true,
                data: Some(event_id),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get current usage for a contract
    ///
    /// Returns the current billing period usage for a contract.
    /// User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/usage",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_usage(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<crate::database::contracts::ContractUsage>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".to_string()),
            });
        }

        match db.get_current_usage(&contract_id).await {
            Ok(Some(usage)) => Json(ApiResponse {
                success: true,
                data: Some(usage),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("No active billing period for this contract".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Submit feedback for a contract
    ///
    /// Submit structured Y/N feedback after a contract is completed/cancelled.
    /// Only the contract requester (renter) can submit feedback, and only once per contract.
    #[oai(
        path = "/contracts/:id/feedback",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn submit_feedback(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<crate::database::stats::SubmitFeedbackInput>,
    ) -> Json<ApiResponse<crate::database::stats::ContractFeedback>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db
            .submit_contract_feedback(&contract_id, &auth.pubkey, &req.0)
            .await
        {
            Ok(feedback) => Json(ApiResponse {
                success: true,
                data: Some(feedback),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get recipe execution log for a contract
    ///
    /// Returns the combined stdout/stderr from the post-provision script execution.
    /// User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/recipe-log",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_recipe_log(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Option<String>>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".into()),
            });
        }

        match db.get_recipe_log_for_contract(&contract_id).await {
            Ok(log) => Json(ApiResponse {
                success: true,
                data: Some(log),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract event timeline
    ///
    /// Returns all timeline events for a contract in chronological order.
    /// User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/events",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_events(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractEvent>>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".into()),
            });
        }

        match db.get_contract_events(&contract_id).await {
            Ok(events) => Json(ApiResponse {
                success: true,
                data: Some(events),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get feedback for a contract
    ///
    /// Returns the feedback submitted for a specific contract, if any.
    /// User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/feedback",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_feedback(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Option<crate::database::stats::ContractFeedback>>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".to_string()),
            });
        }

        match db.get_contract_feedback(&contract_id).await {
            Ok(feedback) => Json(ApiResponse {
                success: true,
                data: Some(feedback),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::contracts::ContractUsage;
    use crate::database::stats::{ContractFeedback, SubmitFeedbackInput};
    use crate::openapi::common::{ApiResponse, RecordUsageRequest};

    #[test]
    fn test_contract_usage_serialization() {
        let usage = ContractUsage {
            id: 3,
            contract_id: "deadbeef".to_string(),
            billing_period_start: 1_700_000_000,
            billing_period_end: 1_700_003_600,
            units_used: 1.5,
            units_included: Some(10.0),
            overage_units: 0.0,
            estimated_charge_cents: Some(50),
            reported_to_stripe: false,
            stripe_usage_record_id: None,
            created_at: 1_700_000_001,
            updated_at: 1_700_003_601,
            billing_unit: "hour".to_string(),
        };
        let json = serde_json::to_value(&usage).unwrap();
        assert_eq!(json["id"], 3_i64);
        assert_eq!(json["contract_id"], "deadbeef");
        assert_eq!(json["units_used"], 1.5_f64);
        assert_eq!(json["units_included"], 10.0_f64);
        assert_eq!(json["overage_units"], 0.0_f64);
        assert_eq!(json["estimated_charge_cents"], 50_i64);
        assert_eq!(json["reported_to_stripe"], false);
        assert!(json.get("stripe_usage_record_id").is_none());
        assert_eq!(json["billing_unit"], "hour");
    }

    #[test]
    fn test_feedback_input_deserialization() {
        // SubmitFeedbackInput has no serde rename, so field names are snake_case
        let json = r#"{"service_matched_description":true,"would_rent_again":false}"#;
        let input: SubmitFeedbackInput = serde_json::from_str(json).unwrap();
        assert!(input.service_matched_description);
        assert!(!input.would_rent_again);
    }

    #[test]
    fn test_contract_feedback_serialization() {
        let feedback = ContractFeedback {
            contract_id: "deadbeef".to_string(),
            provider_pubkey: "aabbcc".to_string(),
            service_matched_description: true,
            would_rent_again: true,
            created_at_ns: 1_700_000_000_000_000_000,
        };
        let json = serde_json::to_value(&feedback).unwrap();
        assert_eq!(json["contract_id"], "deadbeef");
        assert_eq!(json["provider_pubkey"], "aabbcc");
        assert_eq!(json["service_matched_description"], true);
        assert_eq!(json["would_rent_again"], true);
        assert_eq!(json["created_at_ns"], 1_700_000_000_000_000_000_i64);
    }

    #[test]
    fn test_api_response_contract_feedback_success() {
        let feedback = ContractFeedback {
            contract_id: "cafebabe".to_string(),
            provider_pubkey: "112233".to_string(),
            service_matched_description: false,
            would_rent_again: false,
            created_at_ns: 0,
        };
        let resp = ApiResponse {
            success: true,
            data: Some(feedback),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["contract_id"], "cafebabe");
        assert_eq!(json["data"]["service_matched_description"], false);
    }

    #[test]
    fn test_record_usage_request_deserialization_all_fields() {
        let json = r#"{"eventType":"heartbeat","unitsDelta":1.0,"heartbeatAt":1700000000,"source":"agent-01","metadata":"{}"}"#;
        let req: RecordUsageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event_type, "heartbeat");
        assert_eq!(req.units_delta, Some(1.0));
        assert_eq!(req.heartbeat_at, Some(1_700_000_000));
        assert_eq!(req.source.as_deref(), Some("agent-01"));
        assert_eq!(req.metadata.as_deref(), Some("{}"));
    }

    #[test]
    fn test_record_usage_request_deserialization_minimal() {
        let json = r#"{"eventType":"session_start"}"#;
        let req: RecordUsageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event_type, "session_start");
        assert!(req.units_delta.is_none());
        assert!(req.heartbeat_at.is_none());
        assert!(req.source.is_none());
        assert!(req.metadata.is_none());
    }
}
