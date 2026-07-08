//! Authentication helpers.
//!
//! Replaces the old gateway middleware. All handlers call `require_user`
//! directly; there is no separate auth worker.

use auth_core::{OidcConfig, verify_id_token};
use errors::{ApiError, ErrorCode};
use worker::*;

pub struct AuthenticatedUser {
    pub id: String,
    pub email: String,
}

/// Extract and validate the Bearer token from the Authorization header.
/// Returns an `AuthenticatedUser` on success, or an `ApiError` response
/// that the handler can return immediately on failure.
pub async fn require_user(
    req: &Request,
    env: &Env,
) -> std::result::Result<AuthenticatedUser, Response> {
    let header = req
        .headers()
        .get("Authorization")
        .unwrap_or(None)
        .unwrap_or_default();

    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .unwrap_or("")
        .trim();

    if token.is_empty() {
        return Err(error_401("Missing or invalid Authorization header"));
    }

    let config = match OidcConfig::from_env(env) {
        Ok(c) => c,
        Err(_) => return Err(error_401("OIDC not configured")),
    };

    match verify_id_token(env, token, &config).await {
        Ok(claims) => Ok(AuthenticatedUser {
            id: claims.subject,
            email: claims.email,
        }),
        Err(_) => Err(error_401("Invalid or expired token")),
    }
}

pub fn error_401(msg: &str) -> Response {
    let body = ApiError::new(ErrorCode::Unauthorized, msg);
    Response::from_json(&body)
        .map(|r| r.with_status(401))
        .unwrap_or_else(|_| Response::error(msg, 401).unwrap())
}

pub fn error_404(msg: &str) -> Response {
    let body = ApiError::new(ErrorCode::NotFound, msg);
    Response::from_json(&body)
        .map(|r| r.with_status(404))
        .unwrap_or_else(|_| Response::error(msg, 404).unwrap())
}

pub fn error_400(msg: &str) -> Response {
    let body = ApiError::new(ErrorCode::ValidationFailed, msg);
    Response::from_json(&body)
        .map(|r| r.with_status(400))
        .unwrap_or_else(|_| Response::error(msg, 400).unwrap())
}

pub fn json_200<T: serde::Serialize>(data: &T) -> Result<Response> {
    Response::from_json(data).map(|r| r.with_status(200))
}

pub fn json_201<T: serde::Serialize>(data: &T) -> Result<Response> {
    Response::from_json(data).map(|r| r.with_status(201))
}
