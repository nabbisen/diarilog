use crate::i18n::t;
use leptos::prelude::*;

#[component]
pub fn IndexPage(lang: String) -> impl IntoView {
    view! {
        <main>
            <p>{t(&lang, "index-intro")}</p>
            <p class="muted">{t(&lang, "index-trust")}</p>
            <p style="margin-top: 1.5rem;">
                <a href="/login" class="btn">{t(&lang, "sign-in")}</a>
            </p>
        </main>
    }
}
