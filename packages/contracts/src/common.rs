//! 全 Worker で共通に使える原子的な型。

use serde::{Deserialize, Serialize};

/// 認証済みサブジェクトのサービス間表現。
/// OIDC `sub` + `email` のみを伝搬する。内部 Worker はこれ以上のクレームを見ない。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubjectRef {
    pub user_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,
}

/// 単純な一覧レスポンスのラッパ (pagination 未対応、Phase 2 で追加予定)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing<T> {
    pub items: Vec<T>,
    pub total: usize,
}
