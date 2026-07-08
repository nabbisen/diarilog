use crate::auth;
use crate::safety::{ai_client, classifier};
use crate::storage::triggers::TriggerStorage;
use contracts::safety::{
    AddTriggerRequest, ClassifyRequest, ClassifyResponse, TriggerListResponse,
};
use worker::*;

/// GET /api/triggers
pub async fn list_triggers(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let triggers = TriggerStorage::list_active(&ctx.env, &user.id).await?;
    auth::json_200(&TriggerListResponse { triggers })
}

/// POST /api/triggers
pub async fn add_trigger(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let body: AddTriggerRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Ok(auth::error_400("Invalid request body")),
    };
    if body.keyword.trim().is_empty() {
        return Ok(auth::error_400("Keyword cannot be empty"));
    }
    let trigger_id = uuid::Uuid::new_v4().to_string();
    let category = body.category.as_deref().unwrap_or("general");
    TriggerStorage::add(&ctx.env, &trigger_id, &user.id, &body.keyword, category).await?;
    auth::json_201(&serde_json::json!({ "id": trigger_id, "created": true }))
}

/// DELETE /api/triggers/:trigger_id
pub async fn remove_trigger(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let trigger_id = ctx.param("trigger_id").unwrap();
    TriggerStorage::deactivate(&ctx.env, trigger_id, &user.id).await?;
    auth::json_200(&serde_json::json!({ "deleted": true }))
}

/// Internal: run safety classification (called from dialog handler, not a public route).
pub async fn classify_text(env: &Env, text: &str, language: &str) -> Result<ClassifyResponse> {
    if classifier::keyword_crisis_check(text) {
        return Ok(ClassifyResponse {
            level: contracts::safety::SafetyLevel::Crisis,
            resources: Some(classifier::crisis_resources(language)),
        });
    }
    let raw = match ai_client::classify(env, text).await {
        Ok(s) => s,
        Err(_) => {
            return Ok(ClassifyResponse {
                level: contracts::safety::SafetyLevel::MildConcern,
                resources: None,
            });
        }
    };
    let level = classifier::parse_ai_classification(&raw);
    let resources = if matches!(level, contracts::safety::SafetyLevel::Crisis) {
        Some(classifier::crisis_resources(language))
    } else {
        None
    };
    Ok(ClassifyResponse { level, resources })
}
