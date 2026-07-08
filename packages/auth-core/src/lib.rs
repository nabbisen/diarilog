//! 汎用 OpenID Connect (OIDC) クライアント。
//!
//! 本モジュールは特定の IDaaS (Auth0, Okta, Keycloak, Azure AD, Cloudflare Access など)
//! に依存しない、OIDC 準拠プロバイダであればどこでも動作する認証機構を提供する。
//!
//! ## 機能
//! - OIDC Discovery (`/.well-known/openid-configuration`) による provider メタデータ取得
//! - JWKS エンドポイントからの公開鍵取得と KV キャッシュ (TTL 付き)
//! - RS256 / RS384 / RS512 署名検証 (WebCrypto API 経由)
//! - 標準クレーム検証: `iss`, `aud`, `exp`, `iat`, `nbf`
//!
//! ## 設定 (wrangler.toml の [vars])
//! - `OIDC_ISSUER`   必須. Discovery の起点 URL (例: `https://example.auth0.com/`)
//! - `OIDC_AUDIENCE` 必須. 本サービスの `client_id` または audience 識別子
//! - `OIDC_JWKS_TTL_SEC`  任意. JWKS キャッシュの TTL 秒数 (既定 3600)
//!
//! ## クライアントへの要求
//! リクエストは `Authorization: Bearer <id_token>` ヘッダで ID トークンを送る。

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use js_sys::{Object, Reflect, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CryptoKey, SubtleCrypto};
use worker::*;

// ================================================================
// OIDC 設定
// ================================================================

/// OIDC クライアントの実行時設定
#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// Issuer URL (末尾スラッシュは許容)
    pub issuer: String,
    /// 期待する audience (通常は client_id)
    pub audience: String,
    /// JWKS キャッシュ TTL 秒
    pub jwks_ttl_sec: u64,
}

impl OidcConfig {
    /// `Env` から設定を読み込む。
    /// 必須変数が欠けている場合はエラー。
    pub fn from_env(env: &Env) -> Result<Self> {
        let issuer = env
            .var("OIDC_ISSUER")
            .map(|v| v.to_string())
            .map_err(|_| Error::RustError("OIDC_ISSUER is not set in [vars]".into()))?;

        let audience = env
            .var("OIDC_AUDIENCE")
            .map(|v| v.to_string())
            .map_err(|_| Error::RustError("OIDC_AUDIENCE is not set in [vars]".into()))?;

        let jwks_ttl_sec = env
            .var("OIDC_JWKS_TTL_SEC")
            .ok()
            .and_then(|v| v.to_string().parse().ok())
            .unwrap_or(3600);

        // 末尾スラッシュを削除して正規化
        let issuer = issuer.trim_end_matches('/').to_string();

        Ok(Self {
            issuer,
            audience,
            jwks_ttl_sec,
        })
    }
}

// ================================================================
// OIDC Discovery ドキュメント
// ================================================================

/// `/.well-known/openid-configuration` のスキーマ (必要フィールドのみ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryDocument {
    pub issuer: String,
    pub jwks_uri: String,
    #[serde(default)]
    pub id_token_signing_alg_values_supported: Vec<String>,
}

// ================================================================
// JWKS
// ================================================================

/// JSON Web Key Set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkSet {
    pub keys: Vec<Jwk>,
}

/// 公開鍵 (RSA のみを想定; EC は将来対応)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    pub kid: Option<String>,
    pub kty: String,
    #[serde(rename = "use")]
    pub use_: Option<String>,
    pub alg: Option<String>,
    /// RSA modulus (base64url)
    pub n: Option<String>,
    /// RSA exponent (base64url)
    pub e: Option<String>,
}

// ================================================================
// JWT ヘッダ / クレーム
// ================================================================

/// JWT ヘッダ
#[derive(Debug, Clone, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    pub kid: Option<String>,
    #[serde(default)]
    pub typ: Option<String>,
}

/// OIDC ID トークンの標準クレーム
#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    /// Subject — ユーザー一意識別子
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Audience — 単一値・配列の両方を許容
    #[serde(deserialize_with = "deserialize_audience")]
    pub aud: Vec<String>,
    /// Expiration time (Unix 秒)
    pub exp: i64,
    /// Issued at (Unix 秒)
    #[serde(default)]
    pub iat: Option<i64>,
    /// Not before (Unix 秒)
    #[serde(default)]
    pub nbf: Option<i64>,
    /// Email (任意)
    #[serde(default)]
    pub email: Option<String>,
    /// 表示名 (任意)
    #[serde(default)]
    pub name: Option<String>,
    /// 推奨ユーザー名 (任意)
    #[serde(default)]
    pub preferred_username: Option<String>,
}

