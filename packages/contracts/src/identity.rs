//! Types shared between identity-worker and callers.

use serde::{Deserialize, Serialize};

/// User record persisted in D1 `users`.
///
/// ## E2EE fields (RFC 011)
///
/// `kdf_salt`, `wrapped_dek`, and `kdf_params_json` are part of the
/// passphrase-based E2EE key model. They are stored server-side but do not
/// help an attacker read content: the salt is not secret, and the wrapped DEK
/// is useless without the user's passphrase.
///
/// `onboarding_completed` gates access to the main dashboard: if `false` the
/// client should redirect to the passphrase-setup flow before allowing writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub language: String,
    pub created_at: String,
    pub updated_at: String,
    // ── E2EE fields (RFC 011) ────────────────────────────────────────────
    /// Whether the user has completed first-session setup (passphrase + language).
    #[serde(default)]
    pub onboarding_completed: bool,
    /// Base64-encoded random KDF salt (16 bytes minimum). Not secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf_salt: Option<String>,
    /// Base64-encoded wrapped DEK (nonce ‖ ciphertext ‖ tag).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrapped_dek: Option<String>,
    /// JSON-serialized `crypto::KdfParams`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf_params_json: Option<String>,
}

/// Request to complete passphrase setup during first-session onboarding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupPassphraseRequest {
    /// Base64-encoded KDF salt generated client-side.
    pub kdf_salt: String,
    /// Base64-encoded wrapped DEK: nonce ‖ AES-GCM-enc(KEK, DEK_MAGIC ‖ DEK) ‖ tag.
    pub wrapped_dek: String,
    /// JSON-serialized `crypto::KdfParams` matching the salt.
    pub kdf_params_json: String,
}

/// Response returned to the client after successful passphrase setup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupPassphraseResponse {
    pub onboarding_completed: bool,
}

/// Profile update request (non-E2EE fields only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub language: Option<String>,
}

/// First-registration upsert (called from gateway on OIDC callback).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnsureUserRequest {
    pub user_id: String,
    pub email: String,
}

/// Onboarding status returned after OIDC login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingStatus {
    pub completed: bool,
    /// Non-null when `completed` is true: salt, wrapped_dek, and params
    /// needed for the client to unlock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf_salt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrapped_dek: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kdf_params_json: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_record_onboarding_defaults_false() {
        let json = r#"{
            "id":"u1","email":"a@b.com","language":"en",
            "created_at":"2026-01-01","updated_at":"2026-01-01"
        }"#;
        let u: UserRecord = serde_json::from_str(json).unwrap();
        assert!(!u.onboarding_completed);
        assert!(u.kdf_salt.is_none());
        assert!(u.wrapped_dek.is_none());
    }

    #[test]
    fn user_record_with_e2ee_fields() {
        let json = r#"{
            "id":"u1","email":"a@b.com","language":"en",
            "created_at":"2026-01-01","updated_at":"2026-01-01",
            "onboarding_completed":true,
            "kdf_salt":"c2FsdA==",
            "wrapped_dek":"d2Vl",
            "kdf_params_json":"{}"
        }"#;
        let u: UserRecord = serde_json::from_str(json).unwrap();
        assert!(u.onboarding_completed);
        assert_eq!(u.kdf_salt.as_deref(), Some("c2FsdA=="));
    }

    #[test]
    fn onboarding_status_roundtrip() {
        let s = OnboardingStatus {
            completed: false,
            kdf_salt: None,
            wrapped_dek: None,
            kdf_params_json: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: OnboardingStatus = serde_json::from_str(&json).unwrap();
        assert!(!back.completed);
        assert!(back.kdf_salt.is_none());
    }
}
