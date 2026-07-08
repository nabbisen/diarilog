use crate::i18n::t;
use leptos::prelude::*;

#[component]
pub fn LoginPage(issuer: String, lang: String) -> impl IntoView {
    let configured = !issuer.trim().is_empty();
    let login_url = format!("{}/authorize", issuer.trim_end_matches('/'));

    view! {
        <main>
            <h1>{t(&lang, "login-title")}</h1>
            {if configured {
                view! {
                    <p>
                        <a href=login_url class="btn">{t(&lang, "sign-in")}</a>
                    </p>
                }.into_any()
            } else {
                view! {
                    <p class="muted">{t(&lang, "login-not-configured")}</p>
                }.into_any()
            }}
        </main>
    }
}