/// audience クレームは文字列または配列であり得る。両方を `Vec<String>` にデシリアライズする。
fn deserialize_audience<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct AudVisitor;

    impl<'de> Visitor<'de> for AudVisitor {
        type Value = Vec<String>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a string or an array of strings")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Vec<String>, E> {
            Ok(vec![v.to_string()])
        }

        fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Vec<String>, E> {
            Ok(vec![v])
        }

        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Vec<String>, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                out.push(s);
            }
            Ok(out)
        }
    }

    deserializer.deserialize_any(AudVisitor)
}

// ================================================================
// 認証結果
// ================================================================

/// 認証成功時に得られるユーザー情報
#[derive(Debug, Clone)]
pub struct AuthenticatedSubject {
    /// `sub` クレーム
    pub subject: String,
    /// `email` クレーム (無ければ空)
    pub email: String,
    /// 表示名候補 (`name` → `preferred_username` の順で優先)
    pub display_name: Option<String>,
}

// ================================================================
// メインの検証エントリポイント
// ================================================================

/// ID トークンを検証し、認証済みサブジェクト情報を返す。
///
/// 実行ステップ:
/// 1. JWT をヘッダ/ペイロード/署名に分解
/// 2. Discovery ドキュメントを取得 (KV キャッシュ)
/// 3. JWKS から `kid` に一致する公開鍵を選択 (KV キャッシュ)
/// 4. WebCrypto API で署名を検証
/// 5. クレーム (iss, aud, exp, nbf) を検証
pub async fn verify_id_token(
    env: &Env,
    token: &str,
    config: &OidcConfig,
) -> Result<AuthenticatedSubject> {
    // ── 1. 分解 ──
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(Error::RustError("Invalid JWT format".into()));
    }
    let (header_b64, payload_b64, signature_b64) = (parts[0], parts[1], parts[2]);

    // ── 2. ヘッダのデコード ──
    let header: JwtHeader = decode_json_segment(header_b64)
        .map_err(|e| Error::RustError(format!("Invalid JWT header: {}", e)))?;

    // 対応アルゴリズム
    if !matches!(header.alg.as_str(), "RS256" | "RS384" | "RS512") {
        return Err(Error::RustError(format!(
            "Unsupported JWT alg: {}",
            header.alg
        )));
    }

    // ── 3. JWKS の取得 ──
    let discovery = fetch_discovery(env, config).await?;

    // discovery.issuer と config.issuer の一致検証 (末尾スラッシュ揺らぎを吸収)
    let disc_iss = discovery.issuer.trim_end_matches('/');
    if disc_iss != config.issuer {
        return Err(Error::RustError(format!(
            "Discovery issuer mismatch: expected {}, got {}",
            config.issuer, disc_iss
        )));
    }

    let jwks = fetch_jwks(env, &discovery.jwks_uri, config.jwks_ttl_sec).await?;

    // ── 4. kid に一致する公開鍵を選択 ──
    let jwk = find_matching_jwk(&jwks, header.kid.as_deref())?;

    // ── 5. 署名検証 ──
    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let signature = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|e| Error::RustError(format!("Invalid signature base64: {}", e)))?;

    verify_rs_signature(jwk, &header.alg, signing_input.as_bytes(), &signature).await?;

    // ── 6. クレーム検証 ──
    let claims: IdTokenClaims = decode_json_segment(payload_b64)
        .map_err(|e| Error::RustError(format!("Invalid JWT claims: {}", e)))?;

    validate_claims(&claims, config)?;

    // ── 7. 認証済みサブジェクトを返す ──
    Ok(AuthenticatedSubject {
        subject: claims.sub,
        email: claims.email.unwrap_or_default(),
        display_name: claims.name.or(claims.preferred_username),
    })
}

// ================================================================
// Discovery / JWKS の取得・キャッシュ
// ================================================================

