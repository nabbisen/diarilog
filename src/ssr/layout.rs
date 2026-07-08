//! HTML document wrapper — `<html>`, `<head>`, `<body>`, shared CSS, hydration scripts.

const CSS: &str = r#"
:root {
  --bg:     #fafafa;
  --fg:     #1f1f1f;
  --muted:  #6b7280;
  --accent: #5a7d9a;
  --border: #e5e7eb;
  --danger: #c0392b;
}
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI",
               "Hiragino Kaku Gothic ProN", sans-serif;
  color: var(--fg);
  background: var(--bg);
  line-height: 1.6;
  font-size: 16px;
}
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }

/* Site header — present on every page */
.site-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1.5rem;
  border-bottom: 1px solid var(--border);
}
.site-header .brand {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--fg);
  letter-spacing: -0.01em;
}
.site-header .crisis-link {
  font-size: 0.8rem;
  color: var(--muted);
  border: 1px solid var(--border);
  padding: 0.2rem 0.55rem;
  border-radius: 4px;
}
.site-header .crisis-link:hover { color: var(--fg); text-decoration: none; }

/* Page content */
main {
  max-width: 600px;
  margin: 0 auto;
  padding: 2.5rem 1.5rem;
}
h1 { font-size: 1.25rem; font-weight: 500; margin-bottom: 1rem; }
h2 { font-size: 1rem;    font-weight: 500; margin-bottom: 0.5rem; }
p  { margin: 0.5rem 0; }
.muted { color: var(--muted); font-size: 0.9rem; }

/* Primary action button */
.btn {
  display: inline-block;
  padding: 0.65rem 1.4rem;
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  font-size: 0.95rem;
  cursor: pointer;
}
.btn:hover { opacity: 0.9; text-decoration: none; color: #fff; }
.btn--danger { background: var(--danger); }
.btn:disabled, .btn[disabled] { opacity: 0.4; cursor: not-allowed; }

/* Entry list on dashboard */
.entry-list { list-style: none; margin-top: 0.5rem; }
.entry-list li {
  padding: 0.45rem 0;
  border-bottom: 1px solid var(--border);
  font-size: 0.9rem;
}
.entry-list li:last-child { border-bottom: none; }

/* Forms (onboarding, settings) */
.form-group { margin: 1rem 0; }
.form-group label { display: block; font-size: 0.9rem; margin-bottom: 0.3rem; }
.form-group input[type="password"],
.form-group input[type="text"] {
  width: 100%;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border);
  border-radius: 5px;
  font-size: 1rem;
  background: #fff;
}
.form-group input:focus {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
  border-color: transparent;
}
.form-group--checkbox { display: flex; align-items: flex-start; gap: 0.5rem; }
.form-group--checkbox input { margin-top: 0.25rem; flex-shrink: 0; }
.erase-confirm-input { font-family: monospace; }
.passphrase-strength { font-size: 0.8rem; margin-top: 0.2rem; }
.hidden { display: none; }
"#;

pub struct HydrationConfig {
    pub assets_base_url: String,
    pub route_json: String,
    pub data_json: Option<String>,
    pub lang: String,
}

pub fn wrap_document(
    title: &str,
    body_html: &str,
    lang: &str,
    dir: &str,
    hydration: Option<&HydrationConfig>,
) -> String {
    let crisis_label = if lang == "ja" { "危機支援" } else { "Crisis help" };
    let t = html_escape(title);
    let l = html_escape(lang);
    let d = html_escape(dir);

    let hydration_html = match hydration {
        Some(cfg) if !cfg.assets_base_url.trim().is_empty() => {
            let base        = cfg.assets_base_url.trim_end_matches('/');
            let safe_route  = cfg.route_json.replace("</", r"<\/");
            let safe_lang   = cfg.lang.replace("</", r"<\/");
            let data_script = match &cfg.data_json {
                Some(j) => format!(
                    "<script>window.__DIARILOG_DATA__={};</script>\n",
                    js_string(&j.replace("</", r"<\/"))
                ),
                None => String::new(),
            };
            format!(
                "<script>window.__DIARILOG_ROUTE__={};</script>\n\
                 <script>window.__DIARILOG_LANG__={};</script>\n\
                 {}<script type=\"module\" src=\"{}/web-app.js\"></script>",
                js_string(&safe_route),
                js_string(&safe_lang),
                data_script,
                html_attr(base),
            )
        }
        _ => String::new(),
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="{l}" dir="{d}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{t}</title>
<style>{CSS}</style>
</head>
<body>
<header class="site-header">
  <a href="/" class="brand">diarilog</a>
  <a href="/crisis-help" class="crisis-link">{crisis_label}</a>
</header>
{body_html}
{hydration_html}
</body>
</html>"##
    )
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

fn html_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}

fn js_string(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"'  => o.push_str(r#"\""#),
            '\\' => o.push_str(r"\\"),
            '\n' => o.push_str(r"\n"),
            '\r' => o.push_str(r"\r"),
            '\t' => o.push_str(r"\t"),
            '<'  => o.push_str(r"\u003c"),
            '>'  => o.push_str(r"\u003e"),
            '&'  => o.push_str(r"\u0026"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_has_title_body_and_header() {
        let doc = wrap_document("Hello", "<p>world</p>", "en", "ltr", None);
        assert!(doc.contains("<title>Hello</title>"));
        assert!(doc.contains("<p>world</p>"));
        assert!(doc.contains(r#"lang="en""#));
        assert!(doc.contains("site-header"));
        assert!(doc.contains("Crisis help"));
    }

    #[test]
    fn japanese_crisis_label() {
        let doc = wrap_document("X", "", "ja", "ltr", None);
        assert!(doc.contains("危機支援"));
    }

    #[test]
    fn title_is_escaped() {
        let doc = wrap_document("<b>Bad</b>", "", "en", "ltr", None);
        assert!(doc.contains("&lt;b&gt;Bad&lt;/b&gt;"));
    }

    #[test]
    fn no_hydration_when_empty_url() {
        let cfg = HydrationConfig {
            assets_base_url: "".into(),
            route_json: r#"{"kind":"index"}"#.into(),
            data_json: None,
            lang: "en".into(),
        };
        let doc = wrap_document("X", "", "en", "ltr", Some(&cfg));
        assert!(!doc.contains("__DIARILOG_ROUTE__"));
    }

    #[test]
    fn hydration_injects_scripts() {
        let cfg = HydrationConfig {
            assets_base_url: "https://cdn.example.com".into(),
            route_json: r#"{"kind":"index"}"#.into(),
            data_json: None,
            lang: "en".into(),
        };
        let doc = wrap_document("X", "", "en", "ltr", Some(&cfg));
        assert!(doc.contains("__DIARILOG_ROUTE__"));
        assert!(doc.contains("__DIARILOG_LANG__"));
        assert!(doc.contains("web-app.js"));
    }

    #[test]
    fn hydration_escapes_close_tag() {
        let cfg = HydrationConfig {
            assets_base_url: "https://cdn.example.com".into(),
            route_json: r#"{"x":"</script>"}"#.into(),
            data_json: None,
            lang: "en".into(),
        };
        let doc = wrap_document("X", "", "en", "ltr", Some(&cfg));
        assert!(!doc.contains("</script>\""));
    }
}
