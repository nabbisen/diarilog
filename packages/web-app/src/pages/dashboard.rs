use crate::i18n::t;
use contracts::bff::DashboardResponse;
use leptos::prelude::*;

#[component]
pub fn DashboardPage(data: Option<DashboardResponse>, lang: String) -> impl IntoView {
    match data {
        Some(d) => view! { <Dashboard data=d lang /> }.into_any(),
        None => view! { <DashboardUnauthenticated lang /> }.into_any(),
    }
}

#[component]
fn Dashboard(data: DashboardResponse, lang: String) -> impl IntoView {
    let has_active = data.active_session.is_some();
    let entries = data.recent_diaries;

    // Primary action: resume in-progress interview, or write a new entry.
    let (action_href, action_label) = if has_active {
        ("/journal/interview", t(&lang, "dashboard-resume"))
    } else {
        ("/journal/new", t(&lang, "dashboard-write"))
    };

    view! {
        <main>
            <p style="margin-bottom: 1.5rem;">
                <a href=action_href class="btn">{action_label}</a>
            </p>

            {if entries.is_empty() {
                view! {
                    <p class="muted">{t(&lang, "dashboard-no-entries")}</p>
                }.into_any()
            } else {
                view! {
                    <ul class="entry-list">
                        {entries.into_iter().map(|e| {
                            let href  = format!("/journal/{}", e.id);
                            let date  = e.created_at.get(..10).unwrap_or("").to_string();
                            view! {
                                <li>
                                    // data-encrypted-title is populated by JS after
                                    // hydration decrypts it (RFC 011).
                                    <a href=href
                                       data-diary-id=e.id
                                       data-encrypted-title=
                                           e.encrypted_title.unwrap_or_default()>
                                        {date}
                                    </a>
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                }.into_any()
            }}
        </main>
    }
}

#[component]
fn DashboardUnauthenticated(lang: String) -> impl IntoView {
    view! {
        <main>
            <p><a href="/login" class="btn">{t(&lang, "sign-in")}</a></p>
        </main>
    }
}