/// Discovery ドキュメントを取得。KV にキャッシュ (TTL = jwks_ttl_sec)。
async fn fetch_discovery(env: &Env, config: &OidcConfig) -> Result<DiscoveryDocument> {
    let cache_key = format!("oidc:discovery:{}", config.issuer);

    // KV キャッシュの試行 (KV が無い場合は単に fetch にフォールバック)
    if let Ok(kv) = env.kv("SETTINGS_KV") {
        if let Ok(Some(cached)) = kv.get(&cache_key).text().await
            && let Ok(doc) = serde_json::from_str::<DiscoveryDocument>(&cached)
        {
            return Ok(doc);
        }
    }

    let url = format!("{}/.well-known/openid-configuration", config.issuer);
    let mut resp = Fetch::Url(Url::parse(&url)?).send().await?;

    if resp.status_code() != 200 {
        return Err(Error::RustError(format!(
            "Discovery fetch failed: HTTP {}",
            resp.status_code()
        )));
    }

    let doc: DiscoveryDocument = resp.json().await?;

    // KV キャッシュに書き込み (TTL 付き)
    if let Ok(kv) = env.kv("SETTINGS_KV")
        && let Ok(body) = serde_json::to_string(&doc)
    {
        let _ = kv
            .put(&cache_key, body)?
            .expiration_ttl(config.jwks_ttl_sec)
            .execute()
            .await;
    }

    Ok(doc)
}

/// JWKS を取得。KV にキャッシュ (TTL = jwks_ttl_sec)。
async fn fetch_jwks(env: &Env, jwks_uri: &str, ttl_sec: u64) -> Result<JwkSet> {
    let cache_key = format!("oidc:jwks:{}", jwks_uri);

    if let Ok(kv) = env.kv("SETTINGS_KV") {
        if let Ok(Some(cached)) = kv.get(&cache_key).text().await
            && let Ok(jwks) = serde_json::from_str::<JwkSet>(&cached)
        {
            return Ok(jwks);
        }
    }

    let mut resp = Fetch::Url(Url::parse(jwks_uri)?).send().await?;

    if resp.status_code() != 200 {
        return Err(Error::RustError(format!(
            "JWKS fetch failed: HTTP {}",
            resp.status_code()
        )));
    }

    let jwks: JwkSet = resp.json().await?;

    if let Ok(kv) = env.kv("SETTINGS_KV")
        && let Ok(body) = serde_json::to_string(&jwks)
    {
        let _ = kv
            .put(&cache_key, body)?
            .expiration_ttl(ttl_sec)
            .execute()
            .await;
    }

    Ok(jwks)
}

/// `kid` (あれば) に一致する RSA 公開鍵を選ぶ。
/// kid が無い場合は RSA キーの先頭を採用 (プロバイダが単一鍵運用のケース)。
fn find_matching_jwk<'a>(jwks: &'a JwkSet, kid: Option<&str>) -> Result<&'a Jwk> {
    let candidates: Vec<&Jwk> = jwks.keys.iter().filter(|k| k.kty == "RSA").collect();

    if candidates.is_empty() {
        return Err(Error::RustError("No RSA keys in JWKS".into()));
    }

    if let Some(kid) = kid {
        candidates
            .iter()
            .find(|k| k.kid.as_deref() == Some(kid))
            .copied()
            .ok_or_else(|| Error::RustError(format!("No JWK matches kid={}", kid)))
    } else {
        Ok(candidates[0])
    }
}

// ================================================================
// クレーム検証
// ================================================================

fn validate_claims(claims: &IdTokenClaims, config: &OidcConfig) -> Result<()> {
    // iss 一致
    let claim_iss = claims.iss.trim_end_matches('/');
    if claim_iss != config.issuer {
        return Err(Error::RustError(format!(
            "Invalid issuer: expected {}, got {}",
            config.issuer, claim_iss
        )));
    }

    // aud 一致 (配列のいずれかに含まれていれば OK)
    if !claims.aud.iter().any(|a| a == &config.audience) {
        return Err(Error::RustError(format!(
            "Audience {} not found in token aud claim",
            config.audience
        )));
    }

    // exp / nbf / iat
    let now = current_unix_time();
    // 時刻同期のずれを許容する clock skew (60 秒)
    const CLOCK_SKEW_SEC: i64 = 60;

    if now > claims.exp + CLOCK_SKEW_SEC {
        return Err(Error::RustError("Token expired".into()));
    }

    if let Some(nbf) = claims.nbf
        && now + CLOCK_SKEW_SEC < nbf
    {
        return Err(Error::RustError("Token not yet valid (nbf)".into()));
    }

    if let Some(iat) = claims.iat
        && iat > now + CLOCK_SKEW_SEC
    {
        return Err(Error::RustError("Token issued in the future".into()));
    }

    Ok(())
}

