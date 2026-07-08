use crate::auth;
use crate::dialog::session::DialogStorage;
use crate::storage::{diary::DiaryStorage, identity::UserStorage};
use contracts::bff::{DashboardResponse, DashboardStatus};
use contracts::diary::DiaryMeta;
use futures::future::join3;
use worker::*;

const RECENT_LIMIT: usize = 5;

/// GET /api/dashboard
pub async fn aggregate(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let data = fetch(&ctx.env, &user.id).await?;
    auth::json_200(&data)
}

/// Called from SSR dashboard handler to pre-render with data.
/// Returns None if the user is not authenticated (SSR shows empty state).
pub async fn aggregate_data_for_ssr(req: &Request, env: &Env) -> Option<DashboardResponse> {
    let user = auth::require_user(req, env).await.ok()?;
    fetch(env, &user.id).await.ok()
}

async fn fetch(env: &Env, user_id: &str) -> Result<DashboardResponse> {
    let (user_res, diary_res, session_res) = join3(
        UserStorage::get(env, user_id),
        DiaryStorage::list_recent(env, user_id, RECENT_LIMIT as u32),
        DialogStorage::get_active_session(env, user_id),
    )
    .await;

    let (user_record, user_ok) = match user_res {
        Ok(Some(r)) => (Some(r), true),
        _ => (None, false),
    };
    let (recent_diaries, recent_diaries_ok): (Vec<DiaryMeta>, bool) = match diary_res {
        Ok(entries) => (entries, true),
        Err(_) => (Vec::new(), false),
    };
    let (active_session, active_session_ok) = match session_res {
        Ok(s) => (s, true),
        Err(_) => (None, false),
    };

    Ok(DashboardResponse {
        user: user_record,
        recent_diaries,
        active_session,
        status: DashboardStatus {
            user_ok,
            recent_diaries_ok,
            active_session_ok,
        },
    })
}
