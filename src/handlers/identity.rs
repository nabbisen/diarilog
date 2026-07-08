use crate::auth;
use crate::storage::identity::UserStorage;
use contracts::identity::UpdateProfileRequest;
use worker::*;

/// GET /api/me
pub async fn get_profile(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    UserStorage::ensure(&ctx.env, &user.id, &user.email).await?;
    match UserStorage::get(&ctx.env, &user.id).await? {
        Some(record) => auth::json_200(&record),
        None => Ok(auth::error_404("User not found")),
    }
}

/// PUT /api/me
pub async fn update_profile(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let body: UpdateProfileRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Ok(auth::error_400("Invalid request body")),
    };
    if let Some(ref lang) = body.language {
        let supported = ctx.env.var("SUPPORTED_LANGUAGES")
            .ok().map(|v| v.to_string())
            .unwrap_or_else(|| "ja,en,ar,uk,es".to_string());
        if !supported.split(',').any(|s| s == lang.as_str()) {
            return Ok(auth::error_400(&format!("Unsupported language: {}", lang)));
        }
    }
    UserStorage::update(&ctx.env, &user.id,
        body.display_name.as_deref(), body.language.as_deref()).await?;
    auth::json_200(&serde_json::json!({ "updated": true }))
}
