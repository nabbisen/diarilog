# OIDC Providers Configuration

Trauma Journal は OIDC (OpenID Connect) 標準に準拠した任意の認証プロバイダで動作する。本ドキュメントは主要なプロバイダごとの具体的な設定手順を扱う。

## 共通の必要情報

どのプロバイダを使う場合も、最終的に gateway-worker の以下の設定を埋める:

```toml
# workers/gateway/wrangler.toml
[vars]
OIDC_ISSUER = ""           # https://your-tenant.example.com (末尾スラッシュなし)
OIDC_AUDIENCE = ""         # クライアントが要求する audience (= クライアント ID または API identifier)
OIDC_JWKS_TTL_SEC = "3600" # JWKS のキャッシュ寿命 (秒)
```

ブラウザは Authorization Code Flow + PKCE で ID トークンを取得し、`Authorization: Bearer <id_token>` ヘッダで `/api/*` を呼ぶ。

## クライアント設定の共通項目

| 項目 | 値 |
|---|---|
| Application Type | Single Page Application (SPA) または Native (PKCE 必須) |
| Grant Type | Authorization Code with PKCE |
| Token Signing Algorithm | RS256 / RS384 / RS512 のいずれか (HS256 は不可) |
| Redirect URIs | `https://gateway.YOUR-DOMAIN/auth/callback` |
| Logout URIs | `https://gateway.YOUR-DOMAIN/` |
| Allowed Web Origins (CORS) | `https://gateway.YOUR-DOMAIN` |
| Required Claims | `sub`, `email`, `email_verified`, `iss`, `aud`, `exp`, `iat` |

## Auth0

### アプリケーション作成

Auth0 ダッシュボード → Applications → Create Application:

- Name: `Trauma Journal (staging)`
- Application Type: **Single Page Application**

作成後、以下を控える:
- Domain: `your-tenant.auth0.com`
- Client ID: `xxxxxxxxxxxxxxxxxxxxxxxxxxxx`

### API 作成 (Audience)

Auth0 ダッシュボード → APIs → Create API:

- Name: `Trauma Journal API (staging)`
- Identifier: `https://api.trauma-journal.staging` (これが `OIDC_AUDIENCE` になる)
- Signing Algorithm: `RS256`

### Application 設定

作成した Application の Settings タブで:

- Allowed Callback URLs: `https://gateway.YOUR-DOMAIN.workers.dev/auth/callback`
- Allowed Logout URLs: `https://gateway.YOUR-DOMAIN.workers.dev/`
- Allowed Web Origins: `https://gateway.YOUR-DOMAIN.workers.dev`
- Token Endpoint Authentication Method: `None` (SPA + PKCE)
- Refresh Token Rotation: `Rotating`
- ID Token Expiration: `36000` (10 時間)

### gateway 側の設定値

```toml
[env.staging.vars]
OIDC_ISSUER = "https://your-tenant.auth0.com"
OIDC_AUDIENCE = "https://api.trauma-journal.staging"
OIDC_JWKS_TTL_SEC = "3600"
```

JWKS は `https://your-tenant.auth0.com/.well-known/openid-configuration` から自動取得される。

## Keycloak (self-hosted)

### Realm 作成

Keycloak Admin Console → Master → Add realm:

- Name: `trauma-journal-staging`

### Client 作成

Clients → Create:

- Client ID: `trauma-journal-frontend`
- Client Protocol: `openid-connect`
- Access Type: `public` (PKCE 利用)
- Standard Flow Enabled: ON
- Direct Access Grants: OFF (本サービスでは使わない)

Settings タブで:

- Valid Redirect URIs: `https://gateway.YOUR-DOMAIN.workers.dev/auth/callback`
- Web Origins: `https://gateway.YOUR-DOMAIN.workers.dev`
- Proof Key for Code Exchange: `S256`

### Token Mappers (email を ID トークンに含める)

Mappers タブで `email` と `email_verified` のマッパーを追加 (デフォルトで OK な場合が多い)。

### gateway 側の設定値

```toml
[env.staging.vars]
OIDC_ISSUER = "https://keycloak.your-domain.com/realms/trauma-journal-staging"
OIDC_AUDIENCE = "trauma-journal-frontend"
OIDC_JWKS_TTL_SEC = "3600"
```

