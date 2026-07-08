# Feature Flags

`USE_SB_*` フラグの段階的切替手順。

## フラグ一覧

| フラグ | 切り替え対象 | 既定値 (production) |
|---|---|---|
| `USE_SB_JOURNAL` | gateway → journal-worker への日記 CRUD 委譲 | `false` |
| `USE_SB_IDENTITY` | gateway → identity-worker へのプロフィール委譲 | `false` |
| `USE_SB_SAFETY` | gateway → safety-worker への安全性管理委譲 | `false` |
| `USE_SB_DIALOG` | gateway → dialog-worker への AI 対話委譲 | `false` |

## 動作仕様

- `true` のとき: gateway の該当ハンドラは Service Bindings 経由で内部 Worker にリクエストを転送
- `false` のとき: gateway 内部の従来コードがそのまま動作

両方の経路は同じ公開 API パスで動作するため、外部クライアントには切り替えが見えません。

## 段階的切替フロー

```
[ dev ]  USE_SB_*=true で常時稼働、コミットごとに動作確認
   ↓
[ staging ]  PR マージ時に USE_SB_*=false でまず展開
   ↓
[ staging ]  各フラグを 1 つずつ true に切替、E2E 検証
   ↓
[ staging ]  全フラグ true で 1 週間稼働
   ↓
[ production ]  まず USE_SB_*=false で展開
   ↓
[ production ]  USE_SB_JOURNAL=true → 24 時間モニタ
   ↓
[ production ]  USE_SB_IDENTITY=true → 24 時間モニタ
   ↓
[ production ]  USE_SB_SAFETY=true → 24 時間モニタ
   ↓
[ production ]  USE_SB_DIALOG=true → 24 時間モニタ
   ↓
[ Phase 1 完了 ]  全フラグを除去 (Step 7)
```

## フラグの実体

`workers/gateway/wrangler.toml` の `[vars]` に文字列として定義:

```toml
[vars]
USE_SB_JOURNAL = "false"
USE_SB_IDENTITY = "false"
USE_SB_SAFETY = "false"
USE_SB_DIALOG = "false"
```

各環境セクション (`[env.dev.vars]`, `[env.staging.vars]`, `[env.production.vars]`) でも個別に上書きできます。

## フラグの切替手順 (production)

1. `wrangler.toml` の `[env.production.vars]` を編集して当該フラグを `"true"` に変更
2. `git commit` + `git push` で main ブランチへ
3. CI が gateway-worker のみを再デプロイ (CI 設計上、内部 Worker 側には変更が無いため)
4. Cloudflare ダッシュボードで Tail Workers を有効化し、エラー率を 1 時間モニタ
5. 異常があれば次の手順「ロールバック」へ

## ロールバック手順

詳細は [`rollback.md`](./rollback.md) 予定。

最短経路:
1. `wrangler.toml` の該当フラグを `"false"` に戻す
2. gateway-worker を再デプロイ
3. 即座に従来経路に戻る (内部 Worker への呼び出しは停止)

## 注意事項

- フラグは **gateway-worker のみ** に影響します。内部 Worker 自体は常に動作可能な状態でデプロイされます
- erase ハンドラは `EraseScope` 構造体を介してフラグを評価し、各 Worker の所管に応じて消去責務を割り振ります (組み合わせ爆発を防ぐ宣言的設計、Step 4 で導入)
