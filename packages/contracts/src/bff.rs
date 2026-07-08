//! BFF (Backend for Frontend) 専用の集約レスポンス型。
//!
//! これらの型は **bff-worker → ブラウザ** の境界にのみ現れる。
//! core Worker (journal/identity/safety/dialog) のリクエスト/レスポンス型は
//! それぞれの contract モジュール側に定義する。
//!
//! ## 設計方針: 部分的劣化 (partial degradation)
//!
//! 各フィールドは `Option<T>` または欠損可能な型で表現される。bff が複数 core を
//! 並列に呼ぶ際、一部 core が失敗しても他のフィールドは返す。これにより:
//!
//! - dashboard の一部 (例: 最新日記リスト) が一時的に取得できない場合でも、
//!   ユーザープロフィールやアクティブセッションは表示される
//! - 完全失敗時のみ 5xx を返す (全 core が失敗した、認証が無効など)

use crate::diary::DiaryMeta;
use crate::dialog::InterviewSession;
use crate::identity::UserRecord;
use serde::{Deserialize, Serialize};

/// `GET /api/dashboard` のレスポンス。
///
/// ブラウザ初期表示およびサーバー側 SSR (`/dashboard` ルート) の両方で使う。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    /// 認証済みユーザーのプロフィール。identity-worker が落ちている場合は `None`。
    /// `None` でも全体としては 200 を返す (UI 側で適切なフォールバック表示をする想定)。
    pub user: Option<UserRecord>,

    /// 最新の日記メタデータ (R2 本文は含まない)。journal-worker から取得。
    /// 失敗時は空ベクタ (`Vec::new()`)。
    #[serde(default)]
    pub recent_diaries: Vec<DiaryMeta>,

    /// 現在進行中のインタビューセッション。dialog-worker から取得。
    /// 進行中のセッションが無い、もしくは取得失敗の場合は `None`。
    pub active_session: Option<InterviewSession>,

    /// 各 core からの取得状況。UI 側でどのデータが新鮮かを判断する用途。
    pub status: DashboardStatus,
}

/// 各データソースの取得結果ステータス。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardStatus {
    pub user_ok: bool,
    pub recent_diaries_ok: bool,
    pub active_session_ok: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_dashboard_serializes() {
        let resp = DashboardResponse {
            user: None,
            recent_diaries: Vec::new(),
            active_session: None,
            status: DashboardStatus::default(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""user":null"#));
        assert!(json.contains(r#""recent_diaries":[]"#));
        assert!(json.contains(r#""active_session":null"#));
        assert!(json.contains(r#""user_ok":false"#));
    }

    #[test]
    fn dashboard_with_user_serializes() {
        let resp = DashboardResponse {
            user: Some(UserRecord {
                id: "u1".to_string(),
                email: "test@example.com".to_string(),
                display_name: Some("Test".to_string()),
                language: "ja".to_string(),
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
                onboarding_completed: true,
                kdf_salt: None,
                wrapped_dek: None,
                kdf_params_json: None,
            }),
            recent_diaries: Vec::new(),
            active_session: None,
            status: DashboardStatus {
                user_ok: true,
                recent_diaries_ok: true,
                active_session_ok: true,
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""id":"u1""#));
        assert!(json.contains(r#""user_ok":true"#));
    }

    #[test]
    fn dashboard_status_partial_degradation() {
        let s = DashboardStatus {
            user_ok: true,
            recent_diaries_ok: false, // journal が落ちた
            active_session_ok: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains(r#""recent_diaries_ok":false"#));
    }
}
