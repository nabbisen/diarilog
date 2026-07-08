use crate::handlers::dashboard;
use crate::ssr::layout::{wrap_document, HydrationConfig};
use leptos::prelude::*;
use web_app::{
    dashboard_data_to_json,
    i18n::{html_dir, normalize_to_translated},
    route_to_json, App, Route,
};
use worker::{Env, Headers, Request, Response, RouteContext, Result};

fn wants_html(req: &Request) -> bool {
    req.headers().get("Accept").ok().flatten()
        .map(|v| v.contains("text/html") || v.contains("*/*"))
        .unwrap_or(true)
}

fn html_response(body: String) -> Result<Response> {
    let resp = Response::ok(body)?;
    let headers = Headers::new();
    headers.set("Content-Type", "text/html; charset=utf-8")?;
    headers.set("Cache-Control", "private, no-store")?;
    Ok(resp.with_headers(headers))
}

pub fn resolve_lang(req: &Request) -> String {
    let accept = req.headers().get("Accept-Language")
        .ok().flatten().unwrap_or_default();
    let primary = accept.split(',').next().unwrap_or("")
        .split(';').next().unwrap_or("")
        .split('-').next().unwrap_or("")
        .trim().to_lowercase();
    normalize_to_translated(if primary.is_empty() { "en" } else { &primary }).to_string()
}

fn render_route(env: &Env, title: &str, route: Route, lang: String) -> Result<Response> {
    let body = view! { <App route=route.clone() lang=lang.clone() /> }.to_html();
    let assets_base_url = env.var("WEB_ASSETS_BASE_URL")
        .map(|v| v.to_string()).unwrap_or_default();
    let hydration = HydrationConfig {
        assets_base_url,
        route_json: route_to_json(&route),
        data_json: dashboard_data_to_json(&route),
        lang: lang.clone(),
    };
    let dir = html_dir(&lang);
    html_response(wrap_document(title, &body, &lang, dir, Some(&hydration)))
}

pub async fn index(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if !wants_html(&req) { return Response::error("Not Acceptable", 406); }
    render_route(&ctx.env, "diarilog", Route::Index, resolve_lang(&req))
}

pub async fn login(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if !wants_html(&req) { return Response::error("Not Acceptable", 406); }
    let lang = resolve_lang(&req);
    let issuer = ctx.env.var("OIDC_ISSUER").map(|v| v.to_string()).unwrap_or_default();
    render_route(&ctx.env, "Sign in — diarilog", Route::Login { issuer }, lang)
}

pub async fn onboarding(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if !wants_html(&req) { return Response::error("Not Acceptable", 406); }
    render_route(&ctx.env, "Set up your passphrase — diarilog",
        Route::Onboarding, resolve_lang(&req))
}

pub async fn dashboard(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if !wants_html(&req) { return Response::error("Not Acceptable", 406); }
    let lang = resolve_lang(&req);
    // Pre-render with data so the initial HTML is populated.
    let data = dashboard::aggregate_data_for_ssr(&req, &ctx.env).await;
    render_route(&ctx.env, "Dashboard — diarilog", Route::Dashboard { data }, lang)
}

pub async fn settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if !wants_html(&req) { return Response::error("Not Acceptable", 406); }
    render_route(&ctx.env, "Settings — diarilog", Route::Settings, resolve_lang(&req))
}
