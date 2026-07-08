# Prerequisites

Trauma Journal Platform をデプロイするための前提条件をまとめます。

## アカウント

| サービス | 用途 |
|---|---|
| Cloudflare アカウント | Workers / R2 / D1 / KV / Workers AI / Turnstile |
| OIDC プロバイダ | 認証 (Auth0 / Keycloak / Google / Azure AD など) |
| GitHub アカウント | コードホスティングと CI/CD |

## ローカルツール

| ツール | バージョン | インストール方法 |
|---|---|---|
| Rust | 1.91+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `wasm32-unknown-unknown` ターゲット | — | `rustup target add wasm32-unknown-unknown` |
| Node.js | 20 LTS+ | nvm 推奨 |
| `wrangler` | 4.x | `npm install -g wrangler` |
| `worker-build` | (cargo install) | `cargo install -q worker-build` (各 Worker の build コマンドが自動で行う) |

## Cloudflare 側で事前に必要なリソース

各 Worker の `wrangler.toml` に書かれている各バインディングを、デプロイ前に Cloudflare 上に作成しておく必要があります。詳細は [`initial-setup.md`](./initial-setup.md) に記載予定。

- D1 データベース (production / staging / dev で 3 つ)
- R2 バケット (同上、3 つ)
- KV Namespace (同上、3 つ)
- Workers AI バインディング (アカウント単位で自動利用可)

## OIDC プロバイダの準備

- Trauma Journal を表す Application / Client を作成
- Redirect URI: `https://gateway.example.com/auth/callback` (将来 Phase 1.6 で実装予定)
- Audience / Client ID を `OIDC_AUDIENCE` に設定
- Issuer URL を `OIDC_ISSUER` に設定

プロバイダ別の設定値は [`oidc-providers.md`](./oidc-providers.md) 予定。

## ローカルチェック

```bash
cargo check --workspace
cargo test --workspace --tests
```

両方が成功することをデプロイ前に確認してください。
