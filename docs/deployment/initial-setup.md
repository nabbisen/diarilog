# Initial Setup

新規 Cloudflare アカウント・プロジェクトに対して、Trauma Journal Platform をデプロイ可能な状態にするまでの初回セットアップ手順。

`prerequisites.md` の前提が満たされていることを前提とする。

## ステップ 0: 環境変数の確認

このガイドでは以下の環境を staging として作業する例で書く。本番環境 (`production`) や開発環境 (`dev`) でも同様の手順を、`--env` 引数を変えて適用する。

```bash
export ENV=staging        # または dev / production
```

## ステップ 1: Wrangler でログイン

```bash
wrangler login
```

ブラウザで Cloudflare アカウントへの認証フローが開く。完了するとローカルに認証トークンが保存される。

## ステップ 2: D1 データベースを作成

journal、identity、safety、dialog の 4 worker は同一の D1 データベースを共有する (テーブルが分かれている)。dev/staging/production の 3 環境で個別の DB を持つ。

```bash
# staging 環境向けに作成
wrangler d1 create trauma-journal-db-staging
```

実行すると以下のような出力が得られる:

```
✅ Successfully created DB 'trauma-journal-db-staging' in region WNAM
Created your new D1 database.

[[d1_databases]]
binding = "DB"
database_name = "trauma-journal-db-staging"
database_id = "abc12345-6789-0abc-defg-123456789012"
```

`database_id` を控えて、各 worker の `wrangler.toml` の該当環境セクションに記載する:

- `workers/journal/wrangler.toml` の `[[env.staging.d1_databases]]`
- `workers/identity/wrangler.toml` の `[[env.staging.d1_databases]]`
- `workers/safety/wrangler.toml` の `[[env.staging.d1_databases]]`
- `workers/dialog/wrangler.toml` の `[[env.staging.d1_databases]]`

### マイグレーション適用

リポジトリ直下の `migrations/0001_initial.sql` をどれか 1 つの worker から流す (どれでも同じ DB を見るため):

```bash
cd workers/journal
wrangler d1 migrations apply trauma-journal-db-staging --env staging
```

## ステップ 3: R2 バケットを作成

journal-worker が暗号化済み日記本文を保存する R2 バケット。

```bash
wrangler r2 bucket create trauma-journal-diaries-staging
```

`workers/journal/wrangler.toml` の `[[env.staging.r2_buckets]]` に `bucket_name = "trauma-journal-diaries-staging"` を設定する。

## ステップ 4: KV Namespace を作成

gateway は OIDC JWKS のキャッシュと user_settings に KV を使う。bff は (将来的に) UI 状態の永続化に KV を使う想定。

```bash
# gateway 用
wrangler kv namespace create SETTINGS_KV --env staging

# 出力例:
# 🌀 Creating namespace with title "SETTINGS_KV-staging"
# ✨ Success!
# Add the following to your configuration file:
# [[kv_namespaces]]
# binding = "SETTINGS_KV"
# id = "abcdef0123456789abcdef0123456789"
```

`id` を `workers/gateway/wrangler.toml` の `[[env.staging.kv_namespaces]]` に記載。

dialog/safety にも個別の KV が必要なら同様に作成 (現状 Phase 1 では gateway のみが KV を直接使う)。

## ステップ 5: Workers AI バインディング

Workers AI はアカウント単位で自動的に有効化されている。`wrangler.toml` の `[ai] binding = "AI"` を持つ worker (dialog, safety) は、デプロイ時に自動でバインディングが解決される。事前作成は不要。

## ステップ 6: Static Assets ディレクトリの準備 (bff-worker)

bff-worker はハイドレーション WASM/JS を Workers Static Assets で配信するため、初回デプロイ前に物理的なファイルを配置しておく必要がある。

```bash
# 1. wasm-pack で CSR バンドルを生成
./scripts/build-bff-hydrate.sh

# 2. 生成物の確認
ls -la workers/bff/dist/static/_assets/
# 期待: web-app.js, web-app_bg.wasm, web-app.d.ts, package.json
```

詳細は `docs/hydration-verification.md` 参照。

## ステップ 7: シークレットの登録

シークレットは `wrangler secret put` で個別に登録する。`wrangler.toml` には記載しない (リポジトリにコミットされないように)。

### gateway-worker のシークレット

```bash
cd workers/gateway

# OIDC プロバイダ設定 (vars に書く方が運用しやすければ vars でも可)
wrangler secret put OIDC_ISSUER --env staging
# プロンプトで貼り付け: https://your-tenant.auth0.com (末尾スラッシュなし)

wrangler secret put OIDC_AUDIENCE --env staging
# プロンプトで貼り付け: trauma-journal-staging

# Turnstile (登録時のスパム防止)
wrangler secret put TURNSTILE_SECRET_KEY --env staging
# プロンプトで貼り付け: 0x4xxxxxxxxxxxxxxxxxxx

cd ../..
```

### dialog-worker / safety-worker のシークレット (内部 worker は通常不要)

内部 worker は gateway から `X-User-*` ヘッダを信頼するため、独自のシークレットは現状不要。Workers AI も Cloudflare アカウント側で自動認証される。

OIDC プロバイダ別の具体的な設定値については `oidc-providers.md` 参照。

## ステップ 8: デプロイ順序通りに反映

`deploy-order.md` 参照。

```bash
(cd workers/identity && wrangler deploy --env staging)
(cd workers/safety   && wrangler deploy --env staging)
(cd workers/journal  && wrangler deploy --env staging)
(cd workers/dialog   && wrangler deploy --env staging)
(cd workers/bff      && wrangler deploy --env staging)
(cd workers/gateway  && wrangler deploy --env staging)
```

## ステップ 9: 動作確認

```bash
# Health check (各 worker は /_health または /api/health を持つ)
curl -i https://gateway.YOUR-DOMAIN.workers.dev/api/health
# 期待: HTTP/2 200 + body "ok"

# SSR 経路
curl -i https://gateway.YOUR-DOMAIN.workers.dev/
# 期待: HTTP/2 200 + Content-Type: text/html
```

詳細な動作確認手順は `docs/hydration-verification.md` 参照。

## トラブルシューティング

### `Error: Service binding 'JOURNAL' was not found`

deploy-order の通りに、被呼び出し側 (journal) が呼び出し側 (gateway) より先にデプロイされている必要がある。`deploy-order.md` 参照。

### `D1_ERROR: no such table: users`

マイグレーション適用が漏れている。ステップ 2 の `wrangler d1 migrations apply` を再実行。

### `404 Not Found` で `/_assets/web-app.js`

bff-worker のデプロイ時に `dist/static/_assets/` が空だった可能性。ステップ 6 の wasm-pack ビルドを実行してから bff を再デプロイする。
