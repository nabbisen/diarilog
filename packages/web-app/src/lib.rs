//! Trauma Journal フロントエンドコンポーネント (Leptos v0.8)。
//!
//! このクレートは SSR と Hydrate の両方からビルドされる:
//!
//! - **SSR ビルド** (`--features ssr`): gateway-worker が WASM-on-Workers として
//!   実行する際、本クレートの `App` を `to_html()` で文字列化する
//! - **Hydrate ビルド** (`--features hydrate`): wasm-pack 経由でブラウザ用
//!   バンドルを生成し、SSR された DOM に `hydrate_body` で再アタッチする
//!
//! 同じコンポーネント定義を両方で使うことが、ハイドレーション整合性 (= サーバー
//! 出力とクライアント期待値が一致すること) の前提となる。

pub mod i18n;
pub mod pages;

use contracts::bff::DashboardResponse;
use leptos::prelude::*;
use pages::{
    dashboard::DashboardPage, index::IndexPage, login::LoginPage, onboarding::OnboardingPage,
    settings::SettingsPage,
};

/// The current page. SSR resolves this from the URL; hydrate reads it back
/// from `window.__DIARILOG_ROUTE__`.
#[derive(Clone, Debug)]
pub enum Route {
    Index,
    Login { issuer: String },
    Onboarding,
    Dashboard { data: Option<DashboardResponse> },
    Settings,
    NotFound,
}

/// Root component. Dispatches to the right page based on `route`.
///
/// `lang` carries the resolved display-language code ("ja", "en", etc.).
/// It is a separate prop from `route` because language is an orthogonal
/// dimension — `Route` describes *which* page; `lang` describes *how* to
/// render it.
#[component]
pub fn App(route: Route, lang: String) -> impl IntoView {
    let lang_for_not_found = lang.clone();
    view! {
        <div class="app-root">
            {match route {
                Route::Index => view! { <IndexPage lang=lang.clone() /> }.into_any(),
                Route::Login { issuer } => view! { <LoginPage issuer=issuer lang=lang.clone() /> }.into_any(),
                Route::Onboarding => view! { <OnboardingPage lang=lang.clone() /> }.into_any(),
                Route::Dashboard { data } => view! { <DashboardPage data=data lang=lang.clone() /> }.into_any(),
                Route::Settings => view! { <SettingsPage lang=lang.clone() /> }.into_any(),
                Route::NotFound => view! {
                    <main>
                        <h1>{i18n::t(&lang_for_not_found, "not-found-title")}</h1>
                        <p><a href="/" class="muted">{i18n::t(&lang_for_not_found, "back-to-top")}</a></p>
                    </main>
                }.into_any(),
            }}
        </div>
    }
}

// ────────────────────────────────────────────
// Hydrate エントリポイント (ブラウザ実行時のみ)
// ────────────────────────────────────────────

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn hydrate_main() {
    console_error_panic_hook::set_once();

    // SSR 時に注入された JSON / 言語コードを復元する。
    let route = read_route_from_window().unwrap_or(Route::NotFound);
    let lang = read_lang_from_window().unwrap_or_else(|| "en".to_string());

    leptos::mount::hydrate_body(move || view! { <App route=route.clone() lang=lang.clone() /> });
}

#[cfg(feature = "hydrate")]
fn read_lang_from_window() -> Option<String> {
    let window = web_sys::window()?;
    let key = wasm_bindgen::JsValue::from_str("__DIARILOG_LANG__");
    let raw = js_sys::Reflect::get(&window, &key).ok()?;
    raw.as_string()
}

#[cfg(feature = "hydrate")]
fn read_route_from_window() -> Option<Route> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window()?;
    let key = wasm_bindgen::JsValue::from_str("__DIARILOG_ROUTE__");
    let raw = js_sys::Reflect::get(&window, &key).ok()?;
    let json = raw.as_string()?;
    parse_route_json(&json)
}

