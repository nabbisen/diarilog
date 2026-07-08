-- Migration 0002: E2EE key model (RFC 011)
--
-- Adds the passphrase / key-derivation fields to the users table and
-- replaces plaintext title/mood with encrypted equivalents on the diaries
-- table.
--
-- Backward compatibility:
--   * All new columns are nullable (or have safe defaults) so existing rows
--     continue to be readable.
--   * Existing rows have onboarding_completed = 0, which will cause the
--     gateway to route those users through first-session setup on next sign-in.
--   * encrypted_title and encrypted_mood being NULL signals a legacy entry;
--     the journal-worker will treat such entries as "title unknown" until the
--     user edits them.
--   * The legacy `title` and `mood_score` columns are kept for one release to
--     allow a smooth migration; they will be dropped in migration 0004.

-- ── users ────────────────────────────────────────────────────────────────────

ALTER TABLE users ADD COLUMN onboarding_completed INTEGER NOT NULL DEFAULT 0;

-- KDF salt: 16+ random bytes, base64-encoded. Not secret.
ALTER TABLE users ADD COLUMN kdf_salt TEXT;

-- Wrapped DEK: AES-GCM(KEK, DEK_MAGIC || DEK) with nonce prepended, base64.
ALTER TABLE users ADD COLUMN wrapped_dek TEXT;

-- JSON-serialized KdfParams (algorithm, memory_kib, iterations, parallelism).
ALTER TABLE users ADD COLUMN kdf_params_json TEXT;

-- ── diaries ──────────────────────────────────────────────────────────────────

-- Encrypted title: base64(nonce || AES-GCM(DEK, title) || tag).
-- NULL on legacy entries created before RFC 011.
ALTER TABLE diaries ADD COLUMN encrypted_title TEXT;

-- Encrypted mood score: base64(nonce || AES-GCM(DEK, mood_bytes) || tag).
-- NULL when no mood was recorded, or on legacy entries.
ALTER TABLE diaries ADD COLUMN encrypted_mood TEXT;

-- Version counter: increments on each edit (RFC 012 prep).
-- Default 1 for all existing entries.
ALTER TABLE diaries ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
