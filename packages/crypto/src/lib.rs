//! Cryptographic primitives for diarilog end-to-end encryption (RFC 011).
//!
//! ## Design
//!
//! All actual cryptographic operations (Argon2id, AES-256-GCM) happen in the
//! **browser** using the Web Crypto API (`crypto.subtle`) or a WASM-compiled
//! crate. This server-side crate defines only:
//!
//! - The serializable types that flow between client and server
//!   (`KdfParams`, `WrappedDek`, `EncryptedField`)
//! - The validation rules for those types
//! - Constants that both client and server must agree on
//!
//! The actual derive/encrypt/decrypt functions live in `packages/web-app`
//! (compiled to WASM, running in the browser), not here.
//!
//! ## Key hierarchy (summary)
//!
//! ```text
//! passphrase + salt
//!       │  Argon2id
//!       ▼
//!   KEK (key-encryption key, 32 bytes, never leaves the browser)
//!       │  AES-256-GCM wrap
//!       ▼
//!   DEK (data encryption key, 32 bytes, never leaves the browser)
//!       │  AES-256-GCM encrypt (per-field random nonce)
//!       ▼
//!   ciphertext stored in D1 (EncryptedField) or R2 (raw bytes)
//! ```
//!
//! The server holds:
//! - `kdf_salt` — random, not secret
//! - `wrapped_dek` — DEK encrypted under KEK; useless without the passphrase
//! - `kdf_params` — Argon2id cost parameters
//!
//! The server does NOT hold:
//! - the passphrase
//! - the KEK
//! - the DEK
//!
//! See RFC 011 for full threat model and alternative analysis.

use serde::{Deserialize, Serialize};

// ── Constants ────────────────────────────────────────────────────────────────

/// Length in bytes of the Data Encryption Key.
pub const DEK_LEN: usize = 32;

/// Length in bytes of the AES-GCM nonce (96-bit, the standard for AES-GCM).
pub const NONCE_LEN: usize = 12;

/// Length in bytes of the AES-GCM authentication tag.
pub const TAG_LEN: usize = 16;

/// Magic prefix prepended to the plaintext DEK before wrapping. The browser
/// checks this after unwrapping: if the prefix is absent, the passphrase was
/// wrong (or the wrapped_dek is corrupted).
///
/// 4 bytes chosen to be short but unambiguous. The literal "DLGK" stands for
/// "diarilog key".
pub const DEK_MAGIC: &[u8; 4] = b"DLGK";

/// Minimum length of a KDF salt in bytes.
pub const SALT_MIN_LEN: usize = 16;

// ── KDF parameters ────────────────────────────────────────────────────────────

/// Which key-derivation algorithm is in use.
///
/// Versioned here so that future migrations can be detected by the client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KdfAlgorithm {
    /// Argon2id — the default and only supported algorithm in v2.7+.
    Argon2id,
}

/// Cost parameters for the key-derivation function. Stored in `UserRecord`
/// alongside the wrapped DEK and KDF salt.
///
/// The parameters travel from server to client during the unlock flow so the
/// client can re-derive the KEK with the same settings used at passphrase
/// setup.
///
/// See RFC 011 §Design — the specific values are chosen to target ≈ 250 ms
/// on a 2020-era mid-range mobile device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KdfParams {
    pub algorithm: KdfAlgorithm,
    /// Memory cost in KiB (Argon2 m parameter).
    pub memory_kib: u32,
    /// Number of iterations (Argon2 t parameter).
    pub iterations: u32,
    /// Degree of parallelism (Argon2 p parameter).
    pub parallelism: u32,
}

impl KdfParams {
    /// Production default targeting ≈ 250 ms on a mid-range 2020 phone.
    ///
    /// Parameters are intentionally conservative. They can be tightened in a
    /// future `KdfParams` version; the versioning allows the client to
    /// re-derive on upgrade.
    pub fn default_argon2id() -> Self {
        Self {
            algorithm: KdfAlgorithm::Argon2id,
            memory_kib: 19 * 1024, // 19 MiB
            iterations: 2,
            parallelism: 1,
        }
    }

    /// Lightweight parameters for tests where wall-clock time matters.
    #[cfg(test)]
    pub fn fast_for_test() -> Self {
        Self {
            algorithm: KdfAlgorithm::Argon2id,
            memory_kib: 8,
            iterations: 1,
            parallelism: 1,
        }
    }

    /// Validate that the parameters are within acceptable bounds.
    ///
    /// Rejects absurdly low values that would make the KDF trivial to attack,
    /// and absurdly high values that would DoS the client.
    pub fn validate(&self) -> Result<(), String> {
        if self.memory_kib < 8 {
            return Err("memory_kib must be at least 8".to_string());
        }
        if self.memory_kib > 1_048_576 {
            // 1 GiB upper bound
            return Err("memory_kib exceeds 1 GiB limit".to_string());
        }
        if self.iterations == 0 {
            return Err("iterations must be at least 1".to_string());
        }
        if self.parallelism == 0 {
            return Err("parallelism must be at least 1".to_string());
        }
        Ok(())
    }
}

// ── Wrapped DEK ───────────────────────────────────────────────────────────────

