use crate::AppState;
use crate::error::Result;
use crate::models::{Customer, Merchant, Session};
use axum::extract::{Json, Path, Query, State};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ApikeyAuth {
    apikey: String,
}

#[derive(Deserialize)]
pub struct CreateSession {
    customer: String,
    amount: i32,
}

#[derive(Serialize)]
pub struct SessionResponse {
    session_id: i32,
    session_url: String,
    merchant: String,
    customer: String,
    pay_eth: String,
    amount: i32,
    expired: NaiveDateTime,
}

impl SessionResponse {
    fn new(
        merchant: Merchant,
        customer: Customer,
        session: Session,
        domain: &str,
    ) -> SessionResponse {
        SessionResponse {
            session_id: session.id,
            session_url: format!("{domain}/sessions/{}", session.id),
            merchant: merchant.name,
            customer: customer.account,
            pay_eth: customer.eth,
            amount: session.amount,
            expired: session.expired_at,
        }
    }
}

pub async fn create_session(
    State(app): State<Arc<AppState>>,
    Query(auth): Query<ApikeyAuth>,
    Json(data): Json<CreateSession>,
) -> Result<Json<SessionResponse>> {
    let merchant = Merchant::get_by_apikey(&auth.apikey, &app.db).await?;
    let customer = Customer::get_or_insert(
        merchant.id,
        data.customer,
        &app.db,
        &app.mnemonics,
        Some(&app.redis),
    )
    .await?;
    let session = Session::insert(customer.id, data.amount, &app.db).await?;

    Ok(Json(SessionResponse::new(
        merchant,
        customer,
        session,
        &app.domain,
    )))
}

pub async fn get_session(
    State(app): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<SessionResponse>> {
    let session = Session::get(id, &app.db).await?;
    let customer = Customer::get(session.customer, &app.db).await?;
    let merchant = Merchant::get(customer.merchant, &app.db).await?;

    Ok(Json(SessionResponse::new(
        merchant,
        customer,
        session,
        &app.domain,
    )))
}
