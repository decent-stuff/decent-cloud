use super::common::{
    check_authorization, decode_pubkey, ApiResponse, ApiTags, CreateResellerRelationshipRequest,
    FulfillResellerOrderRequest, UpdateResellerRelationshipRequest,
};
use crate::auth::ApiAuthenticatedUser;
use crate::database::providers::ExternalProvider;
use crate::database::reseller::{ResellerOrder, ResellerRelationship};
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, param::Query, payload::Json, Object, OpenApi};
use std::sync::Arc;

/// API response type for reseller relationships with hex-encoded pubkeys
#[derive(Debug, Clone, Object)]
pub struct ResellerRelationshipResponse {
    pub id: i64,
    pub reseller_pubkey: String,
    pub external_provider_pubkey: String,
    pub commission_percent: i64,
    pub status: String,
    pub created_at_ns: i64,
    pub updated_at_ns: Option<i64>,
}

impl From<ResellerRelationship> for ResellerRelationshipResponse {
    fn from(r: ResellerRelationship) -> Self {
        Self {
            id: r.id,
            reseller_pubkey: hex::encode(&r.reseller_pubkey),
            external_provider_pubkey: hex::encode(&r.external_provider_pubkey),
            commission_percent: r.commission_percent,
            status: r.status,
            created_at_ns: r.created_at_ns,
            updated_at_ns: r.updated_at_ns,
        }
    }
}

/// API response type for reseller orders with hex-encoded pubkeys
#[derive(Debug, Clone, Object)]
pub struct ResellerOrderResponse {
    pub id: i64,
    pub contract_id: String,
    pub reseller_pubkey: String,
    pub external_provider_pubkey: String,
    pub offering_id: i64,
    pub base_price_e9s: i64,
    pub commission_e9s: i64,
    pub total_paid_e9s: i64,
    pub external_order_id: Option<String>,
    pub external_order_details: Option<String>,
    pub status: String,
    pub created_at_ns: i64,
    pub fulfilled_at_ns: Option<i64>,
}

impl From<ResellerOrder> for ResellerOrderResponse {
    fn from(o: ResellerOrder) -> Self {
        Self {
            id: o.id,
            contract_id: hex::encode(&o.contract_id),
            reseller_pubkey: hex::encode(&o.reseller_pubkey),
            external_provider_pubkey: hex::encode(&o.external_provider_pubkey),
            offering_id: o.offering_id,
            base_price_e9s: o.base_price_e9s,
            commission_e9s: o.commission_e9s,
            total_paid_e9s: o.total_paid_e9s,
            external_order_id: o.external_order_id,
            external_order_details: o.external_order_details,
            status: o.status,
            created_at_ns: o.created_at_ns,
            fulfilled_at_ns: o.fulfilled_at_ns,
        }
    }
}

pub struct ResellersApi;

