# Rollback

問題が発生した際に、前のバージョンに戻す/影響を最小化する手順。

## 用語

- **構成失敗**: デプロイ自体は成功したが、設定ミス (環境変数、Service Bindings) で動かない状態
- **コード失敗**: コードのバグや破壊的変更で動かない状態
- **データ汚染**: マイグレーションやコードバグで D1/R2/KV のデータが不整合になった状態

## 即応の判断基準

- **5xx 率が 1% 超**: 即座にロールバック
- **特定のエンドポイントだけ失敗**: 該当 worker のみロールバック
- **データ汚染の可能性**: ロールバックしても改善しないので、まず原因特定。間違ってもロールバックでデータをさらに上書きしない

## ロールバック手段の比較

| 手段 | 速度 | 影響範囲 | 用途 |
|---|---|---|---|
| `wrangler rollback` | 数秒 | 単一 worker | コード失敗、構成失敗 |
| 旧バージョンを `wrangler deploy` | 1〜2 分 | 単一 worker | 旧コミットからの再デプロイ |
| Cloudflare ダッシュボードで巻き戻し | 数秒 | 単一 worker | wrangler が動かない時の代替 |
| D1 マイグレーション巻き戻し | 数分 | DB | スキーマの破壊的変更 |
| R2 オブジェクトの復元 | 個別 | バケット | バージョニングが有効な場合のみ |

## ケース 1: gateway-worker の新デプロイで HTML が真っ白

→ `wrangler rollback` で即座に直前バージョンに戻す。

```bash
cd workers/gateway
wrangler rollback --env staging
```

確認プロンプトに `y` で応答すると、直前のデプロイバージョンに戻る。
所要時間 1〜10 秒。

## ケース 2: bff-worker のハイドレーションが壊れた

ハイドレーション WASM のサイズが想定外、SSR と DOM mismatch が出る、など。

```bash
cd workers/bff
wrangler rollback --env staging
```

ロールバックすると **その時点でアップロードされていた static assets** にも戻る (assets binding は worker version と紐付く)。よって CSR バンドルも一緒に巻き戻る。

## ケース 3: 集約 API が一部 core への問い合わせで 500 を返す

bff の `/api/dashboard` は **部分的劣化** 設計なので、ある core が落ちても 200 を返し、該当フィールドは `null` になる。なので「dashboard が 500」になっているのは、bff 側のバグまたは Service Bindings 解決失敗の可能性が高い。

確認:

```bash
# bff のログを見る
wrangler tail --env staging --name trauma-journal-bff-staging --format pretty

# 個別 core が動いていることを確認
wrangler tail --env staging --name trauma-journal-identity-staging --format pretty
```

それぞれが普通に応答していれば、bff のロジックバグ。`wrangler rollback` で bff のみ巻き戻す。

## ケース 4: D1 マイグレーションを間違えた

破壊的変更 (`DROP COLUMN` など) の後で問題が判明した場合:

```bash
# どのマイグレーションが適用されているか確認
wrangler d1 migrations list trauma-journal-db-staging --env staging

# 巻き戻し用マイグレーション (例: 0002_revert.sql) を作成して適用
cat > migrations/0002_revert.sql <<EOF
-- 0001 で DROP した列を復活させる
ALTER TABLE users ADD COLUMN legacy_field TEXT;
EOF

wrangler d1 migrations apply trauma-journal-db-staging --env staging
```

D1 には自動的な「マイグレーションロールバック」機能はない。**前進方向にだけ修正できる**。よって本番マイグレーションは事前に staging で十分検証することが必須。

## ケース 5: 機能フラグでロールバック

Phase 1 では `USE_SB_*` 機能フラグで段階的に切り替えていた。Phase 2 ではフラグは除去済みだが、将来的に追加する場合の手順:

1. `wrangler.toml` の該当フラグを `"false"` に変更
2. gateway-worker のみ再デプロイ:

```bash
cd workers/gateway
wrangler deploy --env staging
```

3. 数分待って効果を確認 (Cloudflare のグローバル伝搬は 30 秒程度)

## ケース 6: 旧バージョンへの完全復旧

ある git タグ/コミットからの完全再デプロイ:

```bash
# 最後の安定版に戻す
git checkout v2.0
git switch --detach        # detached HEAD で確認モード

# デプロイ順序通りに巻き戻し
(cd workers/identity && wrangler deploy --env staging)
(cd workers/safety   && wrangler deploy --env staging)
(cd workers/journal  && wrangler deploy --env staging)
(cd workers/dialog   && wrangler deploy --env staging)
(cd workers/bff      && wrangler deploy --env staging)
(cd workers/gateway  && wrangler deploy --env staging)

# 元のブランチに戻る
git checkout main
```

復旧後の動作確認は `docs/hydration-verification.md` の「ステップ 5」のチェックリストに従う。

## 予防策

### デプロイ前

- staging で十分に動作確認
- マイグレーションは前進・後進両方を用意
- WASM サイズの急増は性能劣化のサイン (build スクリプトで検出)

### デプロイ後

- 5 分間 Cloudflare ダッシュボードでエラー率を監視
- `wrangler tail` で実トラフィックを目視確認
- staging では 24 時間放置してから production に同じバージョンを上げる

### モニタリング設定 (`observability.md` 参照)

- Tail Workers でリアルタイムエラーを Slack 等に流す
- Cloudflare Analytics の Errors を週次でレビュー
- `X-Trace-Id` で gateway → bff → core 全体のリクエスト相関を確認

## ロールバックできない損害が発生した場合

ユーザーデータの破壊が起きてしまった場合:

1. 即座に該当 worker を停止 (`wrangler delete`、ただし最終手段)
2. 影響範囲のユーザーを D1 から特定
3. R2 のオブジェクトバージョニングが有効なら復元を試みる
4. ユーザーへの開示と謝罪準備

これが起きないために、本番マイグレーションは必ず staging で先行検証することを徹底する。
