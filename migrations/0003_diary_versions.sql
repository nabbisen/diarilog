-- Migration 0003: Entry version history (RFC 012)
--
-- Adds the diary_versions table that records every saved edit to a diary
-- entry. The diaries table continues to hold the *current* state for fast
-- reads; this table holds the history.
--
-- Soft cap: 20 versions per entry enforced at write time in journal-worker.
-- The original version (version_number = 1) is always preserved.

CREATE TABLE IF NOT EXISTS diary_versions (
    -- Unique identifier for this specific version row.
    id TEXT NOT NULL PRIMARY KEY,

    -- Foreign key to diaries.id. ON DELETE CASCADE so that erasing a diary
    -- (including emergency-erase) automatically removes all its versions.
    diary_id TEXT NOT NULL REFERENCES diaries(id) ON DELETE CASCADE,

    -- 1 = original write, 2 = first edit, 3 = second edit, etc.
    version_number INTEGER NOT NULL,

    -- When this version was saved (ISO 8601 UTC).
    edited_at TEXT NOT NULL,

    -- R2 object key for this version's encrypted body.
    -- Each version stores a separate R2 object so they can be fetched
    -- independently and deleted independently.
    body_ref TEXT NOT NULL,

    -- Encrypted title for this version (same format as diaries.encrypted_title).
    encrypted_title TEXT NOT NULL,

    -- Encrypted mood score for this version (nullable, same as diaries.encrypted_mood).
    encrypted_mood TEXT,

    UNIQUE(diary_id, version_number)
);

-- Lookup index: fetch version list for a given diary, most-recent-first.
CREATE INDEX IF NOT EXISTS idx_diary_versions_diary
    ON diary_versions(diary_id, version_number DESC);

-- Back-fill: give every existing entry a version 1 row.
-- Uses the entry's own body_ref and encrypted_title.
-- encrypted_title may be NULL on legacy entries; we allow that in version 1
-- so the row is always present.
INSERT INTO diary_versions (id, diary_id, version_number, edited_at, body_ref, encrypted_title, encrypted_mood)
SELECT
    id || '_v1'           AS id,
    id                    AS diary_id,
    1                     AS version_number,
    created_at            AS edited_at,
    r2_key                AS body_ref,
    COALESCE(encrypted_title, '')  AS encrypted_title,
    encrypted_mood
FROM diaries;