#[OpenApi]
impl ResellersApi {
    /// List external providers available for reselling
    ///
    /// Returns a list of external providers with their offering counts.
    /// These are providers that have been seeded but may not have onboarded yet.
    #[oai(
        path = "/reseller/external-providers",
        method = "get",
        tag = "ApiTags::Resellers"
    )]
    async fn list_external_providers(
        &self,
        db: Data<&Arc<Database>>,
    ) -> Json<ApiResponse<Vec<ExternalProvider>>> {
        match db.list_external_providers().await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(providers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create reseller relationship
    ///
    /// Establishes a reseller relationship between the authenticated provider and an external provider.
    /// Requires provider authentication.
    #[oai(
        path = "/reseller/relationships",
        method = "post",
        tag = "ApiTags::Resellers"
    )]
    async fn create_reseller_relationship(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        request: Json<CreateResellerRelationshipRequest>,
    ) -> Json<ApiResponse<i64>> {
        let external_provider_pubkey = match decode_pubkey(&request.external_provider_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        // Check if provider is registered
        match db.get_provider_profile(&auth.pubkey).await {
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(
                        "Only registered providers can create reseller relationships".to_string(),
                    ),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to verify provider registration: {}", e)),
                });
            }
            Ok(Some(_)) => {} // Provider is registered, continue
        }

        match db
            .create_reseller_relationship(
                &auth.pubkey,
                &external_provider_pubkey,
                request.commission_percent,
            )
            .await
        {
            Ok(id) => Json(ApiResponse {
                success: true,
                data: Some(id),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update reseller relationship
    ///
    /// Updates commission settings or status for an existing reseller relationship.
    /// Requires provider authentication.
    #[oai(
        path = "/reseller/relationships/:external_provider_pubkey",
        method = "put",
        tag = "ApiTags::Resellers"
    )]
    async fn update_reseller_relationship(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        external_provider_pubkey: Path<String>,
        request: Json<UpdateResellerRelationshipRequest>,
    ) -> Json<ApiResponse<String>> {
        let external_provider_pubkey_bytes = match decode_pubkey(&external_provider_pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db
            .update_reseller_relationship_by_pubkeys(
                &auth.pubkey,
                &external_provider_pubkey_bytes,
                request.commission_percent,
                request.status.as_deref(),
            )
            .await
        {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some("Reseller relationship updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete reseller relationship
    ///
    /// Deactivates a reseller relationship. Requires provider authentication.
    #[oai(
        path = "/reseller/relationships/:external_provider_pubkey",
        method = "delete",
        tag = "ApiTags::Resellers"
    )]
    async fn delete_reseller_relationship(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        external_provider_pubkey: Path<String>,
    ) -> Json<ApiResponse<String>> {
        let external_provider_pubkey_bytes = match decode_pubkey(&external_provider_pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db
            .delete_reseller_relationship_by_pubkeys(&auth.pubkey, &external_provider_pubkey_bytes)
            .await
        {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some("Reseller relationship deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List reseller relationships
    ///
    /// Returns all reseller relationships for the authenticated provider.
    /// Requires provider authentication.
    #[oai(
        path = "/reseller/relationships",
        method = "get",
        tag = "ApiTags::Resellers"
    )]
    async fn list_reseller_relationships(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<Vec<ResellerRelationshipResponse>>> {
        match db
            .list_reseller_relationships_for_provider(&auth.pubkey)
            .await
        {
            Ok(relationships) => Json(ApiResponse {
                success: true,
                data: Some(relationships.into_iter().map(Into::into).collect()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List reseller orders
    ///
    /// Returns orders that need fulfillment by the authenticated provider.
    /// Supports optional status filtering (pending, fulfilled, failed).
    /// Requires provider authentication.
    #[oai(path = "/reseller/orders", method = "get", tag = "ApiTags::Resellers")]
    async fn list_reseller_orders(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        status: Query<Option<String>>,
    ) -> Json<ApiResponse<Vec<ResellerOrderResponse>>> {
        match db
            .list_reseller_orders_for_provider(&auth.pubkey, status.0.as_deref())
            .await
        {
            Ok(orders) => Json(ApiResponse {
                success: true,
                data: Some(orders.into_iter().map(Into::into).collect()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Fulfill reseller order
    ///
    /// Marks an order as fulfilled by providing external order details.
    /// Requires provider authentication.
    #[oai(
        path = "/reseller/orders/:contract_id/fulfill",
        method = "post",
        tag = "ApiTags::Resellers"
    )]
    async fn fulfill_reseller_order(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        contract_id: Path<String>,
        request: Json<FulfillResellerOrderRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id_bytes = match decode_pubkey(&contract_id.0) {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        // Verify the order belongs to this external provider
        match db.get_reseller_order(&contract_id_bytes).await {
            Ok(Some(order)) => {
                if let Err(e) = check_authorization(&order.external_provider_pubkey, &auth) {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(e),
                    });
                }
            }
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Reseller order not found".to_string()),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        }

        let external_order_details = request
            .external_order_details
            .clone()
            .unwrap_or_else(|| "{}".to_string());

        match db
            .fulfill_reseller_order(
                &contract_id_bytes,
                &request.external_order_id,
                &external_order_details,
            )
            .await
        {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some("Reseller order fulfilled successfully".to_string()),
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
