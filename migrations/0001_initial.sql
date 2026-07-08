-- ============================================================
-- Migration: 0001_initial
-- Description: Core schema for trauma-journal-platform Phase 1
-- ============================================================

-- Users table: stores user identity from Cloudflare Access JWT
CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,                     -- Cloudflare Access sub claim
    email       TEXT,                                 -- From JWT (optional, may be masked)
    display_name TEXT DEFAULT '',
    language    TEXT DEFAULT 'ja',                    -- Preferred UI language
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT DEFAULT (datetime('now'))
);

-- Diary entries metadata (actual body stored in R2 as encrypted blob)
CREATE TABLE IF NOT EXISTS diaries (
    id          TEXT PRIMARY KEY,                     -- UUID v4
    user_id     TEXT NOT NULL,
    r2_key      TEXT NOT NULL,                        -- R2 object key for encrypted body
    title       TEXT DEFAULT '',                      -- Optional user-set title (encrypted client-side)
    mood_score  INTEGER,                              -- 1-5 optional mood indicator
    word_count  INTEGER DEFAULT 0,                    -- Approximate word count
    interview_id TEXT,                                -- Link to originating interview session
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT DEFAULT (datetime('now')),
    deleted_at  TEXT,                                  -- Soft-delete for emergency erase
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_diaries_user_id ON diaries(user_id);
CREATE INDEX IF NOT EXISTS idx_diaries_created_at ON diaries(created_at);
CREATE INDEX IF NOT EXISTS idx_diaries_deleted ON diaries(deleted_at);

-- Interview sessions: tracks multi-turn conversations
CREATE TABLE IF NOT EXISTS interview_sessions (
    id          TEXT PRIMARY KEY,                     -- UUID v4
    user_id     TEXT NOT NULL,
    status      TEXT DEFAULT 'active'
                CHECK(status IN ('active', 'completed', 'abandoned', 'crisis_paused')),
    question_count INTEGER DEFAULT 0,
    language    TEXT DEFAULT 'ja',
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON interview_sessions(user_id);

-- Interview turns: individual Q&A pairs within a session
CREATE TABLE IF NOT EXISTS interview_turns (
    id          TEXT PRIMARY KEY,                     -- UUID v4
    session_id  TEXT NOT NULL,
    turn_order  INTEGER NOT NULL,                     -- Sequence number within session
    question    TEXT NOT NULL,                         -- AI-generated question
    answer_type TEXT DEFAULT 'free'
                CHECK(answer_type IN ('free', 'choice', 'scale')),
    choices     TEXT,                                  -- JSON array for choice-type questions
    answer      TEXT,                                  -- User's response (encrypted client-side)
    created_at  TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES interview_sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_turns_session_id ON interview_turns(session_id);

-- Trigger keywords: topics the user wants to avoid
CREATE TABLE IF NOT EXISTS trigger_keywords (
    id          TEXT PRIMARY KEY,                     -- UUID v4
    user_id     TEXT NOT NULL,
    keyword     TEXT NOT NULL,                         -- The avoided topic/word
    category    TEXT DEFAULT 'general',                -- e.g., violence, loss, abuse
    is_active   INTEGER DEFAULT 1,
    created_at  TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_triggers_user ON trigger_keywords(user_id);

-- Suggestion usage log: enforce daily limits on AI draft generation
CREATE TABLE IF NOT EXISTS suggestion_logs (
    id          TEXT PRIMARY KEY,                     -- UUID v4
    user_id     TEXT NOT NULL,
    char_count  INTEGER NOT NULL,
    created_at  TEXT DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_suggestion_logs_user_date
    ON suggestion_logs(user_id, created_at);
