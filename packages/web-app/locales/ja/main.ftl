# Project Fluent translation file (ja)
#
# 本ファイルは Trauma Journal Platform のフロントエンドに表示される
# UI 文字列をすべて含む。翻訳者向けの注意点:
#
# - Fluent 構文: https://projectfluent.org/fluent/guide/
# - 機能名のキーは `kebab-case` 形式で命名する
# - 危機的状況に関する文言 (`crisis-` プレフィックス) は専門家監修必須

## ── 共通 ──
brand-name = Trauma Journal
back-to-top = トップへ戻る
sign-in = サインイン
loading = 読み込み中...

## ── トップページ ──
index-title = Trauma Journal
index-tagline = トラウマケア・ジャーナリングプラットフォーム
index-description = 安心して書き始められる、自分のための記録空間です。
index-skeleton-notice = 本ページは Leptos v0.8 SSR で生成されています。

## ── ログインページ ──
login-title = サインイン
login-prompt = 以下のボタンから OIDC プロバイダにリダイレクトしてサインインします。
login-issuer-label = issuer:
login-flow-not-implemented = (リダイレクトフローは現在のスケルトンでは未実装です)
login-issuer-missing = OIDC プロバイダが未設定です。サーバー設定を確認してください。

## ── ダッシュボードページ ──
dashboard-title = ダッシュボード
dashboard-greeting = ようこそ、{ $name } さん
dashboard-greeting-guest = ゲスト
dashboard-active-session-heading = 進行中の対話があります。
dashboard-active-session-resume = 対話を再開する
dashboard-recent-heading = 最近のジャーナル
dashboard-recent-empty = まだ日記がありません。
dashboard-recent-fetch-failed = (現在、最近の日記を取得できませんでした。あとで再読込してください)
dashboard-mood-label = mood:
dashboard-partial-degradation = 一部のデータを取得できませんでした。時間をおいて再読込してください。
dashboard-unauthenticated = ダッシュボードのデータを取得できませんでした。ログインしているか確認してから、再読込してください。

## ── 404 ──
not-found-title = 404 - 見つかりませんでした

## ── 設定ページ ──
settings-title = 設定
settings-profile-heading = プロフィール
settings-language-heading = 言語
settings-danger-heading = 危険な操作
settings-erase-heading = すべてのデータを消去
settings-erase-description =
    プロフィール、すべての日記、すべての対話セッション、すべての設定を
    このデバイスとサーバーの両方から完全に削除します。
    復元することはできません。
settings-erase-suggestion = 先にデータをエクスポートすることをおすすめします。
settings-erase-export-link = データをエクスポート
settings-erase-confirm-label = 確認のため「{ $word }」と入力してください
settings-erase-confirm-word = 消去
settings-erase-button = すべて消去する
settings-erase-progress = 消去中…
settings-erase-done = データが消去されました。

## ── オンボーディング ──
onboarding-title = diarilog へようこそ
onboarding-intro =
    始める前に、暗号化パスフレーズを設定してください。
    このパスフレーズで日記が暗号化され、あなただけが読めるようになります。
    パスフレーズはリセットできません。忘れると日記を復元できなくなります。
onboarding-passphrase-label = パスフレーズを設定
onboarding-passphrase-confirm-label = パスフレーズを再入力
onboarding-passphrase-hint =
    覚えやすいものを選んでください。続ける前に安全な場所にメモしてください。
onboarding-passphrase-mismatch = パスフレーズが一致しません。
onboarding-warning-label =
    パスフレーズを忘れるとデータを復元できないことを理解しました。
onboarding-continue = 設定して続ける
onboarding-passphrase-strength-weak = パスフレーズが弱すぎます。文字や単語を追加してください。
