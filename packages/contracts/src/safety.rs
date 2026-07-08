//! safety-worker との境界型。

use serde::{Deserialize, Serialize};

/// 安全性レベル
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    Safe,
    MildConcern,
    Crisis,
}

/// safety-worker による分類結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyClassification {
    pub level: SafetyLevel,
}

/// 危機検知時にユーザーに表示するリソース情報。
///
/// `message_reviewed` は `message` フィールドの翻訳が
/// 専門家監修済みかを示す。`false` の場合は暫定値であり、
/// 公開前のレビューが必要。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrisisResources {
    pub message: String,
    pub hotlines: Vec<HotlineInfo>,
    /// メッセージ翻訳が専門家監修済みか。
    #[serde(default)]
    pub message_reviewed: bool,
}

/// 支援機関の連絡先
///
/// `reviewed` フィールドは、当該翻訳・電話番号・URL が
/// 各言語のメンタルヘルス専門家のレビューを経たかを示す。
/// 専門家監修プロセスは `docs/i18n-review-flow.md` を参照。
///
/// `region` は対象地域 (例: "JP", "US", "ES", "International")。
/// 国ごとに hotline 番号が異なるため、UI で表示時に絞り込む用途。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HotlineInfo {
    pub name: String,
    pub phone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub language: String,
    /// 対象地域 (ISO 3166-1 alpha-2 国コード、または "International")
    #[serde(default)]
    pub region: String,
    /// 専門家による監修済みフラグ。デフォルト false (= 暫定値)。
    /// この値が false の hotline はクライアント側で
    /// "Pending review" マーカーを付けて表示することが推奨される。
    #[serde(default)]
    pub reviewed: bool,
}

// ───── ユーザー入力の安全性チェック ─────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyRequest {
    pub text: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyResponse {
    pub level: SafetyLevel,
    /// level が Crisis の場合のみ付与される
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<CrisisResources>,
}

// ───── AI 出力のガードレール ─────
//
// safety-worker は X-User-Id ヘッダから user_id を取って自身で trigger_keywords を
// D1 から取得するため、リクエストに triggers は含めない。

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOutputRequest {
    pub text: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOutputResponse {
    pub text: String,
    pub was_filtered: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_reason: Option<String>,
}

// ───── トリガーキーワード管理 ─────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TriggerKeyword {
    pub id: String,
    pub user_id: String,
    pub keyword: String,
    pub category: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTriggerRequest {
    pub keyword: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerListResponse {
    pub triggers: Vec<TriggerKeyword>,
}
