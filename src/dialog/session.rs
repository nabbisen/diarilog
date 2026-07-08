//! D1 操作: interview_sessions / interview_turns / suggestion_logs

use contracts::dialog::{InterviewSession, InterviewTurn};
use worker::*;

pub struct DialogStorage;

impl DialogStorage {
    // ────── Sessions ──────

    pub async fn create_session(
        env: &Env,
        session_id: &str,
        user_id: &str,
        language: &str,
    ) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare("INSERT INTO interview_sessions (id, user_id, language) VALUES (?1, ?2, ?3)")
            .bind(&[session_id.into(), user_id.into(), language.into()])?
            .run()
            .await?;
        Ok(())
    }

    pub async fn get_session(
        env: &Env,
        session_id: &str,
        user_id: &str,
    ) -> Result<Option<InterviewSession>> {
        let db = env.d1("DB")?;
        Ok(db
            .prepare(
                "SELECT id, user_id, status, question_count, language, created_at, updated_at \
                 FROM interview_sessions WHERE id = ?1 AND user_id = ?2",
            )
            .bind(&[session_id.into(), user_id.into()])?
            .first::<InterviewSession>(None)
            .await?)
    }

    /// 進行中 (status='active') の最新セッションを 1 件取得する。
    /// なければ `None`。
    /// dashboard 集約 API で「再開できる対話」を検出するために使う。
    pub async fn get_active_session(env: &Env, user_id: &str) -> Result<Option<InterviewSession>> {
        let db = env.d1("DB")?;
        Ok(db
            .prepare(
                "SELECT id, user_id, status, question_count, language, created_at, updated_at \
                 FROM interview_sessions \
                 WHERE user_id = ?1 AND status = 'active' \
                 ORDER BY updated_at DESC LIMIT 1",
            )
            .bind(&[user_id.into()])?
            .first::<InterviewSession>(None)
            .await?)
    }

    pub async fn update_session_status(
        env: &Env,
        session_id: &str,
        user_id: &str,
        status: &str,
    ) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare(
            "UPDATE interview_sessions SET status = ?1, updated_at = datetime('now') \
             WHERE id = ?2 AND user_id = ?3",
        )
        .bind(&[status.into(), session_id.into(), user_id.into()])?
        .run()
        .await?;
        Ok(())
    }

    // ────── Turns ──────

    pub async fn save_turn(
        env: &Env,
        turn_id: &str,
        session_id: &str,
        turn_order: i32,
        question: &str,
        answer_type: &str,
        choices: Option<&str>,
    ) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare(
            "INSERT INTO interview_turns (id, session_id, turn_order, question, answer_type, choices) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&[
            turn_id.into(),
            session_id.into(),
            turn_order.into(),
            question.into(),
            answer_type.into(),
            choices.unwrap_or("").into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    pub async fn save_answer(env: &Env, turn_id: &str, answer: &str) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare("UPDATE interview_turns SET answer = ?1 WHERE id = ?2")
            .bind(&[answer.into(), turn_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    pub async fn get_turns(env: &Env, session_id: &str) -> Result<Vec<InterviewTurn>> {
        let db = env.d1("DB")?;
        let stmt = db.prepare(
            "SELECT id, session_id, turn_order, question, answer_type, choices, answer, created_at \
             FROM interview_turns WHERE session_id = ?1 ORDER BY turn_order ASC",
        );
        let results = stmt.bind(&[session_id.into()])?.all().await?;
        Ok(results.results::<InterviewTurn>()?)
    }

    pub async fn get_session_history(env: &Env, session_id: &str) -> Result<Vec<(String, String)>> {
        let turns = Self::get_turns(env, session_id).await?;
        Ok(turns
            .into_iter()
            .filter_map(|t| t.answer.map(|a| (t.question, a)))
            .collect())
    }

    // ────── Suggestion logs (rate limit) ──────

    pub async fn count_suggestions_today(env: &Env, user_id: &str) -> Result<i32> {
        let db = env.d1("DB")?;
        let row = db
            .prepare(
                "SELECT COUNT(*) AS count FROM suggestion_logs \
                 WHERE user_id = ?1 AND date(created_at) = date('now')",
            )
            .bind(&[user_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(row
            .and_then(|v| v.get("count").and_then(|c| c.as_i64()))
            .unwrap_or(0) as i32)
    }

    pub async fn log_suggestion(env: &Env, user_id: &str, char_count: i32) -> Result<()> {
        let db = env.d1("DB")?;
        let log_id = uuid::Uuid::new_v4().to_string();
        db.prepare("INSERT INTO suggestion_logs (id, user_id, char_count) VALUES (?1, ?2, ?3)")
            .bind(&[log_id.into(), user_id.into(), char_count.into()])?
            .run()
            .await?;
        Ok(())
    }

    // ────── Erase ──────

    /// dialog 所管の全データを消去 (suggestion_logs / interview_turns / interview_sessions)
    pub async fn erase(env: &Env, user_id: &str) -> Result<()> {
        let db = env.d1("DB")?;
        // FK 順守: turns を session 経由で先に消す
        db.prepare(
            "DELETE FROM interview_turns WHERE session_id IN \
             (SELECT id FROM interview_sessions WHERE user_id = ?1)",
        )
        .bind(&[user_id.into()])?
        .run()
        .await?;
        db.prepare("DELETE FROM interview_sessions WHERE user_id = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?;
        db.prepare("DELETE FROM suggestion_logs WHERE user_id = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?;
        Ok(())
    }
}
