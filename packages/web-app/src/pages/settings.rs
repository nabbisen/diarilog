use crate::i18n::{t, t_with};
use fluent_templates::fluent_bundle::FluentValue;
use leptos::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

#[component]
pub fn SettingsPage(lang: String) -> impl IntoView {
    view! {
        <main>
            <h1>{t(&lang, "settings-title")}</h1>
            <EraseSection lang />
        </main>
    }
}

#[component]
fn EraseSection(lang: String) -> impl IntoView {
    let confirm_word  = t(&lang, "settings-erase-confirm-word");
    let mut args: HashMap<Cow<'static, str>, FluentValue<'_>> = HashMap::new();
    args.insert(Cow::Borrowed("word"), FluentValue::from(confirm_word.clone()));
    let confirm_label = t_with(&lang, "settings-erase-confirm-label", &args);

    view! {
        <div class="erase-section" style="margin-top: 3rem; padding-top: 1.5rem; border-top: 1px solid var(--border);">
            <h2 style="color: #c0392b;">{t(&lang, "settings-erase-heading")}</h2>
            <p class="muted">{t(&lang, "settings-erase-description")}</p>
            <p class="muted">
                {t(&lang, "settings-erase-suggestion")}
                " "
                <a href="/settings/export">{t(&lang, "settings-erase-export-link")}</a>
            </p>

            <form method="post" action="/api/erase" id="erase-form"
                  data-confirm-word=confirm_word.clone()
                  style="margin-top: 1rem;">
                <div class="form-group">
                    <label for="erase-confirm">{confirm_label}</label>
                    <input type="text" id="erase-confirm" name="confirm"
                           autocomplete="off" autocapitalize="none"
                           spellcheck="false" class="erase-confirm-input" />
                </div>
                <button type="submit" class="btn btn--danger" disabled=true>
                    {t(&lang, "settings-erase-button")}
                </button>
            </form>

            <p id="erase-done" class="muted hidden">
                {t(&lang, "settings-erase-done")}
            </p>
        </div>
    }
}
