# ハイドレーション動作の実機検証手順

bff-worker の SSR + ハイドレーション統合 (v2.0 + v2.1) を Cloudflare 環境にデプロイし、ブラウザで実際に動作確認するための手順とチェックリストです。

## 前提

- Cloudflare アカウント (Workers Paid プラン推奨、Free でも可)
- `wrangler` CLI 4.x がローカルにインストール済み
- `wasm-pack` 0.13+ がインストール済み (`cargo install wasm-pack`)
- `rustup target add wasm32-unknown-unknown` 実行済み

## 検証フロー全体

```
1. 依存ワーカーを順番にデプロイ
   identity → safety → journal → dialog → bff → gateway

2. bff-hydrate CSR バンドルを生成
   scripts/build-bff-hydrate.sh

3. bff-worker を再デプロイ (assets を反映)
   cd workers/bff && wrangler deploy

4. ブラウザで gateway URL にアクセス
   - / が SSR で返ること
   - ハイドレーション後に Leptos がインタラクティブになること
   - DevTools で WASM がエッジから配信されていること
```

## Step 1: 依存ワーカーのデプロイ

詳細は `docs/deployment/deploy-order.md` 参照。

```bash
# core 4 つを順にデプロイ
(cd workers/identity && wrangler deploy --env staging)
(cd workers/safety   && wrangler deploy --env staging)
(cd workers/journal  && wrangler deploy --env staging)
(cd workers/dialog   && wrangler deploy --env staging)
```

## Step 2: bff-hydrate CSR バンドルを生成

```bash
# rustup target が入っているか確認
rustup target list --installed | grep wasm32-unknown-unknown

# wasm-pack でビルド
./scripts/build-bff-hydrate.sh
```

成功すると `workers/bff/dist/static/_assets/` に以下が生成されます:

- `web-app.js` — wasm-bindgen が生成する JS グルー
- `web-app_bg.wasm` — Leptos コンポーネントとランタイムを含む WASM バイナリ
- `web-app.d.ts` — TypeScript 型定義 (任意)
- `package.json` — wasm-pack のメタデータ (任意)

スクリプト末尾のサニティチェックで、WASM サイズが妥当か (50 KB - 5 MB 程度)、`__wbindgen_start` が含まれているかを確認できます。

## Step 3: bff-worker をデプロイ

```bash
cd workers/bff
wrangler deploy --env staging
```

`wrangler.toml` の `[assets] directory = "./dist/static"` 設定により、Step 2 で生成した CSR バンドルがアセットレイヤーにアップロードされます。

## Step 4: gateway-worker をデプロイ

```bash
cd workers/gateway
wrangler deploy --env staging
```

bff への Service Bindings (`binding = "BFF"`) が解決されることを確認してください。

## Step 5: ブラウザでの動作確認

### 確認項目 1: SSR が動作している

```bash
curl -i https://gateway.YOUR-DOMAIN.workers.dev/
```

期待:
- HTTP 200
- `Content-Type: text/html; charset=utf-8`
- レスポンスボディに `<h1>Trauma Journal</h1>` 等のレンダリング済み HTML が含まれる
- レスポンスボディに `window.__TJP_ROUTE__` を設定する `<script>` が含まれる
- `<script type="module" src="/_assets/web-app.js">` が含まれる

### 確認項目 2: 静的アセットがエッジから配信されている

```bash
curl -i https://gateway.YOUR-DOMAIN.workers.dev/_assets/web-app.js
curl -i https://gateway.YOUR-DOMAIN.workers.dev/_assets/web-app_bg.wasm
```

期待:
- HTTP 200
- `Content-Type: application/javascript` (web-app.js)
- `Content-Type: application/wasm` (web-app_bg.wasm)
- レスポンスヘッダに `cf-cache-status: HIT` (2 度目以降のリクエストで)

### 確認項目 3: ブラウザでハイドレーションが動作する

ChromeまたはFirefox の DevTools を開き:

1. **Network タブ**:
   - `/` のドキュメントレスポンスが SSR HTML
   - `web-app.js` と `web-app_bg.wasm` が後続でロードされる
   - WASM のレスポンスヘッダに `cf-cache-status: HIT` が付く

2. **Console タブ**:
   - エラーが出ていないこと (特に "panicked" や hydration mismatch)
   - `console_error_panic_hook::set_once` でフックされたパニックが見えること (発生していれば)

3. **Elements タブ**:
   - `<body>` 直下に `class="app-root"` の div があり、Leptos の DOM が生えている
   - hydration 後も DOM 構造が変わらないこと (= mismatch していないこと)

### 確認項目 4: 集約 API が動作する

OIDC 認証済みのトークンを取得した上で:

```bash
curl -i \
  -H "Authorization: Bearer $ID_TOKEN" \
  https://gateway.YOUR-DOMAIN.workers.dev/api/dashboard
```

期待:
- HTTP 200
- レスポンスボディが `DashboardResponse` 形式 (`user`, `recent_diaries`, `active_session`, `status`)
- `status.user_ok` 等のフラグが期待通り

部分的劣化テスト:
- 一時的に identity-worker を停止 → `user: null, status.user_ok: false` で 200 が返ること
- すべての core を停止 → 全フィールド `null`/`[]`、`status` 全て `false` で 200

## Step 6: トラブルシューティング

### 問題 A: WASM が 404

- `[assets] directory` のパスが正しいか確認
- `dist/static/_assets/web-app_bg.wasm` が物理的に存在するか確認
- `wrangler deploy` が assets を含めて upload しているか log を確認

### 問題 B: ハイドレーション mismatch

ブラウザコンソールに `Hydration mismatch` のエラーが出る場合:

- SSR 側と Hydrate 側で `Route` の値が一致していない可能性
- `view!` マクロ内に SSR / Hydrate で結果が違うコード (例: `js-sys` 直接呼出) がある可能性
- `console_error_panic_hook` をフロントエンド側で有効化して詳細を取得

### 問題 C: `window.__TJP_ROUTE__ is undefined`

- bff の HTML テンプレート (`layout.rs::wrap_document`) が hydration scripts を埋め込んでいるか
- `WEB_ASSETS_BASE_URL` 環境変数が設定されているか (空だと scripts は注入されない)

### 問題 D: Service Bindings 呼び出し失敗

- gateway → bff の bindings 名が `BFF` で揃っているか
- bff が staging にデプロイ済みで、gateway も staging を参照しているか
- `wrangler tail --env staging --format pretty` でログ確認

## 検証ログテンプレート

実機検証のたびに、以下のフォーマットで結果を記録することを推奨:

```
日時: 2026-XX-XX
環境: staging
gateway URL: https://...
bff URL: https://...

[ ] Step 1: 依存ワーカー全てデプロイ成功
[ ] Step 2: bff-hydrate ビルド成功 (WASM サイズ: ___ KB)
[ ] Step 3: bff-worker デプロイ成功
[ ] Step 4: gateway-worker デプロイ成功
[ ] Step 5-1: SSR HTML が返る
[ ] Step 5-2: WASM がエッジから配信
[ ] Step 5-3: ブラウザで hydration 成功 (DevTools 確認)
[ ] Step 5-4: 集約 API が DashboardResponse を返す

問題:
- (発生した問題と解決策)
```
