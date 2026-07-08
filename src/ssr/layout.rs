//! HTML ドキュメント全体のラッパ (`<html><head><body>` + 共通 CSS + Hydration scripts)。
//!
//! Leptos の `view!` マクロで生成された body 内 HTML を受け取り、
//! 完全な HTML ドキュメントとして組み立てる。
//!
//! ハイドレーション用スクリプトタグも本モジュールから注入する。
//! 環境変数 `WEB_ASSETS_BASE_URL` (例: `https://assets.example.com`) が設定されていれば、
//! `<script type="module" src="{base}/web-app.js">` を `</body>` 直前に注入する。

const BASE_CSS: &str = r#"
:root {
  --bg: #fafafa;
  --fg: #1f1f1f;
  --muted: #6b6b6b;
  --accent: #5a7d9a;
  --border: #e1e1e1;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Hiragino Kaku Gothic ProN", sans-serif;
  color: var(--fg);
  background: var(--bg);
  line-height: 1.6;
}
main {
  max-width: 760px;
  margin: 0 auto;
  padding: 2rem 1rem;
}
h1 { font-weight: 500; margin-top: 0; }
.muted { color: var(--muted); }
.btn {
  display: inline-block;
  padding: 0.6rem 1.2rem;
  border: 1px solid var(--accent);
  border-radius: 6px;
  background: var(--accent);
  color: #fff;
  text-decoration: none;
}
.btn:hover { opacity: 0.9; }
.notice {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 1rem;
  background: #fff;
  margin-top: 1rem;
}
"#;

/// ハイドレーション設定。
pub struct HydrationConfig {
    /// CSR バンドルの配信ベース URL (例: "https://assets.example.com")。
    /// 空文字列の場合はハイドレーションスクリプトを注入しない (= サーバーレンダリングのみ)。
    pub assets_base_url: String,
    /// SSR 側で決定した Route の JSON 表現。
    /// ブラウザ側で `window.__DIARILOG_ROUTE__` から読み取る。
    pub route_json: String,
    /// SSR 側で集約 API から取得した Dashboard データの JSON 表現。
    /// `Some(json)` の場合 `window.__DIARILOG_DATA__` に埋め込む。
    /// Dashboard 以外のルートでは `None` で OK (空のまま)。
    pub data_json: Option<String>,
    /// SSR 側で決定した表示言語コード ("ja" / "en" 等)。
    /// `window.__DIARILOG_LANG__` に埋め込み、ブラウザ側 hydrate で同じ値を使う。
    pub lang: String,
}

