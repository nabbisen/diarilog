use crate::auth;
use crate::dialog::{ai_client, session::DialogStorage};
use crate::handlers::safety::classify_text;
use contracts::dialog::*;
use contracts::safety::SafetyLevel;
use worker::*;

fn lang(opt: &Option<String>) -> String {
    opt.clone().unwrap_or_else(|| "en".to_string())
}

/// POST /api/interview/start
pub async fn start_session(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let body: StartSessionRequest = req
        .json()
        .await
        .map_err(|_| Error::RustError("bad body".into()))?;
    let env = &ctx.env;
    let language = lang(&body.language);

    let session_id = uuid::Uuid::new_v4().to_string();
    DialogStorage::create_session(env, &session_id, &user.id, &language).await?;

    let q = ai_client::generate_first_question(env, &language).await?;
    DialogStorage::save_turn(
        env,
        &session_id,
        &user.id,
        1,
        &q.question,
        q.answer_type.as_str(),
        None,
    )
    .await?;

    auth::json_201(&StartSessionResponse {
        session_id,
        first_question: q,
    })
}

/// POST /api/interview/answer
pub async fn submit_answer(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let body: SubmitAnswerRequest = req
        .json()
        .await
        .map_err(|_| Error::RustError("bad body".into()))?;
    let env = &ctx.env;

    let session = match DialogStorage::get_session(env, &body.session_id, &user.id).await? {
        Some(s) => s,
        None => return Ok(auth::error_404("Session not found")),
    };
    let language = session.language.clone();

    // Safety classification — direct call, no service binding needed.
    let safety = classify_text(env, &body.answer, &language).await?;
    if matches!(safety.level, SafetyLevel::Crisis) {
        DialogStorage::update_session_status(env, &body.session_id, &user.id, "crisis_paused")
            .await?;
        return auth::json_200(&SubmitAnswerResponse {
            next_question: None,
            session_completed: false,
            crisis_resources: safety.resources,
        });
    }

    DialogStorage::save_answer(env, &body.turn_id, &body.answer).await?;

    let turns = DialogStorage::get_turns(env, &body.session_id).await?;
    let answered = turns.iter().filter(|t| t.answer.is_some()).count();
    let complete = answered >= 5;

    let next = if complete {
        DialogStorage::update_session_status(env, &body.session_id, &user.id, "completed").await?;
        None
    } else {
        let history = DialogStorage::get_session_history(env, &body.session_id).await?;
        let q = ai_client::generate_next_question(env, &language, &history, answered as i32 + 1)
            .await?;
        let turn_order = turns.len() as i32 + 1;
        DialogStorage::save_turn(
            env,
            &body.session_id,
            &user.id,
            turn_order,
            &q.question,
            q.answer_type.as_str(),
            None,
        )
        .await?;
        Some(q)
    };

    auth::json_200(&SubmitAnswerResponse {
        next_question: next,
        session_completed: complete,
        crisis_resources: None,
    })
}

/// GET /api/interview/:session_id
pub async fn get_session(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let session_id = ctx.param("session_id").unwrap();
    let session = match DialogStorage::get_session(&ctx.env, session_id, &user.id).await? {
        Some(s) => s,
        None => return Ok(auth::error_404("Session not found")),
    };
    let turns = DialogStorage::get_turns(&ctx.env, session_id).await?;
    auth::json_200(&SessionDetailResponse { session, turns })
}

/// POST /api/suggest
pub async fn generate_draft(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user = match auth::require_user(&req, &ctx.env).await {
        Ok(u) => u,
        Err(r) => return Ok(r),
    };
    let body: SuggestRequest = req
        .json()
        .await
        .map_err(|_| Error::RustError("bad body".into()))?;
    let env = &ctx.env;
    let max: i32 = env
        .var("MAX_SUGGEST_PER_DAY")
        .ok()
        .and_then(|v| v.to_string().parse().ok())
        .unwrap_or(10);
    let count = DialogStorage::count_suggestions_today(env, &user.id).await?;
    if count >= max {
        return Ok(auth::error_400(&format!(
            "Daily limit reached ({}/{})",
            count, max
        )));
    }
    let language = lang(&body.language);
    let input = body.user_input.unwrap_or_default();
    let drafts = ai_client::generate_drafts(env, &language, &input, 500).await?;
    let char_count = input.len() as i32;
    DialogStorage::log_suggestion(env, &user.id, char_count).await?;
    auth::json_200(&SuggestResponse {
        drafts,
        remaining_uses_today: max - count - 1,
    })
}