#[cfg(feature = "hydrate")]
fn parse_route_json(json: &str) -> Option<Route> {
    // 完全 serde_json でデシリアライズすると WASM サイズが膨らむため、
    // 軽量な JSON 解釈に留める。Dashboard データだけは `__DIARILOG_DATA__` 経由で
    // 別途読み出す。
    let s = json.trim();
    if s.contains(r#""kind":"index""#) {
        Some(Route::Index)
    } else if s.contains(r#""kind":"onboarding""#) {
        Some(Route::Onboarding)
    } else if s.contains(r#""kind":"dashboard""#) {
        Some(Route::Dashboard {
            data: read_dashboard_data_from_window(),
        })
    } else if s.contains(r#""kind":"login""#) {
        let issuer = s
            .split(r#""issuer":""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or("")
            .to_string();
        Some(Route::Login { issuer })
    } else if s.contains(r#""kind":"settings""#) {
        Some(Route::Settings)
    } else {
        Some(Route::NotFound)
    }
}

/// `window.__DIARILOG_DATA__` に SSR 時に埋め込まれた Dashboard データの JSON 文字列を読み、
/// `DashboardResponse` にデシリアライズする。失敗したら `None`。
#[cfg(feature = "hydrate")]
fn read_dashboard_data_from_window() -> Option<DashboardResponse> {
    let window = web_sys::window()?;
    let key = wasm_bindgen::JsValue::from_str("__DIARILOG_DATA__");
    let raw = js_sys::Reflect::get(&window, &key).ok()?;
    let json = raw.as_string()?;
    serde_json::from_str::<DashboardResponse>(&json).ok()
}

// ────────────────────────────────────────────
// SSR 用 helper (サーバー側でしか呼ばない)
// ────────────────────────────────────────────

/// SSR 側で生成する Route 復元用の JSON 文字列。
/// HTML に埋め込んでハイドレーション時にブラウザが読み取る。
///
/// Dashboard のデータ部分はこの JSON には含めず、`__DIARILOG_DATA__` として別変数で
/// 埋め込む方針 (Dashboard 以外のルートでは無駄なシリアライゼーションを避ける)。
pub fn route_to_json(route: &Route) -> String {
    match route {
        Route::Index => r#"{"kind":"index"}"#.to_string(),
        Route::Onboarding => r#"{"kind":"onboarding"}"#.to_string(),
        Route::Dashboard { .. } => r#"{"kind":"dashboard"}"#.to_string(),
        Route::Login { issuer } => {
            let safe = issuer.replace('"', r#"\""#);
            format!(r#"{{"kind":"login","issuer":"{}"}}"#, safe)
        }
        Route::Settings => r#"{"kind":"settings"}"#.to_string(),
        Route::NotFound => r#"{"kind":"not_found"}"#.to_string(),
    }
}

/// SSR 側で生成する Dashboard データの JSON 文字列。
/// 結果は `<script>window.__DIARILOG_DATA__ = ...</script>` の形で埋め込まれ、
/// ブラウザ側で hydrate 時に読み取られる。
///
/// Dashboard 以外のルートでは `None` を返し、bff 側で埋め込みをスキップさせる。
pub fn dashboard_data_to_json(route: &Route) -> Option<String> {
    match route {
        Route::Dashboard { data: Some(data) } => serde_json::to_string(data).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_to_json_index() {
        assert_eq!(route_to_json(&Route::Index), r#"{"kind":"index"}"#);
    }

    #[test]
    fn route_to_json_dashboard() {
        assert_eq!(
            route_to_json(&Route::Dashboard { data: None }),
            r#"{"kind":"dashboard"}"#
        );
    }

    #[test]
    fn route_to_json_login() {
        let r = Route::Login {
            issuer: "https://example.com".to_string(),
        };
        assert_eq!(
            route_to_json(&r),
            r#"{"kind":"login","issuer":"https://example.com"}"#
        );
    }

    #[test]
    fn route_to_json_login_escapes_quotes() {
        let r = Route::Login {
            issuer: r#"weird"issuer"#.to_string(),
        };
        let json = route_to_json(&r);
        assert!(json.contains(r#"\"issuer"#));
    }

    #[test]
    fn dashboard_data_to_json_none_when_no_data() {
        let r = Route::Dashboard { data: None };
        assert!(dashboard_data_to_json(&r).is_none());
    }

    #[test]
    fn dashboard_data_to_json_serializes_data() {
        use contracts::bff::{DashboardResponse, DashboardStatus};
        let r = Route::Dashboard {
            data: Some(DashboardResponse {
                user: None,
                recent_diaries: Vec::new(),
                active_session: None,
                status: DashboardStatus::default(),
            }),
        };
        let json = dashboard_data_to_json(&r).expect("should serialize");
        assert!(json.contains(r#""user":null"#));
        assert!(json.contains(r#""user_ok":false"#));
    }

    #[test]
    fn dashboard_data_to_json_none_for_other_routes() {
        assert!(dashboard_data_to_json(&Route::Index).is_none());
        assert!(dashboard_data_to_json(&Route::NotFound).is_none());
        assert!(
            dashboard_data_to_json(&Route::Login {
                issuer: "x".to_string()
            })
            .is_none()
        );
    }

    #[test]
    fn route_to_json_settings() {
        assert_eq!(route_to_json(&Route::Settings), r#"{"kind":"settings"}"#);
    }

    #[test]
    fn route_to_json_not_found() {
        assert_eq!(route_to_json(&Route::NotFound), r#"{"kind":"not_found"}"#);
    }

    #[test]
    fn route_to_json_onboarding() {
        assert_eq!(
            route_to_json(&Route::Onboarding),
            r#"{"kind":"onboarding"}"#
        );
    }
}