JWKS は `{issuer}/protocol/openid-connect/certs` から自動取得される。

### 自前ホストの注意

Keycloak のホスト先 (例: `keycloak.your-domain.com`) は **公開されている必要がある**。Cloudflare Workers の gateway-worker から JWKS エンドポイントへの HTTPS 通信が成立しないと OIDC 検証ができない。

## Google Sign-In

### プロジェクト + OAuth クライアント作成

Google Cloud Console → APIs & Services → Credentials → Create Credentials → OAuth client ID:

- Application type: `Web application`
- Name: `Trauma Journal staging`
- Authorized JavaScript origins: `https://gateway.YOUR-DOMAIN.workers.dev`
- Authorized redirect URIs: `https://gateway.YOUR-DOMAIN.workers.dev/auth/callback`

控える値: Client ID (例: `123456789012-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com`)

### gateway 側の設定値

```toml
[env.staging.vars]
OIDC_ISSUER = "https://accounts.google.com"
OIDC_AUDIENCE = "123456789012-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com"
OIDC_JWKS_TTL_SEC = "3600"
```

JWKS は `https://www.googleapis.com/oauth2/v3/certs` から自動取得される。

### 注意

- Google Sign-In の `aud` クレームは Client ID なので、`OIDC_AUDIENCE` には Client ID をそのまま入れる
- `email_verified: true` のクレームを必ず確認すること。本サービスは `email_verified` が `true` のユーザーのみ受け入れる前提

## Azure AD / Microsoft Entra ID

### App 登録

Azure Portal → Microsoft Entra ID → App registrations → New registration:

- Name: `Trauma Journal staging`
- Supported account types: `Single tenant` または `Multi-tenant` (要件次第)
- Redirect URI: SPA / `https://gateway.YOUR-DOMAIN.workers.dev/auth/callback`

控える値:
- Application (client) ID
- Directory (tenant) ID

### Authentication 設定

- Implicit grant: 不要 (PKCE のみ)
- ID tokens: チェックを入れる

### Expose an API (オプション)

API として scopes を切る場合:

- Application ID URI: `api://{client-id}` (デフォルトのまま)
- Add scope: `access_as_user`

### gateway 側の設定値

```toml
[env.staging.vars]
OIDC_ISSUER = "https://login.microsoftonline.com/{TENANT_ID}/v2.0"
OIDC_AUDIENCE = "{CLIENT_ID}"
OIDC_JWKS_TTL_SEC = "3600"
```

JWKS は `{issuer}/discovery/v2.0/keys` から自動取得される。

## トラブルシューティング

### 401 Unauthorized: Invalid token signature

- `OIDC_JWKS_TTL_SEC` を短くする (300 秒など)、もしくは KV の jwks エントリを削除して再取得を強制
- プロバイダ側でキーローテーションがあった可能性

### 401 Unauthorized: Invalid audience

- `OIDC_AUDIENCE` の値がトークンの `aud` クレームと完全に一致しているか確認
- Auth0 では Application Client ID と API Identifier を混同しがち。本サービスは API Identifier を使う

### 401 Unauthorized: Token expired

- ブラウザ側のトークン更新フローが回っていない可能性。SPA ライブラリ (auth0-spa-js, @azure/msal-browser など) のリフレッシュ設定を見直す

### CORS エラー

gateway-worker は OPTIONS preflight を扱うので、`Access-Control-Allow-Origin: *` を返す。それでもエラーが出る場合:

- ブラウザのリクエストヘッダ (`Authorization`) が `Access-Control-Allow-Headers` に含まれているか
- gateway-worker の最新がデプロイされているか

## セキュリティ留意点

- **HS256 は不可**: 共有秘密鍵方式は KV キャッシュとの相性が悪く、本サービスは RS256/384/512 のみサポート
- **`email_verified: true` を必須に**: メール検証を経ないアカウント作成を防ぐ
- **トークン保存場所**: ブラウザでは `sessionStorage` 推奨。`localStorage` は XSS で攻撃される
- **Redirect URI のホワイトリスト**: ワイルドカードは使わず、必ず完全一致
