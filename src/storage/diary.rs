//! R2 + D1 diary storage operations.
//!
//! Reflects RFC 011 (encrypted title/mood) and RFC 012 (version history).
//!
//! ## Encryption contract
//!
//! This layer is encryption-agnostic: it stores and retrieves opaque
//! base64 ciphertext strings for `encrypted_title` and `encrypted_mood`.
//! The browser encrypts before sending and decrypts after receiving.
//! The server never sees plaintext for these fields.
//!
//! ## Version history (RFC 012)
//!
//! Every write to an existing diary entry produces a new `diary_versions`
//! row and increments `diaries.version`. A soft cap of `MAX_VERSIONS`
//! per entry is enforced at write time; the oldest non-original version
//! (version_number > 1) is pruned when the cap is exceeded.

use contracts::diary::{DiaryMeta, DiaryVersionMeta};
use worker::*;

/// Maximum number of versions retained per diary entry.
const MAX_VERSIONS: u32 = 20;

pub struct DiaryStorage;

impl DiaryStorage {
    // ── Create ────────────────────────────────────────────────────────────

    /// Save an encrypted diary body to R2 and insert metadata into D1.
    /// Also inserts the first `diary_versions` row (version 1).
    pub async fn save(
        env: &Env,
        user_id: &str,
        diary_id: &str,
        encrypted_body: &[u8],
        encrypted_title: &str,
        encrypted_mood: Option<&str>,
        word_count: i32,
        interview_id: Option<&str>,
    ) -> Result<()> {
        let r2_key = format!("diaries/{}/{}", user_id, diary_id);

        // R2 body
        env.bucket("DIARY_BUCKET")?
            .put(&r2_key, encrypted_body.to_vec())
            .execute()
            .await?;

        let db = env.d1("DB")?;

        // D1 diaries row
        db.prepare(
            "INSERT INTO diaries \
             (id, user_id, r2_key, encrypted_title, encrypted_mood, \
              word_count, interview_id, version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        )
        .bind(&[
            diary_id.into(),
            user_id.into(),
            r2_key.as_str().into(),
            encrypted_title.into(),
            encrypted_mood.unwrap_or("").into(),
            word_count.into(),
            interview_id.unwrap_or("").into(),
        ])?
        .run()
        .await?;

        // First version row
        let version_id = format!("{}_v1", diary_id);
        db.prepare(
            "INSERT INTO diary_versions \
             (id, diary_id, version_number, edited_at, body_ref, \
              encrypted_title, encrypted_mood) \
             VALUES (?1, ?2, 1, datetime('now'), ?3, ?4, ?5)",
        )
        .bind(&[
            version_id.as_str().into(),
            diary_id.into(),
            r2_key.as_str().into(),
            encrypted_title.into(),
            encrypted_mood.unwrap_or("").into(),
        ])?
        .run()
        .await?;

        Ok(())
    }

    // ── Read ──────────────────────────────────────────────────────────────

    pub async fn list(env: &Env, user_id: &str) -> Result<Vec<DiaryMeta>> {
        let db = env.d1("DB")?;
        let results = db
            .prepare(
                "SELECT id, user_id, r2_key, encrypted_title, encrypted_mood, \
                        word_count, interview_id, created_at, updated_at, version \
                 FROM diaries \
                 WHERE user_id = ?1 AND deleted_at IS NULL \
                 ORDER BY created_at DESC",
            )
            .bind(&[user_id.into()])?
            .all()
            .await?;
        Ok(results.results::<DiaryMeta>()?)
    }

    pub async fn list_recent(env: &Env, user_id: &str, limit: u32) -> Result<Vec<DiaryMeta>> {
        let db = env.d1("DB")?;
        let results = db
            .prepare(
                "SELECT id, user_id, r2_key, encrypted_title, encrypted_mood, \
                        word_count, interview_id, created_at, updated_at, version \
                 FROM diaries \
                 WHERE user_id = ?1 AND deleted_at IS NULL \
                 ORDER BY created_at DESC \
                 LIMIT ?2",
            )
            .bind(&[user_id.into(), (limit as f64).into()])?
            .all()
            .await?;
        Ok(results.results::<DiaryMeta>()?)
    }

    pub async fn get_meta(env: &Env, user_id: &str, diary_id: &str) -> Result<Option<DiaryMeta>> {
        let db = env.d1("DB")?;
        Ok(db
            .prepare(
                "SELECT id, user_id, r2_key, encrypted_title, encrypted_mood, \
                        word_count, interview_id, created_at, updated_at, version \
                 FROM diaries \
                 WHERE id = ?1 AND user_id = ?2 AND deleted_at IS NULL",
            )
            .bind(&[diary_id.into(), user_id.into()])?
            .first::<DiaryMeta>(None)
            .await?)
    }

    pub async fn get_body(env: &Env, r2_key: &str) -> Result<Option<Vec<u8>>> {
        let bucket = env.bucket("DIARY_BUCKET")?;
        match bucket.get(r2_key).execute().await? {
            Some(obj) => {
                let bytes = obj
                    .body()
                    .ok_or_else(|| Error::RustError("Empty R2 body".into()))?
                    .bytes()
                    .await?;
                Ok(Some(bytes))
            }
            None => Ok(None),
        }
    }

    // ── Update (creates a new version — RFC 012) ─────────────────────────

    /// Update an existing entry. Writes a new R2 object for the body,
    /// increments the version counter, and inserts a `diary_versions` row.
    ///
    /// If the version count for this entry would exceed `MAX_VERSIONS`, the
    /// oldest non-original version (version_number > 1) is pruned.
    pub async fn update(
        env: &Env,
        user_id: &str,
        diary_id: &str,
        encrypted_body: Option<&[u8]>,
        encrypted_title: Option<&str>,
        encrypted_mood: Option<&str>,
        word_count: Option<i32>,
    ) -> Result<()> {
        let db = env.d1("DB")?;

        // Fetch current entry to get r2_key, current version, current encrypted_title.
        let current = match Self::get_meta(env, user_id, diary_id).await? {
            Some(m) => m,
            None => {
                return Err(Error::RustError(format!("diary {} not found", diary_id)));
            }
        };

        let new_version = current.version + 1;

        // Determine the new R2 key for the body. If a new body is provided,
        // write it with a fresh key so old versions can be accessed independently.
        let new_r2_key = if encrypted_body.is_some() {
            format!("diaries/{}/{}_v{}", user_id, diary_id, new_version)
        } else {
            current.r2_key.clone()
        };

        if let Some(body) = encrypted_body {
            env.bucket("DIARY_BUCKET")?
                .put(&new_r2_key, body.to_vec())
                .execute()
                .await?;
        }

        // Determine new title / mood (fall back to current).
        let new_title =
            encrypted_title.unwrap_or_else(|| current.encrypted_title.as_deref().unwrap_or(""));
        let new_mood = encrypted_mood.or_else(|| current.encrypted_mood.as_deref());
        let new_wc = word_count.unwrap_or(current.word_count);

        // Update the diaries row (current state).
        db.prepare(
            "UPDATE diaries \
             SET r2_key = ?1, encrypted_title = ?2, encrypted_mood = ?3, \
                 word_count = ?4, version = ?5, updated_at = datetime('now') \
             WHERE id = ?6 AND user_id = ?7",
        )
        .bind(&[
            new_r2_key.as_str().into(),
            new_title.into(),
            new_mood.unwrap_or("").into(),
            new_wc.into(),
            (new_version as f64).into(),
            diary_id.into(),
            user_id.into(),
        ])?
        .run()
        .await?;

        // Insert version history row.
        let version_id = format!("{}_v{}", diary_id, new_version);
        db.prepare(
            "INSERT INTO diary_versions \
             (id, diary_id, version_number, edited_at, body_ref, \
              encrypted_title, encrypted_mood) \
             VALUES (?1, ?2, ?3, datetime('now'), ?4, ?5, ?6)",
        )
        .bind(&[
            version_id.as_str().into(),
            diary_id.into(),
            (new_version as f64).into(),
            new_r2_key.as_str().into(),
            new_title.into(),
            new_mood.unwrap_or("").into(),
        ])?
        .run()
        .await?;

        // Enforce MAX_VERSIONS cap: prune oldest non-original version.
        // Check count first to avoid a write on every update.
        let count_result = db
            .prepare("SELECT COUNT(*) AS cnt FROM diary_versions WHERE diary_id = ?1")
            .bind(&[diary_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        if let Some(val) = count_result {
            let count = val.get("cnt").and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;
            if count > MAX_VERSIONS {
                // Find oldest version with version_number > 1 (preserve original).
                if let Some(oldest) = db
                    .prepare(
                        "SELECT id, body_ref FROM diary_versions \
                         WHERE diary_id = ?1 AND version_number > 1 \
                         ORDER BY version_number ASC LIMIT 1",
                    )
                    .bind(&[diary_id.into()])?
                    .first::<serde_json::Value>(None)
                    .await?
                {
                    let old_id = oldest
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let old_r2 = oldest
                        .get("body_ref")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Delete the pruned R2 object if it's not shared with current.
                    if !old_r2.is_empty() && old_r2 != current.r2_key {
                        let _ = env.bucket("DIARY_BUCKET")?.delete(&old_r2).await;
                    }

                    db.prepare("DELETE FROM diary_versions WHERE id = ?1")
                        .bind(&[old_id.as_str().into()])?
                        .run()
                        .await?;
                }
            }
        }

        Ok(())
    }

    // ── Version history (RFC 012) ────────────────────────────────────────

    /// List version metadata (no bodies) for a diary entry.
    pub async fn list_versions(
        env: &Env,
        user_id: &str,
        diary_id: &str,
    ) -> Result<Vec<DiaryVersionMeta>> {
        // Verify the entry belongs to user_id before exposing versions.
        if Self::get_meta(env, user_id, diary_id).await?.is_none() {
            return Err(Error::RustError(format!("diary {} not found", diary_id)));
        }
        let db = env.d1("DB")?;
        let results = db
            .prepare(
                "SELECT version_number AS version, edited_at, encrypted_title \
                 FROM diary_versions \
                 WHERE diary_id = ?1 \
                 ORDER BY version_number DESC",
            )
            .bind(&[diary_id.into()])?
            .all()
            .await?;
        Ok(results.results::<DiaryVersionMeta>()?)
    }

    /// Get the encrypted body for a specific version.
    pub async fn get_version_body(
        env: &Env,
        user_id: &str,
        diary_id: &str,
        version: u32,
    ) -> Result<Option<(DiaryVersionMeta, Vec<u8>)>> {
        if Self::get_meta(env, user_id, diary_id).await?.is_none() {
            return Ok(None);
        }
        let db = env.d1("DB")?;
        let row = db
            .prepare(
                "SELECT version_number AS version, edited_at, encrypted_title, body_ref \
                 FROM diary_versions \
                 WHERE diary_id = ?1 AND version_number = ?2",
            )
            .bind(&[diary_id.into(), (version as f64).into()])?
            .first::<serde_json::Value>(None)
            .await?;

        let Some(row) = row else { return Ok(None) };

        let body_ref = row
            .get("body_ref")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let meta = DiaryVersionMeta {
            version,
            edited_at: row
                .get("edited_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            encrypted_title: row
                .get("encrypted_title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        };

        let body = Self::get_body(env, &body_ref).await?.unwrap_or_default();
        Ok(Some((meta, body)))
    }

    /// Delete a specific version (not the current one).
    pub async fn delete_version(
        env: &Env,
        user_id: &str,
        diary_id: &str,
        version: u32,
    ) -> Result<()> {
        // Must not delete the current version through this path.
        let current = Self::get_meta(env, user_id, diary_id)
            .await?
            .ok_or_else(|| Error::RustError(format!("diary {} not found", diary_id)))?;
        if version == current.version {
            return Err(Error::RustError(
                "Cannot delete the current version; edit forward instead.".into(),
            ));
        }
        let db = env.d1("DB")?;
        // Fetch body_ref before deletion for R2 cleanup.
        if let Some(row) = db
            .prepare(
                "SELECT body_ref FROM diary_versions \
                 WHERE diary_id = ?1 AND version_number = ?2",
            )
            .bind(&[diary_id.into(), (version as f64).into()])?
            .first::<serde_json::Value>(None)
            .await?
        {
            let r2_key = row
                .get("body_ref")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            // Only delete the R2 object if this version has a unique key
            // (version 1 shares the key with the original creation).
            if !r2_key.is_empty() && !r2_key.ends_with(&format!("/{}", diary_id)) {
                let _ = env.bucket("DIARY_BUCKET")?.delete(&r2_key).await;
            }
        }
        db.prepare("DELETE FROM diary_versions WHERE diary_id = ?1 AND version_number = ?2")
            .bind(&[diary_id.into(), (version as f64).into()])?
            .run()
            .await?;
        Ok(())
    }

    // ── Soft delete ───────────────────────────────────────────────────────

    pub async fn soft_delete(env: &Env, user_id: &str, diary_id: &str) -> Result<()> {
        let db = env.d1("DB")?;
        db.prepare(
            "UPDATE diaries SET deleted_at = datetime('now') \
             WHERE id = ?1 AND user_id = ?2",
        )
        .bind(&[diary_id.into(), user_id.into()])?
        .run()
        .await?;
        Ok(())
    }

    // ── Emergency erase (RFC 010) ────────────────────────────────────────

    /// Permanently delete all diary data for a user from R2 and D1.
    /// Includes all version history. This is irreversible.
    pub async fn erase_all_user_data(env: &Env, user_id: &str) -> Result<()> {
        let db = env.d1("DB")?;

        // Collect all R2 keys: main entries + version-specific objects.
        let main_keys = db
            .prepare("SELECT r2_key FROM diaries WHERE user_id = ?1")
            .bind(&[user_id.into()])?
            .all()
            .await?;

        // Collect diary_ids first for version key lookup.
        let diary_ids = db
            .prepare("SELECT id FROM diaries WHERE user_id = ?1")
            .bind(&[user_id.into()])?
            .all()
            .await?;

        let bucket = env.bucket("DIARY_BUCKET")?;

        // Delete main R2 objects.
        if let Ok(rows) = main_keys.results::<serde_json::Value>() {
            for row in rows {
                if let Some(key) = row.get("r2_key").and_then(|v| v.as_str()) {
                    let _ = bucket.delete(key).await;
                }
            }
        }

        // Delete version-specific R2 objects.
        if let Ok(ids) = diary_ids.results::<serde_json::Value>() {
            for id_row in ids {
                if let Some(did) = id_row.get("id").and_then(|v| v.as_str()) {
                    let version_keys = db
                        .prepare(
                            "SELECT body_ref FROM diary_versions \
                             WHERE diary_id = ?1 AND version_number > 1",
                        )
                        .bind(&[did.into()])?
                        .all()
                        .await?;
                    if let Ok(vrows) = version_keys.results::<serde_json::Value>() {
                        for vrow in vrows {
                            if let Some(key) = vrow.get("body_ref").and_then(|v| v.as_str()) {
                                let _ = bucket.delete(key).await;
                            }
                        }
                    }
                }
            }
        }

        // D1: diary_versions rows cascade-delete from diaries.
        db.prepare("DELETE FROM diaries WHERE user_id = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?;

        Ok(())
    }
}