/// 現在の Unix 時刻 (秒)。
///
/// WASM ターゲットでは `worker::Date::now()` (JavaScript の `Date.now()`) を使用。
/// それ以外 (ホストでのユニットテスト) では `std::time::SystemTime` を使用。
#[cfg(target_arch = "wasm32")]
fn current_unix_time() -> i64 {
    (Date::now().as_millis() / 1000) as i64
}

#[cfg(not(target_arch = "wasm32"))]
fn current_unix_time() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ================================================================
// WebCrypto による RS256/384/512 署名検証
// ================================================================

/// WebCrypto SubtleCrypto を用いて RSA 署名を検証する。
async fn verify_rs_signature(
    jwk: &Jwk,
    alg: &str,
    signing_input: &[u8],
    signature: &[u8],
) -> Result<()> {
    // SubtleCrypto ハンドル取得
    let subtle = get_subtle_crypto()?;

    // 1. JWK を JavaScript オブジェクトとして構築
    let jwk_obj = build_rsa_jwk_js(jwk, alg)?;

    // 2. importKey 用 algorithm オブジェクト
    let hash_name = match alg {
        "RS256" => "SHA-256",
        "RS384" => "SHA-384",
        "RS512" => "SHA-512",
        other => return Err(Error::RustError(format!("Unsupported alg: {}", other))),
    };

    let import_alg = Object::new();
    Reflect::set(
        &import_alg,
        &JsValue::from_str("name"),
        &JsValue::from_str("RSASSA-PKCS1-v1_5"),
    )
    .map_err(js_to_err)?;
    let hash_obj = Object::new();
    Reflect::set(
        &hash_obj,
        &JsValue::from_str("name"),
        &JsValue::from_str(hash_name),
    )
    .map_err(js_to_err)?;
    Reflect::set(&import_alg, &JsValue::from_str("hash"), &hash_obj).map_err(js_to_err)?;

    // key_usages = ["verify"]
    let usages = js_sys::Array::new();
    usages.push(&JsValue::from_str("verify"));

    // 3. importKey(format="jwk", jwk_obj, algorithm, extractable=false, usages)
    let import_promise = subtle
        .import_key_with_object("jwk", &jwk_obj, &import_alg, false, &usages)
        .map_err(js_to_err)?;

    let key_js = JsFuture::from(import_promise).await.map_err(js_to_err)?;
    let crypto_key: CryptoKey = key_js
        .dyn_into()
        .map_err(|_| Error::RustError("importKey did not return a CryptoKey".into()))?;

    // 4. verify(algorithm, key, signature, data)
    let verify_alg = Object::new();
    Reflect::set(
        &verify_alg,
        &JsValue::from_str("name"),
        &JsValue::from_str("RSASSA-PKCS1-v1_5"),
    )
    .map_err(js_to_err)?;

    let sig_array = Uint8Array::from(signature);
    let data_array = Uint8Array::from(signing_input);

    let verify_promise = subtle
        .verify_with_object_and_buffer_source_and_buffer_source(
            &verify_alg,
            &crypto_key,
            &sig_array,
            &data_array,
        )
        .map_err(js_to_err)?;

    let result = JsFuture::from(verify_promise).await.map_err(js_to_err)?;

    if result.as_bool() == Some(true) {
        Ok(())
    } else {
        Err(Error::RustError("JWT signature verification failed".into()))
    }
}

/// `globalThis.crypto.subtle` を取得する
fn get_subtle_crypto() -> Result<SubtleCrypto> {
    let global = js_sys::global();
    let crypto_val = Reflect::get(&global, &JsValue::from_str("crypto")).map_err(js_to_err)?;
    let crypto: web_sys::Crypto = crypto_val
        .dyn_into()
        .map_err(|_| Error::RustError("globalThis.crypto is not a Crypto object".into()))?;
    Ok(crypto.subtle())
}

/// JWK を SubtleCrypto.importKey(format="jwk") に渡せる JS オブジェクトに変換
fn build_rsa_jwk_js(jwk: &Jwk, alg: &str) -> Result<Object> {
    let n = jwk
        .n
        .as_deref()
        .ok_or_else(|| Error::RustError("JWK missing 'n'".into()))?;
    let e = jwk
        .e
        .as_deref()
        .ok_or_else(|| Error::RustError("JWK missing 'e'".into()))?;

    let obj = Object::new();
    set_str(&obj, "kty", "RSA")?;
    set_str(&obj, "alg", alg)?;
    set_str(&obj, "n", n)?;
    set_str(&obj, "e", e)?;
    set_str(&obj, "use", "sig")?;
    if let Some(kid) = &jwk.kid {
        set_str(&obj, "kid", kid)?;
    }
    // ext = true にしないと一部実装が import に失敗することがある
    Reflect::set(&obj, &JsValue::from_str("ext"), &JsValue::from_bool(true)).map_err(js_to_err)?;
    Ok(obj)
}

