//! ダッシュボードページ `/dashboard` のコンポーネント。
//!
//! SSR 時には bff の集約 API から取得した `DashboardResponse` を props として
//! 受け取り、ユーザー名・最新日記・進行中セッションを直接描画する。

use crate::i18n::{t, t_with};
use contracts::bff::DashboardResponse;
use fluent_templates::fluent_bundle::FluentValue;
use leptos::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

#[component]
pub fn DashboardPage(data: Option<DashboardResponse>, lang: String) -> impl IntoView {
    match data {
        Some(d) => view! { <DashboardWithData data=d lang=lang.clone() /> }.into_any(),
        None => view! { <DashboardEmpty lang=lang.clone() /> }.into_any(),
    }
}

#[component]
fn DashboardWithData(data: DashboardResponse, lang: String) -> impl IntoView {
    let user_name = data
        .user
        .as_ref()
        .and_then(|u| u.display_name.clone())
        .unwrap_or_else(|| t(&lang, "dashboard-greeting-guest"));

    let diary_count = data.recent_diaries.len();
    let has_active = data.active_session.is_some();
    let status = data.status.clone();

    // 挨拶 (引数付き)
    let mut greeting_args = HashMap::new();
    greeting_args.insert(Cow::Borrowed("name"), FluentValue::from(user_name.as_str()));
    let greeting = t_with(&lang, "dashboard-greeting", &greeting_args);

    let l_title = lang.clone();
    let l_active = lang.clone();
    let l_active_btn = lang.clone();
    let l_recent_h = lang.clone();
    let l_recent_empty = lang.clone();
    let l_recent_fail = lang.clone();
    let l_mood = lang.clone();
    let l_partial = lang.clone();
    let l_back = lang.clone();

    view! {
        <main>
            <h1>{t(&l_title, "dashboard-title")}</h1>
            <p class="muted">{greeting}</p>

            // ── 進行中のインタビューセッション ──
            {if has_active {
                view! {
                    <div class="notice">
                        <p><strong>{t(&l_active, "dashboard-active-session-heading")}</strong></p>
                        <p><a href="/journal/interview" class="btn">
                            {t(&l_active_btn, "dashboard-active-session-resume")}
                        </a></p>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}

            // ── 最近の日記 ──
            <h2>{t(&l_recent_h, "dashboard-recent-heading")}</h2>
            {if diary_count == 0 {
                if status.recent_diaries_ok {
                    view! { <p class="muted">{t(&l_recent_empty, "dashboard-recent-empty")}</p> }.into_any()
                } else {
                    view! {
                        <p class="muted">
                            {t(&l_recent_fail, "dashboard-recent-fetch-failed")}
                        </p>
                    }.into_any()
                }
            } else {
                let mood_label = t(&l_mood, "dashboard-mood-label");
                view! {
                    <ul>
                        {data.recent_diaries.iter().map(|entry| {
                            let id = entry.id.clone();
                            let created = entry.created_at.clone();
                            // encrypted_title and encrypted_mood are ciphertext on the
                            // server. The browser hydrates and decrypts them client-side
                            // (RFC 011). In the SSR render we show a placeholder so
                            // the layout is stable before hydration completes.
                            let title_placeholder = "(encrypted)".to_string();
                            let has_mood = entry.encrypted_mood.is_some();
                            let m_label = mood_label.clone();
                            view! {
                                <li>
                                    <a href=format!("/journal/{}", id)
                                       class="diary-title"
                                       data-diary-id=id.clone()
                                       data-encrypted-title=entry.encrypted_title.clone().unwrap_or_default()>
                                       {title_placeholder}
                                    </a>
                                    <span class="muted">" — " {created}</span>
                                    {if has_mood {
                                        view! {
                                            <span class="muted"
                                                  data-encrypted-mood=entry.encrypted_mood.clone().unwrap_or_default()>
                                                " (" {m_label.clone()} " ···)"
                                            </span>
                                        }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }}
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                }.into_any()
            }}

            // ── 部分的劣化警告 ──
            {if !status.user_ok || !status.recent_diaries_ok || !status.active_session_ok {
                view! {
                    <div class="notice muted">
                        {t(&l_partial, "dashboard-partial-degradation")}
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}

            <p><a href="/" class="muted">{t(&l_back, "back-to-top")}</a></p>
        </main>
    }
}

#[component]
fn DashboardEmpty(lang: String) -> impl IntoView {
    let l1 = lang.clone();
    let l2 = lang.clone();
    let l3 = lang.clone();
    let l4 = lang.clone();
    view! {
        <main>
            <h1>{t(&l1, "dashboard-title")}</h1>
            <p class="muted">{t(&l2, "dashboard-unauthenticated")}</p>
            <p><a href="/login" class="btn">{t(&l3, "sign-in")}</a></p>
            <p><a href="/" class="muted">{t(&l4, "back-to-top")}</a></p>
        </main>
    }
}