/// Leptos `view!` でレンダリング済みの body HTML を `<html>` ドキュメントでラップする。
///
/// `lang` と `dir` は SSR 側で決定したものをそのまま埋め込む。
/// ハイドレーション設定があれば、`__DIARILOG_ROUTE__` / `__DIARILOG_DATA__` / `__DIARILOG_LANG__` を
/// 注入する。
pub fn wrap_document(
    title: &str,
    body_html: &str,
    lang: &str,
    dir: &str,
    hydration: Option<&HydrationConfig>,
) -> String {
    let title = html_escape(title);
    let lang_attr = html_escape(lang);
    let dir_attr = html_escape(dir);

    let hydration_html = match hydration {
        Some(cfg) if !cfg.assets_base_url.trim().is_empty() => {
            let base = cfg.assets_base_url.trim_end_matches('/');
            // route JSON は `<script>` の中で window.__DIARILOG_ROUTE__ に文字列として設定する。
            // JSON 内の `</` は `<\/` にエスケープして XSS を防ぐ。
            let safe_route = cfg.route_json.replace("</", r#"<\/"#);

            // data JSON も同様にエスケープして window.__DIARILOG_DATA__ に設定する。
            // 設定されていなければ空文字列 (script 自体は生成しない)。
            let data_script = match &cfg.data_json {
                Some(json) => {
                    let safe = json.replace("</", r#"<\/"#);
                    format!(
                        "<script>window.__DIARILOG_DATA__={};</script>\n",
                        serialize_string_literal(&safe)
                    )
                }
                None => String::new(),
            };

            // lang script
            let safe_lang = cfg.lang.replace("</", r#"<\/"#);
            let lang_script = format!(
                "<script>window.__DIARILOG_LANG__={};</script>\n",
                serialize_string_literal(&safe_lang)
            );

            format!(
                r#"<script>window.__DIARILOG_ROUTE__={};</script>
{}{}<script type="module" src="{}/web-app.js"></script>"#,
                serialize_string_literal(&safe_route),
                lang_script,
                data_script,
                html_escape_attr(base)
            )
        }
        _ => String::new(),
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="{lang_attr}" dir="{dir_attr}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>{BASE_CSS}</style>
</head>
<body>
{body_html}
{hydration_html}
</body>
</html>"##
    )
}

/// HTML エスケープ (テキストノード / 属性向け簡易版)。
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// 属性値向けの軽量エスケープ (URL を入れる用、`<>"&` のみ)。
fn html_escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// JS 文字列リテラルとして安全な形式に変換 (`"..."` 形式の出力)。
fn serialize_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str(r#"\""#),
            '\\' => out.push_str(r"\\"),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            '<' => out.push_str(r"\u003c"),
            '>' => out.push_str(r"\u003e"),
            '&' => out.push_str(r"\u0026"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_basic() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a&b"), "a&amp;b");
        assert_eq!(html_escape("\"q\""), "&quot;q&quot;");
    }

    #[test]
    fn document_contains_title_and_body() {
        let doc = wrap_document("Hello", "<p>world</p>", "ja", "ltr", None);
        assert!(doc.contains("<title>Hello</title>"));
        assert!(doc.contains("<p>world</p>"));
        assert!(doc.contains(r#"lang="ja""#));
    }

    #[test]
    fn document_escapes_title() {
        let doc = wrap_document("<x>", "", "en", "ltr", None);
        assert!(doc.contains("&lt;x&gt;"));
    }

    #[test]
    fn no_hydration_when_assets_url_empty() {
        let cfg = HydrationConfig {
            assets_base_url: "".to_string(),
            route_json: r#"{"kind":"index"}"#.to_string(),
            data_json: None,
            lang: "en".to_string(),
        };
        let doc = wrap_document("X", "<p/>", "en", "ltr", Some(&cfg));
        assert!(!doc.contains("__DIARILOG_ROUTE__"));
        assert!(!doc.contains("web-app.js"));
    }

    #[test]
    fn hydration_script_injected_when_url_set() {
        let cfg = HydrationConfig {
            assets_base_url: "https://assets.example.com".to_string(),
            route_json: r#"{"kind":"index"}"#.to_string(),
            data_json: None,
            lang: "en".to_string(),
        };
        let doc = wrap_document("X", "<p/>", "en", "ltr", Some(&cfg));
        assert!(doc.contains("__DIARILOG_ROUTE__"));
        assert!(doc.contains("https://assets.example.com/web-app.js"));
    }

    #[test]
    fn hydration_script_escapes_close_tag() {
        // </script> がそのまま埋め込まれると XSS になりうる
        let cfg = HydrationConfig {
            assets_base_url: "https://assets.example.com".to_string(),
            route_json: r#"{"x":"</script>"}"#.to_string(),
            data_json: None,
            lang: "en".to_string(),
        };
        let doc = wrap_document("X", "", "en", "ltr", Some(&cfg));
        assert!(!doc.contains("</script>\"")); // 閉じタグそのままは出ない
        assert!(doc.contains(r"\u003c") || doc.contains(r"<\/"));
    }

    #[test]
    fn hydration_data_script_injected_when_data_present() {
        let cfg = HydrationConfig {
            assets_base_url: "https://assets.example.com".to_string(),
            route_json: r#"{"kind":"dashboard"}"#.to_string(),
            data_json: Some(r#"{"user":null,"recent_diaries":[]}"#.to_string()),
            lang: "en".to_string(),
        };
        let doc = wrap_document("X", "<p/>", "en", "ltr", Some(&cfg));
        assert!(doc.contains("__DIARILOG_ROUTE__"));
        assert!(doc.contains("__DIARILOG_DATA__"));
    }

    #[test]
    fn hydration_data_script_omitted_when_data_absent() {
        let cfg = HydrationConfig {
            assets_base_url: "https://assets.example.com".to_string(),
            route_json: r#"{"kind":"index"}"#.to_string(),
            data_json: None,
            lang: "en".to_string(),
        };
        let doc = wrap_document("X", "<p/>", "en", "ltr", Some(&cfg));
        assert!(doc.contains("__DIARILOG_ROUTE__"));
        assert!(!doc.contains("__DIARILOG_DATA__"));
    }

    #[test]
    fn hydration_lang_script_injected() {
        let cfg = HydrationConfig {
            assets_base_url: "https://assets.example.com".to_string(),
            route_json: r#"{"kind":"index"}"#.to_string(),
            data_json: None,
            lang: "ja".to_string(),
        };
        let doc = wrap_document("X", "<p/>", "ja", "ltr", Some(&cfg));
        assert!(doc.contains("__DIARILOG_LANG__"));
        assert!(doc.contains(r#""ja""#));
    }

    #[test]
    fn html_dir_attribute_set_for_rtl() {
        let cfg = HydrationConfig {
            assets_base_url: "".to_string(),
            route_json: r#"{"kind":"index"}"#.to_string(),
            data_json: None,
            lang: "ar".to_string(),
        };
        let doc = wrap_document("X", "<p/>", "ar", "rtl", Some(&cfg));
        assert!(doc.contains(r#"lang="ar""#));
        assert!(doc.contains(r#"dir="rtl""#));
    }
}