fn set_str(obj: &Object, key: &str, value: &str) -> Result<()> {
    Reflect::set(obj, &JsValue::from_str(key), &JsValue::from_str(value)).map_err(js_to_err)?;
    Ok(())
}

fn js_to_err(v: JsValue) -> Error {
    Error::RustError(format!("JS error: {:?}", v))
}

// ================================================================
// Base64URL ヘルパ
// ================================================================

/// Base64URL-encoded JSON セグメントを任意の型にデコードする。
fn decode_json_segment<T: for<'de> serde::Deserialize<'de>>(
    segment: &str,
) -> std::result::Result<T, String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|e| format!("base64 decode error: {}", e))?;
    serde_json::from_slice::<T>(&bytes).map_err(|e| format!("json parse error: {}", e))
}

// ================================================================
// 未使用警告抑止のため、wasm-bindgen の JsCast を明示的に使用
// ================================================================
use wasm_bindgen::JsCast;

// ================================================================
// テスト (純粋関数のみ)
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audience_deserialize_string() {
        let v: serde_json::Value = serde_json::json!({
            "sub": "u1",
            "iss": "https://example.com",
            "aud": "my-client",
            "exp": 9999999999i64,
        });
        let claims: IdTokenClaims = serde_json::from_value(v).unwrap();
        assert_eq!(claims.aud, vec!["my-client".to_string()]);
    }

    #[test]
    fn audience_deserialize_array() {
        let v: serde_json::Value = serde_json::json!({
            "sub": "u1",
            "iss": "https://example.com",
            "aud": ["c1", "c2"],
            "exp": 9999999999i64,
        });
        let claims: IdTokenClaims = serde_json::from_value(v).unwrap();
        assert_eq!(claims.aud, vec!["c1".to_string(), "c2".to_string()]);
    }

    #[test]
    fn discovery_parse_minimal() {
        let raw = r#"{
            "issuer":"https://example.com",
            "jwks_uri":"https://example.com/.well-known/jwks.json"
        }"#;
        let d: DiscoveryDocument = serde_json::from_str(raw).unwrap();
        assert_eq!(d.issuer, "https://example.com");
        assert_eq!(d.jwks_uri, "https://example.com/.well-known/jwks.json");
        assert!(d.id_token_signing_alg_values_supported.is_empty());
    }

    fn make_cfg() -> OidcConfig {
        OidcConfig {
            issuer: "https://example.com".into(),
            audience: "my-client".into(),
            jwks_ttl_sec: 3600,
        }
    }

    fn make_claims(exp: i64, iss: &str, aud: Vec<&str>) -> IdTokenClaims {
        IdTokenClaims {
            sub: "u1".into(),
            iss: iss.into(),
            aud: aud.into_iter().map(|s| s.to_string()).collect(),
            exp,
            iat: None,
            nbf: None,
            email: None,
            name: None,
            preferred_username: None,
        }
    }

    #[test]
    fn validate_claims_ok() {
        let cfg = make_cfg();
        let claims = make_claims(i64::MAX / 2, "https://example.com", vec!["my-client"]);
        assert!(validate_claims(&claims, &cfg).is_ok());
    }

    #[test]
    fn validate_claims_iss_mismatch() {
        let cfg = make_cfg();
        let claims = make_claims(i64::MAX / 2, "https://other.example.org", vec!["my-client"]);
        assert!(validate_claims(&claims, &cfg).is_err());
    }

    #[test]
    fn validate_claims_aud_mismatch() {
        let cfg = make_cfg();
        let claims = make_claims(i64::MAX / 2, "https://example.com", vec!["other-client"]);
        assert!(validate_claims(&claims, &cfg).is_err());
    }

    #[test]
    fn validate_claims_expired() {
        let cfg = make_cfg();
        let claims = make_claims(0, "https://example.com", vec!["my-client"]);
        assert!(validate_claims(&claims, &cfg).is_err());
    }

    #[test]
    fn validate_claims_trailing_slash_tolerance() {
        let cfg = make_cfg();
        // トークン側の iss に末尾スラッシュがあっても一致と判定
        let claims = make_claims(i64::MAX / 2, "https://example.com/", vec!["my-client"]);
        assert!(validate_claims(&claims, &cfg).is_ok());
    }
}
