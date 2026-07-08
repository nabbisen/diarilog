//! Types shared between journal-worker and callers.
//!
//! ## E2EE field naming (RFC 011)
//!
//! Plaintext `title` and `mood_score` fields are replaced by
//! `encrypted_title` and `encrypted_mood` (base64-encoded AES-GCM ciphertext,
//! nonce-prepended). The server stores only ciphertext. The client decrypts
//! after receiving and re-encrypts before sending.
//!
//! Fields marked `#[serde(default)]` accept old records that predate RFC 011
//! (where title was cleartext). New writes must always send the encrypted form.

use serde::{Deserialize, Serialize};

/// Public metadata for a diary entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiaryMeta {
    pub id: String,
    pub user_id: String,
    /// R2 object key for the encrypted body. Random, not a secret.
    pub r2_key: String,
    /// Encrypted title: base64(nonce ‖ AES-GCM(DEK, title_plaintext) ‖ tag).
    /// Absent on pre-RFC-011 entries (legacy field `title` may exist instead).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_title: Option<String>,
    /// Encrypted mood score (1–5): base64(nonce ‖ AES-GCM(DEK, mood_bytes) ‖ tag).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_mood: Option<String>,
    pub word_count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interview_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Version number for this entry (1 = original, increments on each edit).
    /// Absent on entries created before RFC 012.
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

/// Create a new diary entry (client sends pre-encrypted data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiaryRequest {
    /// AES-GCM encrypted body: base64(nonce ‖ ciphertext ‖ tag).
    pub encrypted_body: String,
    /// Encrypted title (required for new entries).
    pub encrypted_title: String,
    /// Encrypted mood score (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_mood: Option<String>,
    pub word_count: Option<i32>,
    pub interview_id: Option<String>,
}

/// Response on successful creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiaryResponse {
    pub id: String,
    pub created: bool,
}

/// Update an existing entry (produces a new version — RFC 012).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiaryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_mood: Option<String>,
    pub word_count: Option<i32>,
}

/// Paginated diary list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryListResponse {
    pub entries: Vec<DiaryMeta>,
    pub total: usize,
}

/// Full diary detail including encrypted body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryDetailResponse {
    pub meta: DiaryMeta,
    /// Base64-encoded AES-GCM ciphertext (nonce prepended).
    pub encrypted_body: String,
}

/// Version history entry (RFC 012).
/// Body is fetched separately on demand to avoid unnecessary R2 reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryVersionMeta {
    pub version: u32,
    pub edited_at: String,
    pub encrypted_title: String,
}

/// List of version history entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryVersionListResponse {
    pub diary_id: String,
    pub versions: Vec<DiaryVersionMeta>,
}

/// Full content of a specific version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryVersionDetailResponse {
    pub diary_id: String,
    pub version: u32,
    pub edited_at: String,
    pub encrypted_title: String,
    pub encrypted_body: String,
}

/// Offline sync request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<String>,
    pub new_entries: Vec<CreateDiaryRequest>,
    pub updated_entries: Vec<SyncUpdateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncUpdateEntry {
    pub diary_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_mood: Option<String>,
    pub word_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub server_updates: Vec<DiaryMeta>,
    pub created_ids: Vec<String>,
    pub synced_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diary_meta_version_defaults_to_1() {
        let json = r#"{
            "id":"d1","user_id":"u1","r2_key":"k1",
            "word_count":100,"created_at":"2026-01-01","updated_at":"2026-01-01"
        }"#;
        let m: DiaryMeta = serde_json::from_str(json).unwrap();
        assert_eq!(m.version, 1);
        assert!(m.encrypted_title.is_none());
        assert!(m.encrypted_mood.is_none());
    }

    #[test]
    fn diary_meta_with_encrypted_fields() {
        let json = r#"{
            "id":"d2","user_id":"u1","r2_key":"k2",
            "encrypted_title":"AAAA","encrypted_mood":"BBBB",
            "word_count":200,"created_at":"2026-01-02","updated_at":"2026-01-02",
            "version":3
        }"#;
        let m: DiaryMeta = serde_json::from_str(json).unwrap();
        assert_eq!(m.version, 3);
        assert_eq!(m.encrypted_title.as_deref(), Some("AAAA"));
    }

    #[test]
    fn diary_version_meta_roundtrip() {
        let v = DiaryVersionMeta {
            version: 2,
            edited_at: "2026-05-01T10:00:00Z".to_string(),
            encrypted_title: "CCCC".to_string(),
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: DiaryVersionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, 2);
        assert_eq!(back.encrypted_title, "CCCC");
    }

    #[test]
    fn create_diary_request_requires_encrypted_title() {
        let req = CreateDiaryRequest {
            encrypted_body: "EEEE".to_string(),
            encrypted_title: "FFFF".to_string(),
            encrypted_mood: None,
            word_count: Some(50),
            interview_id: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("encrypted_title"));
        assert!(!json.contains("encrypted_mood")); // skipped when None
    }
}
