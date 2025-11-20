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
