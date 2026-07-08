//! ログインページ `/login` のコンポーネント。
//! OIDC issuer が設定されていればプレースホルダボタンを出す。

use crate::i18n::t;
use leptos::prelude::*;

#[component]
pub fn LoginPage(issuer: String, lang: String) -> impl IntoView {
    let has_issuer = !issuer.trim().is_empty();
    let issuer_display = issuer.trim_end_matches('/').to_string();

    let l1 = lang.clone();
    let l2 = lang.clone();
    let l3 = lang.clone();
    let l4 = lang.clone();

    view! {
        <main>
            <h1>{t(&l1, "login-title")}</h1>
            {move || if has_issuer {
                let lp = l2.clone();
                let li = l3.clone();
                let lf = l4.clone();
                let issuer_d = issuer_display.clone();
                view! {
                    <div>
                        <p>{t(&lp, "login-prompt")}</p>
                        <p class="muted">{t(&li, "login-issuer-label")} " " {issuer_d}</p>
                        <p>
                            <a href="#" class="btn">
                                {t(&lf, "sign-in")}
                            </a>
                        </p>
                        <p class="muted">
                            {t(&lf, "login-flow-not-implemented")}
                        </p>
                    </div>
                }.into_any()
            } else {
                let lm = l2.clone();
                view! {
                    <p class="muted">
                        {t(&lm, "login-issuer-missing")}
                    </p>
                }.into_any()
            }}
            <p><a href="/" class="muted">{t(&lang, "back-to-top")}</a></p>
        </main>
    }
}