/// The DEK encrypted (wrapped) under the KEK. Stored in `UserRecord`.
///
/// On the wire / in D1 this is base64-encoded binary. The layout of the
/// binary is:
///
/// ```text
/// [ 12-byte nonce ][ 4-byte magic in plaintext after decrypt ][ 32-byte DEK ][ 16-byte tag ]
/// ^^^^^^^^^^^^^^^^^                                                           ^^^^^^^^^^^^^^
/// nonce prepended   The magic + DEK are the plaintext encrypted              AES-GCM auth tag
///                   under KEK with the nonce above.
/// ```
///
/// Total wrapped size: 12 + 4 + 32 + 16 = 64 bytes.
/// Base64-encoded: 88 characters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WrappedDek(
    /// Base64-encoded wrapped DEK bytes.
    pub String,
);

impl WrappedDek {
    /// Minimum expected length of the base64-encoded string.
    /// 64 raw bytes → 88 base64 chars (no padding variant may be shorter).
    pub const MIN_B64_LEN: usize = 80;

    pub fn new(b64: impl Into<String>) -> Self {
        Self(b64.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate that this looks like a plausibly-formed wrapped DEK.
    /// Does not decrypt; just checks length.
    pub fn validate(&self) -> Result<(), String> {
        if self.0.len() < Self::MIN_B64_LEN {
            return Err(format!(
                "wrapped_dek too short: {} chars, need at least {}",
                self.0.len(),
                Self::MIN_B64_LEN
            ));
        }
        Ok(())
    }
}

// ── Encrypted field ───────────────────────────────────────────────────────────

/// A server-stored ciphertext for a single D1 field (title, mood, etc.).
///
/// Layout (raw bytes before base64):
///
/// ```text
/// [ 12-byte nonce ][ ciphertext ][ 16-byte tag ]
/// ```
///
/// The nonce is randomly generated per-encryption. The DEK is the key.
/// On the wire this is stored base64-encoded in D1.
///
/// For empty-string plaintexts the ciphertext portion is zero-length, but
/// the nonce and tag are still present (28 bytes total). This ensures the
/// server cannot distinguish an empty title from a missing one beyond what
/// the column's nullability already reveals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EncryptedField(
    /// Base64-encoded [ nonce | ciphertext | tag ].
    pub String,
);

impl EncryptedField {
    /// Minimum size: nonce (12) + tag (16) → 28 bytes → 40 base64 chars.
    pub const MIN_B64_LEN: usize = 38;

    pub fn new(b64: impl Into<String>) -> Self {
        Self(b64.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.0.len() < Self::MIN_B64_LEN {
            return Err(format!(
                "encrypted_field too short: {} chars, need at least {}",
                self.0.len(),
                Self::MIN_B64_LEN
            ));
        }
        Ok(())
    }
}

// ── Passphrase change request ─────────────────────────────────────────────────

/// Payload for `POST /api/me/passphrase` (RFC 011 §Design).
///
/// Sent by the client after it has:
/// 1. Re-derived the old KEK (to verify the current passphrase is correct).
/// 2. Unwrapped the existing DEK using the old KEK.
/// 3. Generated a new random salt and derived a new KEK from the new passphrase.
/// 4. Re-wrapped the same DEK under the new KEK.
///
/// The server replaces `kdf_salt`, `wrapped_dek`, and `kdf_params` atomically.
/// The DEK (and therefore all ciphertext) is unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassphraseChangeRequest {
    pub new_salt: String,
    pub new_wrapped_dek: WrappedDek,
    pub new_kdf_params: KdfParams,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kdf_params_default_validates() {
        KdfParams::default_argon2id().validate().unwrap();
    }

    #[test]
    fn kdf_params_rejects_zero_memory() {
        let p = KdfParams {
            algorithm: KdfAlgorithm::Argon2id,
            memory_kib: 0,
            iterations: 1,
            parallelism: 1,
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn kdf_params_rejects_zero_iterations() {
        let p = KdfParams {
            algorithm: KdfAlgorithm::Argon2id,
            memory_kib: 8,
            iterations: 0,
            parallelism: 1,
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn kdf_params_serializes_roundtrip() {
        let p = KdfParams::default_argon2id();
        let json = serde_json::to_string(&p).unwrap();
        let back: KdfParams = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn wrapped_dek_rejects_short_value() {
        let w = WrappedDek::new("tooshort");
        assert!(w.validate().is_err());
    }

    #[test]
    fn wrapped_dek_accepts_min_length() {
        // 88-char base64 represents 64 bytes — the correct wrapped DEK size.
        let valid = "A".repeat(88);
        let w = WrappedDek::new(valid);
        assert!(w.validate().is_ok());
    }

    #[test]
    fn encrypted_field_rejects_short_value() {
        let f = EncryptedField::new("x");
        assert!(f.validate().is_err());
    }

    #[test]
    fn encrypted_field_accepts_min_length() {
        // 40-char base64 represents the minimum 28 bytes (nonce + tag only).
        let valid = "A".repeat(40);
        let f = EncryptedField::new(valid);
        assert!(f.validate().is_ok());
    }

    #[test]
    fn passphrase_change_request_serializes_roundtrip() {
        let req = PassphraseChangeRequest {
            new_salt: "abc123".to_string(),
            new_wrapped_dek: WrappedDek::new("A".repeat(88)),
            new_kdf_params: KdfParams::default_argon2id(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: PassphraseChangeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.new_salt, "abc123");
        assert_eq!(back.new_wrapped_dek, req.new_wrapped_dek);
        assert_eq!(back.new_kdf_params, req.new_kdf_params);
    }

    #[test]
    fn dek_magic_constant_is_four_bytes() {
        assert_eq!(DEK_MAGIC.len(), 4);
    }
}
