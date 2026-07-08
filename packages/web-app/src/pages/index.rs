//! トップページ `/` のコンポーネント。

use crate::i18n::t;
use leptos::prelude::*;

#[component]
pub fn IndexPage(lang: String) -> impl IntoView {
    let l = lang.clone();
    view! {
        <main>
            <h1>{t(&l, "index-title")}</h1>
            <p class="muted">{t(&l, "index-tagline")}</p>
            <p>{t(&l, "index-description")}</p>
            <p><a href="/login" class="btn">{t(&l, "sign-in")}</a></p>
            <div class="notice muted">
                <p>{t(&l, "index-skeleton-notice")}</p>
            </div>
        </main>
    }
}
