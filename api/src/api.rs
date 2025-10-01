use crate::AppState;
use crate::error::{ApiError, Result};
use crate::models::{Customer, Session, store_address_in_redis};
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
    customer: String,
    pay_eth: String,
    amount: i32,
    expired: NaiveDateTime,
    completed: bool,
}

impl SessionResponse {
    fn new(customer: Customer, session: Session) -> SessionResponse {
        SessionResponse {
            session_id: session.id,
            customer: customer.account,
            pay_eth: customer.eth,
            amount: session.amount,
            expired: session.expired_at,
            completed: session.deposit.is_some(),
        }
    }
}

pub async fn create_session(
    State(app): State<Arc<AppState>>,
    Query(auth): Query<ApikeyAuth>,
    Json(data): Json<CreateSession>,
) -> Result<Json<SessionResponse>> {
    if auth.apikey != app.apikey {
        return Err(ApiError::UserAuth);
    }

    let customer = Customer::get_or_insert(data.customer, &app.db, &app.mnemonics).await?;
    let session = Session::insert(customer.id, data.amount, &app.db).await?;

    // save address to redis cache
    store_address_in_redis(&app.redis, &customer.eth, customer.id)
        .await
        .map_err(|_| ApiError::Internal)?;

    Ok(Json(SessionResponse::new(customer, session)))
}

pub async fn get_session(
    State(app): State<Arc<AppState>>,
    Query(auth): Query<ApikeyAuth>,
    Path(id): Path<i32>,
) -> Result<Json<SessionResponse>> {
    if auth.apikey != app.apikey {
        return Err(ApiError::UserAuth);
    }

    let session = Session::get(id, &app.db).await?;
    let customer = Customer::get(session.customer, &app.db).await?;

    Ok(Json(SessionResponse::new(customer, session)))
}
