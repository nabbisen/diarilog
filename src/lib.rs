//! diarilog — single Cloudflare Worker.
//!
//! Consolidates what was previously 6 separate workers (gateway, bff,
//! journal, identity, safety, dialog) into one. All routing is internal;
//! no Service Bindings are used.

use worker::*;

pub mod auth;
pub mod dialog;
pub mod handlers;
pub mod safety;
pub mod ssr;
pub mod storage;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::new()
        // ── Static assets (Workers Static Assets binding) ─────────────────
        // Served by the [assets] binding without invoking worker code.
        // Listed here only for documentation; wrangler handles them.

        // ── SSR pages ─────────────────────────────────────────────────────
        .get_async("/", ssr::handlers::index)
        .get_async("/login", ssr::handlers::login)
        .get_async("/onboarding", ssr::handlers::onboarding)
        .get_async("/dashboard", ssr::handlers::dashboard)
        .get_async("/settings", ssr::handlers::settings)

        // ── Auth ──────────────────────────────────────────────────────────
        .get_async("/auth/callback", handlers::auth::callback)
        .post_async("/api/auth/verify-turnstile", handlers::auth::verify_turnstile)

        // ── Health ────────────────────────────────────────────────────────
        .get("/api/health", |_, _| Response::ok("ok"))

        // ── Dashboard aggregation ─────────────────────────────────────────
        .get_async("/api/dashboard", handlers::dashboard::aggregate)

        // ── User profile ──────────────────────────────────────────────────
        .get_async("/api/me", handlers::identity::get_profile)
        .put_async("/api/me", handlers::identity::update_profile)

        // ── Diary CRUD ────────────────────────────────────────────────────
        .post_async("/api/diary", handlers::diary::create_entry)
        .get_async("/api/diary", handlers::diary::list_entries)
        .get_async("/api/diary/:diary_id", handlers::diary::get_entry)
        .put_async("/api/diary/:diary_id", handlers::diary::update_entry)
        .delete_async("/api/diary/:diary_id", handlers::diary::delete_entry)

        // ── Version history ───────────────────────────────────────────────
        .get_async("/api/diary/:diary_id/versions", handlers::diary::list_versions)
        .get_async("/api/diary/:diary_id/versions/:version", handlers::diary::get_version)
        .delete_async("/api/diary/:diary_id/versions/:version", handlers::diary::delete_version)

        // ── Interview + AI ────────────────────────────────────────────────
        .post_async("/api/interview/start", handlers::dialog::start_session)
        .post_async("/api/interview/answer", handlers::dialog::submit_answer)
        .get_async("/api/interview/:session_id", handlers::dialog::get_session)
        .post_async("/api/suggest", handlers::dialog::generate_draft)

        // ── Trigger keywords ──────────────────────────────────────────────
        .get_async("/api/triggers", handlers::safety::list_triggers)
        .post_async("/api/triggers", handlers::safety::add_trigger)
        .delete_async("/api/triggers/:trigger_id", handlers::safety::remove_trigger)

        // ── Sync + erase ──────────────────────────────────────────────────
        .post_async("/api/sync", handlers::diary::sync_data)
        .post_async("/api/erase", handlers::erase::erase_all)

        .run(req, env)
        .await
}
