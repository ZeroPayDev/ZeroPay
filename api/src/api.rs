use crate::AppState;
use crate::error::{ApiError, Result};
use crate::models::Customer;
use axum::extract::{Json, Query, State};
use serde::Deserialize;
use std::sync::Arc;
use x402::{
    DiscoveryRequest, DiscoveryResponse, Payee, PaymentRequirementsResponse, SettlementResponse,
    SupportedResponse, VerifyRequest,
};

#[derive(Deserialize)]
pub struct ApikeyAuth {
    apikey: String,
}

#[derive(Deserialize)]
pub struct PaymentRequest {
    customer: String,
    amount: i32,
}

pub async fn x402_requirements(
    State(app): State<Arc<AppState>>,
    Query(auth): Query<ApikeyAuth>,
    Json(data): Json<PaymentRequest>,
) -> Result<Json<PaymentRequirementsResponse>> {
    if auth.apikey != app.apikey {
        return Err(ApiError::UserAuth);
    }
    let customer = Customer::get_or_insert(data.customer, &app.db, &app.mnemonics).await?;

    // convert amount (2-decimal) to f32 price
    let price = format!("{:.2}", data.amount as f32 / 10f32.powi(2));
    let payee = Payee {
        evm: Some(customer.eth),
        sol: None,
    };
    let res = app.facilitator.create(&price, payee);

    Ok(Json(res))
}

pub async fn x402_payment(
    State(app): State<Arc<AppState>>,
    Query(auth): Query<ApikeyAuth>,
    Json(data): Json<VerifyRequest>,
) -> Result<Json<SettlementResponse>> {
    if auth.apikey != app.apikey {
        return Err(ApiError::UserAuth);
    }

    let res = app.facilitator.verify(&data).await;
    if !res.is_valid {
        return Ok(Json(res.to_settle(&data.payment_payload.network, "")));
    }

    let res2 = app.facilitator.settle(&data).await;

    Ok(Json(res2))
}

pub async fn x402_support(
    State(app): State<Arc<AppState>>,
    Query(auth): Query<ApikeyAuth>,
) -> Result<Json<SupportedResponse>> {
    if auth.apikey != app.apikey {
        return Err(ApiError::UserAuth);
    }

    let res = app.facilitator.support();
    Ok(Json(res))
}

pub async fn x402_discovery(
    State(app): State<Arc<AppState>>,
    Query(auth): Query<ApikeyAuth>,
    Query(data): Query<DiscoveryRequest>,
) -> Result<Json<DiscoveryResponse>> {
    if auth.apikey != app.apikey {
        return Err(ApiError::UserAuth);
    }

    let res = app.facilitator.discovery(data);
    Ok(Json(res))
}
