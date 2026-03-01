use super::common::{ApiResponse, ApiTags};
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct ValidatorsApi;

#[OpenApi]
impl ValidatorsApi {
    /// Get active validators
    ///
    /// Returns validators that have checked in within the specified number of days
    #[oai(
        path = "/validators/active/:days",
        method = "get",
        tag = "ApiTags::Validators"
    )]
    async fn get_active_validators(
        &self,
        db: Data<&Arc<Database>>,
        days: Path<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::Validator>>> {
        match db.get_active_validators(days.0).await {
            Ok(validators) => Json(ApiResponse {
                success: true,
                data: Some(validators),
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
    use crate::database::providers::Validator;
    use crate::openapi::common::ApiResponse;

    fn sample_validator() -> Validator {
        Validator {
            pubkey: "ab".repeat(32),
            name: Some("TestValidator".to_string()),
            description: Some("A test validator node".to_string()),
            website_url: Some("https://example.com".to_string()),
            logo_url: None,
            total_check_ins: 1500,
            check_ins_24h: 24,
            check_ins_7d: 168,
            check_ins_30d: 720,
            last_check_in_ns: 1_700_000_000_000_000_000,
            registered_at_ns: 1_690_000_000_000_000_000,
        }
    }

    #[test]
    fn test_validator_serialization_camel_case_fields() {
        let v = sample_validator();
        let json = serde_json::to_value(&v).unwrap();
        // Verify camelCase from #[serde(rename_all = "camelCase")]
        assert!(
            json.get("totalCheckIns").is_some(),
            "Expected camelCase field totalCheckIns"
        );
        assert!(
            json.get("checkIns24h").is_some(),
            "Expected camelCase field checkIns24h"
        );
        assert!(json.get("checkIns7d").is_some());
        assert!(json.get("checkIns30d").is_some());
        assert!(json.get("lastCheckInNs").is_some());
        assert!(json.get("registeredAtNs").is_some());
        // Verify no snake_case leakage
        assert!(
            json.get("total_check_ins").is_none(),
            "snake_case field should not exist"
        );
    }

    #[test]
    fn test_validator_serialization_values() {
        let v = sample_validator();
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["pubkey"], "ab".repeat(32));
        assert_eq!(json["name"], "TestValidator");
        assert_eq!(json["totalCheckIns"], 1500);
        assert_eq!(json["checkIns24h"], 24);
        assert_eq!(json["checkIns7d"], 168);
        assert_eq!(json["checkIns30d"], 720);
        assert_eq!(json["lastCheckInNs"], 1_700_000_000_000_000_000_i64);
    }

    #[test]
    fn test_validator_optional_fields_none_serialize_as_null() {
        let v = Validator {
            logo_url: None,
            name: None,
            description: None,
            website_url: None,
            ..sample_validator()
        };
        let json = serde_json::to_value(&v).unwrap();
        assert!(json.get("logoUrl").is_none());
        assert!(json.get("name").is_none());
        assert!(json.get("description").is_none());
        assert!(json.get("websiteUrl").is_none());
        // Required fields still present
        assert_eq!(json["totalCheckIns"], 1500);
    }

    #[test]
    fn test_api_response_validators_success() {
        let validators = vec![sample_validator()];
        let resp = ApiResponse {
            success: true,
            data: Some(validators),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["pubkey"], "ab".repeat(32));
    }

    #[test]
    fn test_api_response_validators_error() {
        let resp: ApiResponse<Vec<Validator>> = ApiResponse {
            success: false,
            data: None,
            error: Some("query timeout".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "query timeout");
        assert!(json.get("data").is_none());
    }

    #[test]
    fn test_api_response_validators_empty_list() {
        let resp = ApiResponse {
            success: true,
            data: Some(Vec::<Validator>::new()),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"].as_array().unwrap().len(), 0);
    }
}
