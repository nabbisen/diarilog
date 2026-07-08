//! Settings page `/settings`.
//!
//! Divided into three zones:
//!
//! 1. **Profile** — display name, email (read-only from OIDC).
//! 2. **Language** — language picker (reads current setting, links to
//!    `POST /api/lang` for change).
//! 3. **Danger zone** — emergency erase, visually separated and at the
//!    bottom. The confirmation word (`ERASE` / localized) must be typed
//!    correctly before the submit button is enabled.
//!
//! ## Emergency erase UX (RFC 010)
//!
//! The user types the confirmation word, hits "Erase everything", and the
//! client posts to `POST /api/erase`. On success the page replaces itself
//! with a brief confirmation message and redirects to `/` after 3 seconds.

use crate::i18n::{t, t_with};
use fluent_templates::fluent_bundle::FluentValue;
use leptos::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

/// Top-level settings page component.
#[component]
pub fn SettingsPage(lang: String) -> impl IntoView {
    let l = lang.clone();
    view! {
        <main class="settings-page">
            <h1>{t(&l, "settings-title")}</h1>

            <section class="settings-section" id="profile">
                <h2>{t(&l, "settings-profile-heading")}</h2>
                <p class="muted">"(Profile editing — coming soon)"</p>
            </section>

            <section class="settings-section" id="language">
                <h2>{t(&l, "settings-language-heading")}</h2>
                <p class="muted">"(Language picker — see RFC 004)"</p>
            </section>

            // ── Danger zone ────────────────────────────────────────────────
            // Visually isolated: rule above, muted-red border, large top margin.
            // Must never be adjacent to a normal settings action. (RFC 010)
            <div class="danger-zone-separator" role="separator" aria-hidden="true" />

            <section class="settings-section settings-section--danger" id="erase">
                <h2>{t(&l, "settings-danger-heading")}</h2>
                <EraseSection lang=lang.clone() />
            </section>
        </main>
    }
}

/// Emergency erase sub-section with two-step typed confirmation.
#[component]
fn EraseSection(lang: String) -> impl IntoView {
    let confirm_word = t(&lang, "settings-erase-confirm-word");

    // Build the confirm-label string with the word substituted in.
    let mut label_args: HashMap<Cow<'static, str>, FluentValue<'_>> = HashMap::new();
    label_args.insert(Cow::Borrowed("word"), FluentValue::from(confirm_word.clone()));
    let confirm_label = t_with(&lang, "settings-erase-confirm-label", &label_args);

    let l1 = lang.clone();
    let l2 = lang.clone();
    let l3 = lang.clone();
    let l4 = lang.clone();
    let l5 = lang.clone();
    let l6 = lang.clone();

    view! {
        <div class="erase-section">
            <h3>{t(&l1, "settings-erase-heading")}</h3>
            <p>{t(&l2, "settings-erase-description")}</p>

            <p class="erase-suggestion">
                {t(&l3, "settings-erase-suggestion")}
                " "
                <a href="/settings/export">{t(&l4, "settings-erase-export-link")}</a>
            </p>

            // Plain HTML form: works before hydration completes.
            // JS enhances by enabling the button on input match and replacing
            // the form with the done-message on success.
            <form
                method="post"
                action="/api/erase"
                class="erase-form"
                id="erase-form"
                data-confirm-word=confirm_word.clone()
            >
                <label for="erase-confirm" class="erase-confirm-label">
                    {confirm_label}
                </label>
                <input
                    type="text"
                    id="erase-confirm"
                    name="confirm"
                    autocomplete="off"
                    autocapitalize="none"
                    spellcheck="false"
                    aria-required="true"
                    class="erase-confirm-input"
                />
                <button
                    type="submit"
                    class="btn btn--danger erase-submit"
                    disabled=true
                >
                    {t(&l5, "settings-erase-button")}
                </button>
            </form>

            // Shown by JS after successful erase; hidden until then.
            <p id="erase-done" class="erase-done hidden">
                {t(&l6, "settings-erase-done")}
            </p>
        </div>
    }
}
