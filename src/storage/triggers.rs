//! D1 `trigger_keywords` テーブル操作。

use contracts::safety::TriggerKeyword;
use worker::*;

pub struct TriggerStorage;

impl TriggerStorage {
    /// アクティブなトリガー一覧を取得
    pub async fn list_active(env: &Env, user_id: &str) -> Result<Vec<TriggerKeyword>> {
        let db = env.d1("DB")?;
        let stmt = db.prepare(
            "SELECT id, user_id, keyword, category, is_active, created_at \
             FROM trigger_keywords WHERE user_id = ?1 AND is_active = 1",
        );
        let results = stmt.bind(&[user_id.into()])?.all().await?;
        Ok(results.results::<TriggerKeyword>()?)
    }

    /// 追加 (idempotent でなくてよい — クライアント側で重複防止)
    pub async fn add(
        env: &Env,
        trigger_id: &str,
        user_id: &str,
        keyword: &str,
        category: &str,
    ) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare(
            "INSERT INTO trigger_keywords (id, user_id, keyword, category) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&[
            trigger_id.into(),
            user_id.into(),
            keyword.into(),
            category.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// 無効化 (論理削除)
    pub async fn deactivate(env: &Env, trigger_id: &str, user_id: &str) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare(
            "UPDATE trigger_keywords SET is_active = 0 WHERE id = ?1 AND user_id = ?2",
        )
        .bind(&[trigger_id.into(), user_id.into()])?
        .run()
        .await?;
        Ok(())
    }

    /// 緊急消去 (safety 所管: trigger_keywords 行のみ)
    pub async fn erase(env: &Env, user_id: &str) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare("DELETE FROM trigger_keywords WHERE user_id = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?;
        Ok(())
    }
}
