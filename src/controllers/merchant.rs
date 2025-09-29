use crate::{
    AppState,
    auth::UserAuth,
    error::{ApiError, Result},
    models::Merchant,
};
use alloy::primitives::Address;
use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub async fn update_apikey(
    user: UserAuth,
    State(app): State<Arc<AppState>>,
) -> Result<Json<Value>> {
    let apikey = Merchant::update_apikey(user.id, &app.db).await?;
    Ok(Json(json!({ "apikey": apikey })))
}

#[derive(Serialize, Deserialize)]
pub struct MerchantInfo {
    name: String,
    webhook: String,
    eth: String,
}

pub async fn update_info(
    user: UserAuth,
    State(app): State<Arc<AppState>>,
    Json(data): Json<MerchantInfo>,
) -> Result<Json<Value>> {
    let eth: Address = data
        .eth
        .parse()
        .map_err(|_| ApiError::Verify("Invalid eth address".to_owned()))?;
    let eth_str = eth.to_checksum(None);

    let _ = Merchant::update_info(user.id, &data.name, &data.webhook, &eth_str, &app.db).await?;
    Ok(Json(json!({
        "name": data.name,
        "webhook": data.webhook,
        "eth": eth_str,
    })))
}
