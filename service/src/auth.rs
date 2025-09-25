use crate::{
    AppState,
    error::{ApiError, Result},
};
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::prelude::*;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use siwe::{Message, VerificationOpts};
use std::sync::Arc;

/// Verify signature and return the address
pub async fn siwe_verify(message: &str, signature: &str) -> Result<(String, String)> {
    let message: Message = message
        .parse()
        .map_err(|e| ApiError::Verify(format!("Parse message error: {}", e)))?;
    let signature = hex::decode(signature.trim_start_matches("0x"))
        .map_err(|e| ApiError::Verify(format!("Decode signature error: {}", e)))?;

    // check signature
    message
        .verify(&signature, &VerificationOpts::default())
        .await
        .map_err(|e| ApiError::Verify(format!("Signature verification failed: {}", e)))?;

    Ok((
        format!("0x{}", hex::encode(&message.address)),
        message.nonce,
    ))
}

#[derive(Serialize, Deserialize)]
pub struct UserAuth {
    /// user id
    pub id: i32,
    /// issue timestamp
    pub iat: i64,
    /// token expiration
    pub exp: i64,
}

pub fn create_user_jwt(id: i32, secret: &[u8]) -> Result<String> {
    let now = Utc::now();
    let iat = now.timestamp_millis();
    let exp = now
        .checked_add_signed(chrono::Duration::days(90))
        .expect("valid timestamp")
        .timestamp_millis();

    let header = Header::new(Algorithm::HS512);
    let claims = UserAuth { id, iat, exp };

    encode(&header, &claims, &EncodingKey::from_secret(secret)).map_err(|_| ApiError::UserAuth)
}

impl FromRequestParts<std::sync::Arc<crate::AppState>> for UserAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        req: &mut Parts,
        state: &Arc<AppState>,
    ) -> std::result::Result<Self, Self::Rejection> {
        // Get authorisation header
        let authorisation = req
            .headers
            .get(AUTHORIZATION)
            .ok_or(ApiError::UserAuth)?
            .to_str()
            .map_err(|_| ApiError::UserAuth)?;

        // Check that is bearer and jwt
        let split = authorisation.split_once(' ');
        let jwt = match split {
            Some((name, contents)) if name == "Bearer" => Ok(contents),
            _ => Err(ApiError::UserAuth),
        }?;

        let decoded = decode::<UserAuth>(
            jwt,
            &DecodingKey::from_secret(&state.secret),
            &Validation::new(Algorithm::HS512),
        )
        .map_err(|_| ApiError::UserAuth)?;

        if decoded.claims.exp < Utc::now().timestamp_millis() {
            return Err(ApiError::UserAuth.into());
        }

        Ok(decoded.claims)
    }
}
