use crate::auth;
use crate::storage::{diary::DiaryStorage, identity::UserStorage, triggers::TriggerStorage};
use worker::*;

/// POST /api/erase — permanently delete all data for the authenticated user.
pub async fn erase_all(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let env = &ctx.env;

    // Order matters: diary versions cascade from diaries, but we delete
    // R2 objects first to avoid orphans if D1 delete succeeds but R2 doesn't.
    DiaryStorage::erase_all_user_data(env, &user.id).await?;
    TriggerStorage::erase(env, &user.id).await?;
    UserStorage::erase(env, &user.id).await?;

    auth::json_200(&serde_json::json!({ "erased": true }))
}
