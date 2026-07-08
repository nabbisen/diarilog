# Deploy Order

Worker のデプロイ順序とその理由を説明します。

## 理由: Service Bindings の依存解決

Cloudflare の Service Bindings は **被呼び出し側の Worker が存在していないと、呼び出し側のデプロイが拒否される** ことがあります。したがって依存関係の葉から順にデプロイします。

## 推奨順序

```
1. identity-worker
2. safety-worker
3. journal-worker
4. dialog-worker  ← safety-worker に依存 (二段ホップ)
5. gateway-worker ← 上記 4 つすべてに依存
```

## 依存関係グラフ

```
gateway ──┬──→ journal
          ├──→ identity
          ├──→ safety  ←──┐
          └──→ dialog ────┘ (二段ホップ)
```

## 環境ごとの実施例

### dev / staging へのデプロイ

```bash
# 各 Worker ディレクトリで実施
(cd workers/identity && wrangler deploy --env dev)
(cd workers/safety   && wrangler deploy --env dev)
(cd workers/journal  && wrangler deploy --env dev)
(cd workers/dialog   && wrangler deploy --env dev)
(cd workers/gateway  && wrangler deploy --env dev)
```

`--env staging` に変えれば staging 環境にデプロイされます。

### production へのデプロイ

```bash
(cd workers/identity && wrangler deploy --env production)
(cd workers/safety   && wrangler deploy --env production)
(cd workers/journal  && wrangler deploy --env production)
(cd workers/dialog   && wrangler deploy --env production)
(cd workers/gateway  && wrangler deploy --env production)
```

production への切り替えは段階的に: 各 Worker の機能フラグ (`USE_SB_*`) を `false` で先にデプロイし、staging で検証してから手動で `true` に切り替える運用が推奨されます。詳細は [`feature-flags.md`](./feature-flags.md) 参照。

## 2 回目以降のデプロイ

リソース名 (Worker 名 / バインディング名) が変わらなければ任意順で構いません。ただし、Service Bindings の interface (リクエスト / レスポンスの型) を破壊的に変更する場合は、被呼び出し側を先にデプロイします。

## CI/CD での実施

`.github/workflows/deploy.yml` で上記順序を強制する想定です (Phase 1.6 で整備予定)。
