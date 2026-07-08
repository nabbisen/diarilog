//! Worker 間で構造化エラーを返すための共通型定義。
//!
//! すべての Worker は 4xx/5xx のレスポンスボディを
//! `ApiError` の JSON シリアライズ結果に揃える。

use serde::{Deserialize, Serialize};

/// API エラーの統一表現。Service Bindings 越しのレスポンスでも、
/// 外部公開 API のエラーレスポンスでも同じ形を使う。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            trace_id: None,
        }
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// 標準の HTTP ステータスコード (ポリシー)
    pub fn default_status(&self) -> u16 {
        self.code.default_status()
    }
}

/// エラー区分の列挙。
///
/// `#[serde(rename_all = "snake_case")]` により、ワイヤ上は
/// `"unauthorized"`, `"validation_failed"`, ... のような形で表現される。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// 認証なし / 無効なトークン
    Unauthorized,
    /// 認可なし (権限不足)
    Forbidden,
    /// リソースが見つからない
    NotFound,
    /// 入力バリデーションエラー
    ValidationFailed,
    /// 利用ポリシーに反するメソッド
    MethodNotAllowed,
    /// 危機的状況の検知 (self-harm 等)
    CrisisDetected,
    /// レート制限超過
    RateLimited,
    /// 上流サービス (他 Worker、AI、外部 API) からのエラー
    UpstreamFailure,
    /// 本サービス内部の不整合・未分類エラー
    Internal,
}

impl ErrorCode {
    pub fn default_status(&self) -> u16 {
        match self {
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::ValidationFailed => 400,
            Self::MethodNotAllowed => 405,
            Self::CrisisDetected => 200, // 危機検知は「正常なドメイン応答」として 200 を返し、body で伝える
            Self::RateLimited => 429,
            Self::UpstreamFailure => 502,
            Self::Internal => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let e = ApiError::new(ErrorCode::ValidationFailed, "bad field")
            .with_trace_id("trace-1");
        let json = serde_json::to_string(&e).unwrap();
        let decoded: ApiError = serde_json::from_str(&json).unwrap();
        assert_eq!(e, decoded);
    }

    #[test]
    fn error_code_serialization_snake_case() {
        let code = ErrorCode::ValidationFailed;
        assert_eq!(
            serde_json::to_string(&code).unwrap(),
            "\"validation_failed\""
        );
    }

    #[test]
    fn default_status_mapping() {
        assert_eq!(ErrorCode::Unauthorized.default_status(), 401);
        assert_eq!(ErrorCode::NotFound.default_status(), 404);
        assert_eq!(ErrorCode::UpstreamFailure.default_status(), 502);
        assert_eq!(ErrorCode::Internal.default_status(), 500);
    }

    #[test]
    fn trace_id_omitted_when_none() {
        let e = ApiError::new(ErrorCode::NotFound, "no such record");
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("trace_id"));
    }
}
