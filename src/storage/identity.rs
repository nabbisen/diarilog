//! D1 `users` テーブル操作。

use contracts::identity::UserRecord;
use worker::*;

pub struct UserStorage;

impl UserStorage {
    /// 初回登録 (idempotent)
    pub async fn ensure(env: &Env, user_id: &str, email: &str) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare("INSERT OR IGNORE INTO users (id, email) VALUES (?1, ?2)")
            .bind(&[user_id.into(), email.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// プロフィール取得
    pub async fn get(env: &Env, user_id: &str) -> Result<Option<UserRecord>> {
        let db = env.d1("DB")?;
        Ok(db
            .prepare(
                "SELECT id, email, display_name, language, created_at, updated_at \
                 FROM users WHERE id = ?1",
            )
            .bind(&[user_id.into()])?
            .first::<UserRecord>(None)
            .await?)
    }

    /// プロフィール更新
    pub async fn update(
        env: &Env,
        user_id: &str,
        display_name: Option<&str>,
        language: Option<&str>,
    ) -> Result<()> {
        let db = env.d1("DB")?;
        if let Some(name) = display_name {
            db.prepare(
                "UPDATE users SET display_name = ?1, updated_at = datetime('now') WHERE id = ?2",
            )
            .bind(&[name.into(), user_id.into()])?
            .run()
            .await?;
        }
        if let Some(lang) = language {
            db.prepare(
                "UPDATE users SET language = ?1, updated_at = datetime('now') WHERE id = ?2",
            )
            .bind(&[lang.into(), user_id.into()])?
            .run()
            .await?;
        }
        Ok(())
    }

    /// 緊急消去 (identity 所管: users 行のみ)
    pub async fn erase(env: &Env, user_id: &str) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare("DELETE FROM users WHERE id = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?;
        Ok(())
    }
}
