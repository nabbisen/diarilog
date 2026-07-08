# Multi-Environment

dev / staging / production の 3 環境を、`wrangler --env` で切り替えながら運用する手順。

## 環境の位置づけ

| 環境 | 用途 | デプロイ頻度 | データの扱い |
|---|---|---|---|
| **dev** | 開発者個人 / フィーチャーブランチ | 任意 (push 毎、手元から) | 使い捨て、リセット OK |
| **staging** | 統合テスト / E2E 検証 | PR マージ毎 | 本番に近い偽データ |
| **production** | 実利用 | 慎重 (リリース時のみ) | 永続、慎重に扱う |

## 命名規則

各 worker / リソースは `-{env}` サフィックスで分離する (production はサフィックスなし、もしくは明示的に `-production`)。本リポジトリでは:

| Worker | dev 名 | staging 名 | production 名 |
|---|---|---|---|
| gateway | trauma-journal-gateway-dev | trauma-journal-gateway-staging | trauma-journal-gateway |
| bff | trauma-journal-bff-dev | trauma-journal-bff-staging | trauma-journal-bff |
| journal | trauma-journal-journal-dev | trauma-journal-journal-staging | trauma-journal-journal |
| identity | trauma-journal-identity-dev | trauma-journal-identity-staging | trauma-journal-identity |
| safety | trauma-journal-safety-dev | trauma-journal-safety-staging | trauma-journal-safety |
| dialog | trauma-journal-dialog-dev | trauma-journal-dialog-staging | trauma-journal-dialog |

リソース (D1 / R2 / KV) も同様のサフィックス命名を採用する。

## wrangler.toml の構造

各 worker の `wrangler.toml` は、トップレベルが `production` のデフォルトとして書かれており、`[env.dev]` / `[env.staging]` で個別に上書きする。例 (`workers/journal/wrangler.toml`):

```toml
# トップレベル = production の設定
name = "trauma-journal-journal"
main = "build/worker/shim.mjs"
compatibility_date = "2025-01-01"

[[d1_databases]]
binding = "DB"
database_name = "trauma-journal-db"
database_id = ""           # production の D1 ID をここに

# dev 環境
[env.dev]
name = "trauma-journal-journal-dev"

[[env.dev.d1_databases]]
binding = "DB"
database_name = "trauma-journal-db-dev"
database_id = ""           # dev の D1 ID

# staging 環境
[env.staging]
name = "trauma-journal-journal-staging"

[[env.staging.d1_databases]]
binding = "DB"
database_name = "trauma-journal-db-staging"
database_id = ""           # staging の D1 ID
```

`wrangler deploy --env staging` を指定すると `[env.staging]` セクションが有効になる。`--env` 無指定の場合はトップレベル (= production) が使われる。

## 各環境のデプロイコマンド

### dev (個人開発)

```bash
# 単一 worker を素早く更新
(cd workers/bff && wrangler deploy --env dev)

# 全 worker を順次デプロイ
for w in identity safety journal dialog bff gateway; do
  (cd workers/$w && wrangler deploy --env dev)
done
```

### staging (PR マージ後)

CI で実行する想定。手動なら:

```bash
for w in identity safety journal dialog bff gateway; do
  (cd workers/$w && wrangler deploy --env staging)
done
```

### production

production は `--env` なしで:

```bash
for w in identity safety journal dialog bff gateway; do
  (cd workers/$w && wrangler deploy)
done
```

または明示的に:

```bash
for w in identity safety journal dialog bff gateway; do
  (cd workers/$w && wrangler deploy --env production)
done
```

(両方の表記をサポートする `wrangler.toml` を持つことが望ましい)

## 環境ごとの設定差異

| 設定項目 | dev | staging | production |
|---|---|---|---|
| `OIDC_ISSUER` | `https://staging-auth0.../` | `https://staging-auth0.../` | `https://prod-auth0.../` |
| `OIDC_AUDIENCE` | `trauma-journal-dev` | `trauma-journal-staging` | `trauma-journal` |
| `WEB_ASSETS_BASE_URL` | `/_assets` (相対) | `/_assets` | `/_assets` または専用 CDN |
| `MAX_SUGGEST_PER_DAY` | 100 (緩く) | 10 | 10 |
| Workers AI `MAX_TOKENS` | 256 (節約) | 512 | 512 |
| log level | debug | info | warn |
| Tail Workers | OFF | ON | ON (アラート連動) |

## カスタムドメイン

production だけは `trauma-journal.YOUR-DOMAIN.com` のようなカスタムドメインに紐付けたい:

```toml
# workers/gateway/wrangler.toml の production セクション (= トップレベル)
routes = [
  { pattern = "trauma-journal.your-domain.com/*", zone_name = "your-domain.com" }
]
```

dev/staging はデフォルトの `*.workers.dev` に任せる (workers_dev = true は gateway のみ)。bff など内部 worker は全環境で `workers_dev = false`。

## ローカル開発 (`wrangler dev`)

dev 環境を Cloudflare 上に作らずに、ローカルで bff + core を動かしたい:

```bash
# Terminal 1
(cd workers/journal && wrangler dev --env dev --port 8101)

# Terminal 2
(cd workers/identity && wrangler dev --env dev --port 8102)

# ... 他の worker も同様 ...

# Terminal X
(cd workers/gateway && wrangler dev --env dev --port 8000)
```

`wrangler dev` は Service Bindings を **同じプロセス内のローカル worker** として解決できるが、複数プロセスにまたがるとデフォルトでは解決できない。`miniflare` の上位機能として `wrangler dev --remote` を使うか、Cloudflare の dev 環境にデプロイしたものを使う方が速い。

実用上は **Cloudflare 上の dev 環境** で開発する方が、Service Bindings の動作を正確に再現できる。

## 環境の同期確認

デプロイ後、3 環境が同じバージョンで動いているか確認:

```bash
for w in gateway bff journal identity safety dialog; do
  echo "=== $w ==="
  for env in dev staging production; do
    case "$env" in
      production) NAME="trauma-journal-$w" ;;
      *) NAME="trauma-journal-$w-$env" ;;
    esac
    wrangler deployments list --name $NAME 2>/dev/null | head -2
  done
done
```

各環境の最新デプロイのコミット SHA を取って、想定通りのバージョンが反映されているか確認する。

## 環境ごとのデータ分離

D1 / R2 / KV は完全に独立した別リソースを使うため、環境間のデータ汚染は起こらない。**ただし Workers AI の利用枠はアカウント全体で共有** されるため、dev で大量にリクエストすると production にも影響することに注意。

## トラブル: 「環境を間違えてデプロイした」

```bash
# 直前のデプロイ確認
wrangler deployments list --env staging --name trauma-journal-bff-staging

# 1 つ前に巻き戻し
wrangler rollback --env staging
```

詳細は `rollback.md` 参照。
