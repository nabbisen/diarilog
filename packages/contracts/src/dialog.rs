//! dialog-worker との境界型。

use serde::{Deserialize, Serialize};

/// インタビューセッションの状態 (公開される表現)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
    Abandoned,
    CrisisPaused,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
            Self::CrisisPaused => "crisis_paused",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "completed" => Self::Completed,
            "abandoned" => Self::Abandoned,
            "crisis_paused" => Self::CrisisPaused,
            _ => Self::Active,
        }
    }
}

/// 回答方式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnswerType {
    Free,
    Choice,
    Scale,
}

impl AnswerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Choice => "choice",
            Self::Scale => "scale",
        }
    }
}

/// インタビューの個別ターン (一問一答)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewTurn {
    pub id: String,
    pub session_id: String,
    pub turn_order: i32,
    pub question: String,
    pub answer_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    pub created_at: String,
}

/// インタビューセッション (公開表現)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewSession {
    pub id: String,
    pub user_id: String,
    pub status: String,
    pub question_count: i32,
    pub language: String,
    pub created_at: String,
    pub updated_at: String,
}

// ───── セッション開始 ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSessionRequest {
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSessionResponse {
    pub session_id: String,
    pub first_question: QuestionPayload,
}

/// AI が生成した質問
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionPayload {
    pub turn_id: String,
    pub turn_order: i32,
    pub question: String,
    pub answer_type: AnswerType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
}

// ───── 回答送信 ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitAnswerRequest {
    pub session_id: String,
    pub turn_id: String,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitAnswerResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_question: Option<QuestionPayload>,
    pub session_completed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crisis_resources: Option<crate::safety::CrisisResources>,
}

// ───── セッション詳細 ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetailResponse {
    pub session: InterviewSession,
    pub turns: Vec<InterviewTurn>,
}

// ───── 下書きサジェスト ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_input: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestResponse {
    pub drafts: Vec<DraftOption>,
    pub remaining_uses_today: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftOption {
    pub id: String,
    pub text: String,
    pub style: String,
}

/// `GET /interview/active` のレスポンス。
/// 進行中のセッションがあれば `Some`、なければ `None`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSessionResponse {
    pub session: Option<InterviewSession>,
}
