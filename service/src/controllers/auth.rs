use crate::{
    AppState,
    auth::{create_user_jwt, siwe_verify},
    chain::{check_nonce, get_nonce},
    error::Result,
    models::Merchant,
};
use axum::extract::{Json, State};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

pub async fn nonce(State(app): State<Arc<AppState>>) -> Result<Json<Value>> {
    let nonce = get_nonce(&app.sender).await?;
    Ok(Json(json!({ "nonce": nonce })))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    message: String,
    signature: String,
}

pub async fn login(
    State(app): State<Arc<AppState>>,
    Json(data): Json<LoginRequest>,
) -> Result<Json<Value>> {
    // Verify the signature
    let (address, nonce) = siwe_verify(&data.message, &data.signature).await?;

    // Check if nonce is valid and not expired
    let nonce_valid = check_nonce(&nonce, &app.sender).await?;
    if !nonce_valid {
        return Err(crate::error::ApiError::Verify(
            "Invalid or expired nonce".to_string(),
        ));
    }

    let merchant = Merchant::insert(address, &app.db).await?;
    let token = create_user_jwt(merchant.id, &app.secret)?;

    Ok(Json(json!({
        "token": token,
        "account": merchant.account,
        "apikey": merchant.apikey,
        "name": merchant.name,
        "webhook": merchant.webhook,
        "eth": merchant.eth
    })))
}
