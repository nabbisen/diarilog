# Observability

Cloudflare Workers 環境での Trauma Journal Platform のログ運用、トレース相関、メトリクスの取り扱い。

## アーキテクチャ上のトレース構造

リクエストは `gateway → bff → core` の 3 層 (もしくは `gateway → core` の 2 層) を流れる。すべての呼び出しに `X-Trace-Id` ヘッダを伝搬し、Cloudflare のログ全体で同一リクエストを追跡できるようにしている。

```
ブラウザ
  ↓ (X-Trace-Id: 自動生成、なければ gateway が UUID v4 を発行)
gateway-worker
  ↓ (X-Trace-Id 伝搬、認証ヘッダ付与)
bff-worker
  ↓ (X-Trace-Id 伝搬、CallContext.with_trace_id)
core-worker (journal/identity/safety/dialog)
  ↓ (二段ホップの場合: dialog → safety にも伝搬)
safety-worker
```

各 worker のログには `X-Trace-Id` を出力する慣習で、Tail Workers + ログ集約で 1 リクエストの全ホップを再構成できる。

## Tail Workers (リアルタイムログ)

### コマンド

```bash
# 単一 worker のログをリアルタイム表示
wrangler tail --env staging --name trauma-journal-bff-staging --format pretty

# JSON 出力 (集約用)
wrangler tail --env staging --name trauma-journal-gateway-staging --format json | jq
```

### フィルタ

```bash
# 特定の Trace ID で絞る
wrangler tail --env staging \
  --name trauma-journal-gateway-staging \
  --search "trace_id=abc-123"

# エラーのみ
wrangler tail --env staging \
  --name trauma-journal-gateway-staging \
  --status error
```

### 制限事項

- Tail Workers は **接続中のリクエスト** のみ取得 (履歴は保持しない)
- 接続の上限が低い (1 worker あたり数本) ため、長時間運用には不向き
- 履歴ログは Logpush に送ることで永続化する

## Logpush (永続ログ)

production では Logpush を有効化し、Workers Trace Events を R2 / S3 / Datadog 等に流す。

### Cloudflare ダッシュボードから設定

Workers & Pages → Workers Trace Events → Create Job:

- Destination: R2 bucket (例: `trauma-journal-logs-production`)
- Filter: `Status code >= 400` (4xx/5xx のみ送る場合)
- Frequency: `5 minutes`

R2 への蓄積は安価で、後から集約クエリができる。

### スキーマ

各イベントは JSON で:

```json
{
  "ScriptName": "trauma-journal-gateway",
  "EventTimestampMs": 1730000000000,
  "Outcome": "ok",
  "Logs": [
    { "Level": "info", "Message": ["request received", { "trace_id": "abc" }] }
  ],
  "Exceptions": []
}
```

`Logs[].Message` はワーカーが `console.log` した内容そのまま。

## ログの書き方 (worker-rs)

各 worker は構造化ログを `worker::console_log!` または `wasm_bindgen::JsValue` で出力する。例:

```rust
use worker::*;

console_log!(
    "trace_id={} user_id={} path={} status={}",
    trace_id.as_deref().unwrap_or("-"),
    user_id,
    path,
    status
);
```

Phase 1 までは `console_log!` を平文で使っているが、構造化ログ化のためには:

```rust
let log_event = serde_json::json!({
    "trace_id": trace_id,
    "user_id": user_id,
    "path": path,
    "status": status,
    "level": "info",
});
console_log!("{}", log_event);
```

これを Logpush 集約後に JSON パースすれば、`trace_id` で全 worker のログを横断検索できる。

## メトリクス

### Cloudflare Analytics

Workers & Pages → 各 worker → Metrics タブ:

- リクエスト数
- エラー率 (4xx, 5xx)
- 応答時間 (P50 / P95 / P99)
- CPU 時間

無料プランでも 24 時間分は閲覧可能。Workers Paid プランで 7 日間。

### Workers Analytics Engine (有料)

カスタムメトリクスを蓄積したい場合:

```rust
// 仮: dialog-worker で AI 利用回数をカウント
let analytics = env.analytics_engine("ANALYTICS")?;
analytics.write_data_point(AnalyticsEngineDataPoint {
    indexes: vec![user_id.clone()],
    blobs: vec![route.to_string(), language.to_string()],
    doubles: vec![char_count as f64],
})?;
```

このデータは SQL ライクなクエリで集約できる。

## アラート

### Cloudflare Notifications

Account → Notifications → Add → Workers Errors:

- Trigger: error rate > 5% over 5 minutes
- Targets: メールアドレスまたは Webhook (Slack / PagerDuty)

### カスタムアラート (Workers + Webhook)

別 worker (例: `monitor-worker`) を定期実行し、Cloudflare GraphQL API で各 worker のメトリクスを取得して閾値超過なら Slack に通知:

```rust
// 概念コード
async fn check_error_rate(env: &Env) -> Result<()> {
    let stats = fetch_cloudflare_graphql(env, "trauma-journal-gateway").await?;
    if stats.error_rate > 0.05 {
        post_to_slack(env, format!("⚠️ gateway error rate: {}%", stats.error_rate * 100.0)).await?;
    }
    Ok(())
}
```

これは Phase 3 の運用整備で導入する想定。

## デバッグの実践: 1 リクエストを追う

ユーザーから「dashboard が 500 になる」報告を受けた場合:

1. ブラウザの DevTools → Network → 該当リクエストのレスポンスヘッダ `X-Trace-Id` を確認 (例: `abc-123`)
2. Logpush 蓄積先 (R2) で `trace_id=abc-123` を grep:

```bash
aws s3 sync s3://trauma-journal-logs-production/2026/01/15/ ./logs/
grep -r "abc-123" ./logs/ | sort -k 1
```

3. gateway → bff → 各 core のログが時系列で出るので、どこで異常が起きたか特定

4. 該当 worker のコードバグなら `wrangler rollback` (`docs/deployment/rollback.md` 参照)

## ログの個人情報取扱い

- **平文の email や日記本文は絶対にログに出さない**
- ハッシュ化した `user_id` のみログ出力可
- safety-worker で危機検知された場合のみ、運用者が把握できるよう特別ログ (ただし内容は出さず、検知レベルのみ)

```rust
// 良い例
console_log!("crisis detected user={} level={:?}", user_id_hash, level);

// 悪い例 (やってはいけない)
console_log!("user said: {}", user_message); // 内容を漏らす
```

## ヘルスチェック

各 worker は `/_health` (内部 worker) または `/api/health` (gateway) を持つ。Cloudflare Health Checks で定期監視:

```bash
curl https://gateway.YOUR-DOMAIN.workers.dev/api/health
# 期待: HTTP 200, body "ok"
```

Notifications で Health Check 失敗を Slack 連動。

## まとめ: 運用のチェックリスト

- [ ] Logpush 設定済み (production)
- [ ] Cloudflare Notifications 設定済み (error rate, health check)
- [ ] `X-Trace-Id` がフロントエンドから末端の core まで伝搬している
- [ ] ログに個人情報を含めていないことをコードレビューでチェック
- [ ] 週次でエラー率レビュー
- [ ] インシデント発生時は trace_id ベースで 1 リクエストを追う運用が定着
