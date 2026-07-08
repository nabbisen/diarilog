//! First-session onboarding page `/onboarding`.
//!
//! ## Purpose (RFC 011)
//!
//! A user who has just authenticated via OIDC for the first time — or who
//! has not yet completed passphrase setup — is redirected here before being
//! allowed to write anything. The page:
//!
//! 1. Explains the E2EE model in plain language.
//! 2. Collects a passphrase (entry + re-entry confirmation).
//! 3. Warns explicitly that the passphrase is unrecoverable.
//! 4. On submit (POST `/api/onboarding/passphrase`), the browser's JS
//!    derives the KEK, generates the DEK, wraps the DEK, and posts the
//!    wrapped material to the server.
//!
//! ## Server-side rendering
//!
//! This page is SSR-only with no data dependencies; it is fast. JS
//! hydration adds:
//! - Client-side passphrase match validation (before POST)
//! - Passphrase strength indication
//! - The actual key-derivation computation (Argon2id WASM)
//! - Enabling/disabling the submit button
//!
//! If JS is not available the form still submits — but the server cannot
//! do the crypto, so it will return an error telling the user to enable JS.
//! This is acceptable: E2EE *requires* client-side crypto.

use crate::i18n::t;
use leptos::prelude::*;

/// Onboarding page — passphrase setup.
#[component]
pub fn OnboardingPage(lang: String) -> impl IntoView {
    let l = lang.clone();
    view! {
        <main class="onboarding-page">
            <h1>{t(&l, "onboarding-title")}</h1>
            <div class="onboarding-intro">
                <p>{t(&l, "onboarding-intro")}</p>
            </div>

            // The form action is intentionally a real endpoint so that the
            // page is usable even while hydration is loading. JS replaces the
            // submit handler to run the crypto before posting.
            <form
                method="post"
                action="/api/onboarding/passphrase"
                id="onboarding-form"
                class="onboarding-form"
            >
                <div class="form-group">
                    <label for="passphrase">{t(&l, "onboarding-passphrase-label")}</label>
                    <input
                        type="password"
                        id="passphrase"
                        name="passphrase"
                        autocomplete="new-password"
                        required=true
                        class="passphrase-input"
                    />
                    // Hydrate inserts a strength indicator here.
                    <div id="passphrase-strength" class="passphrase-strength hidden" />
                </div>

                <div class="form-group">
                    <label for="passphrase-confirm">
                        {t(&l, "onboarding-passphrase-confirm-label")}
                    </label>
                    <input
                        type="password"
                        id="passphrase-confirm"
                        name="passphrase_confirm"
                        autocomplete="new-password"
                        required=true
                        class="passphrase-input"
                    />
                    <p id="passphrase-mismatch" class="error-msg hidden">
                        {t(&l, "onboarding-passphrase-mismatch")}
                    </p>
                </div>

                <p class="passphrase-hint muted">{t(&l, "onboarding-passphrase-hint")}</p>

                <div class="form-group form-group--checkbox">
                    <input
                        type="checkbox"
                        id="warning-acknowledged"
                        name="warning_acknowledged"
                        required=true
                    />
                    <label for="warning-acknowledged" class="warning-label">
                        {t(&l, "onboarding-warning-label")}
                    </label>
                </div>

                <button
                    type="submit"
                    class="btn btn--primary"
                    id="onboarding-submit"
                >
                    {t(&l, "onboarding-continue")}
                </button>
            </form>
        </main>
    }
}
