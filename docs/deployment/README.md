# Deployment Documentation

本ディレクトリは Trauma Journal Platform を Cloudflare Workers にデプロイするための実務手順書です。

## 構成

| ドキュメント | 内容 | 主な読者 |
|---|---|---|
| [`prerequisites.md`](./prerequisites.md) | 必要なアカウント・ツール・前提リソース | 初めて構築する人 |
| [`initial-setup.md`](./initial-setup.md) | D1 / R2 / KV / Static Assets 作成、シークレット登録 | 新環境を立ち上げる人 |
| [`deploy-order.md`](./deploy-order.md) | Worker のデプロイ順序とその理由 | デプロイを実行する人 |
| [`multi-env.md`](./multi-env.md) | dev / staging / production の使い分け | 環境設計をする人 |
| [`feature-flags.md`](./feature-flags.md) | `USE_SB_*` フラグの段階的切替 (Phase 1 履歴) | Phase 1 の経緯を辿る人 |
| [`oidc-providers.md`](./oidc-providers.md) | Auth0 / Keycloak / Google / Azure AD 設定例 | 認証連携を実装する人 |
| [`rollback.md`](./rollback.md) | ロールバック手順、データ汚染時の対応 | 障害対応をする人 |
| [`observability.md`](./observability.md) | `X-Trace-Id`、Tail Workers、Logpush、アラート | 運用を維持する人 |

## クイックスタート

新規 Cloudflare アカウントに staging 環境を立ち上げる最短ルート:

1. [`prerequisites.md`](./prerequisites.md) でアカウントとローカルツールを準備
2. [`initial-setup.md`](./initial-setup.md) で D1 / R2 / KV / Secrets を作成
3. [`oidc-providers.md`](./oidc-providers.md) で OIDC プロバイダ側を設定
4. [`deploy-order.md`](./deploy-order.md) の順番で各 worker をデプロイ
5. [`../hydration-verification.md`](../hydration-verification.md) のチェックリストで動作確認

production への昇格は staging で 1 週間程度の安定運用後を推奨。

## 関連ドキュメント

- [`../hydration-verification.md`](../hydration-verification.md) — bff のハイドレーション動作の実機検証手順 (リポジトリのトップ `docs/` に置く)

## 現在のステータス

| ファイル | 状態 |
|---|---|
| README.md (このファイル) | ✅ v2.3 で完成 |
| prerequisites.md | ✅ |
| initial-setup.md | ✅ v2.3 で追加 |
| deploy-order.md | ✅ |
| multi-env.md | ✅ v2.3 で追加 |
| feature-flags.md | ✅ |
| oidc-providers.md | ✅ v2.3 で追加 |
| rollback.md | ✅ v2.3 で追加 |
| observability.md | ✅ v2.3 で追加 |

## Related

- [`../local-development.md`](../local-development.md) — why `wrangler dev` fails at the repo root and how to run workers locally
