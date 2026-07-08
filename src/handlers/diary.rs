use crate::auth;
use crate::storage::diary::DiaryStorage;
use contracts::diary::*;
use worker::*;

/// POST /api/diary
pub async fn create_entry(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let body: CreateDiaryRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Ok(auth::error_400("Invalid request body")),
    };
    let encrypted_bytes = decode_b64(&body.encrypted_body)?;
    let diary_id = uuid::Uuid::new_v4().to_string();
    DiaryStorage::save(
        &ctx.env,
        &user.id,
        &diary_id,
        &encrypted_bytes,
        &body.encrypted_title,
        body.encrypted_mood.as_deref(),
        body.word_count.unwrap_or(0),
        body.interview_id.as_deref(),
    )
    .await?;
    auth::json_201(&CreateDiaryResponse {
        id: diary_id,
        created: true,
    })
}

/// GET /api/diary
pub async fn list_entries(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let entries = DiaryStorage::list(&ctx.env, &user.id).await?;
    let total = entries.len();
    auth::json_200(&DiaryListResponse { entries, total })
}

/// GET /api/diary/:diary_id
pub async fn get_entry(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let diary_id = ctx.param("diary_id").unwrap();
    let meta = match DiaryStorage::get_meta(&ctx.env, &user.id, diary_id).await? {
        Some(m) => m,
        None => return Ok(auth::error_404("Diary not found")),
    };
    let body = DiaryStorage::get_body(&ctx.env, &meta.r2_key)
        .await?
        .unwrap_or_default();
    auth::json_200(&DiaryDetailResponse {
        meta,
        encrypted_body: encode_b64(&body),
    })
}

/// PUT /api/diary/:diary_id
pub async fn update_entry(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let diary_id = ctx.param("diary_id").unwrap();
    let body: UpdateDiaryRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Ok(auth::error_400("Invalid request body")),
    };
    let encrypted_bytes = match body.encrypted_body.as_ref() {
        Some(s) => Some(decode_b64(s)?),
        None => None,
    };
    DiaryStorage::update(
        &ctx.env,
        &user.id,
        diary_id,
        encrypted_bytes.as_deref(),
        body.encrypted_title.as_deref(),
        body.encrypted_mood.as_deref(),
        body.word_count,
    )
    .await?;
    auth::json_200(&serde_json::json!({ "updated": true }))
}

/// DELETE /api/diary/:diary_id
pub async fn delete_entry(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let diary_id = ctx.param("diary_id").unwrap();
    DiaryStorage::soft_delete(&ctx.env, &user.id, diary_id).await?;
    auth::json_200(&serde_json::json!({ "deleted": true }))
}

/// GET /api/diary/:diary_id/versions
pub async fn list_versions(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let diary_id = ctx.param("diary_id").unwrap();
    let versions = DiaryStorage::list_versions(&ctx.env, &user.id, diary_id).await?;
    auth::json_200(&DiaryVersionListResponse {
        diary_id: diary_id.to_string(),
        versions,
    })
}

/// GET /api/diary/:diary_id/versions/:version
pub async fn get_version(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let diary_id = ctx.param("diary_id").unwrap();
    let version: u32 = ctx
        .param("version")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    match DiaryStorage::get_version_body(&ctx.env, &user.id, diary_id, version).await? {
        Some((meta, body)) => auth::json_200(&DiaryVersionDetailResponse {
            diary_id: diary_id.to_string(),
            version: meta.version,
            edited_at: meta.edited_at,
            encrypted_title: meta.encrypted_title,
            encrypted_body: encode_b64(&body),
        }),
        None => Ok(auth::error_404("Version not found")),
    }
}

/// DELETE /api/diary/:diary_id/versions/:version
pub async fn delete_version(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let diary_id = ctx.param("diary_id").unwrap();
    let version: u32 = ctx
        .param("version")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    DiaryStorage::delete_version(&ctx.env, &user.id, diary_id, version).await?;
    auth::json_200(&serde_json::json!({ "deleted": true }))
}

/// POST /api/sync
pub async fn sync_data(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let body: SyncRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Ok(auth::error_400("Invalid sync request body")),
    };
    let mut created_ids = Vec::new();
    for entry in &body.new_entries {
        let encrypted_bytes = decode_b64(&entry.encrypted_body)?;
        let diary_id = uuid::Uuid::new_v4().to_string();
        DiaryStorage::save(
            &ctx.env,
            &user.id,
            &diary_id,
            &encrypted_bytes,
            &entry.encrypted_title,
            entry.encrypted_mood.as_deref(),
            entry.word_count.unwrap_or(0),
            entry.interview_id.as_deref(),
        )
        .await?;
        created_ids.push(diary_id);
    }
    for update in &body.updated_entries {
        let encrypted_bytes = match update.encrypted_body.as_ref() {
            Some(s) => Some(decode_b64(s)?),
            None => None,
        };
        DiaryStorage::update(
            &ctx.env,
            &user.id,
            &update.diary_id,
            encrypted_bytes.as_deref(),
            update.encrypted_title.as_deref(),
            update.encrypted_mood.as_deref(),
            update.word_count,
        )
        .await?;
    }
    let server_updates = DiaryStorage::list(&ctx.env, &user.id).await?;
    let synced_at = format!("{}", worker::Date::now().as_millis() / 1000);
    auth::json_200(&SyncResponse {
        server_updates,
        created_ids,
        synced_at,
    })
}

fn decode_b64(s: &str) -> Result<Vec<u8>> {
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
        .map_err(|_| Error::RustError("Invalid base64".into()))
}

fn encode_b64(bytes: &[u8]) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
}
