use crate::storage::identity::UserStorage;
use auth_core::{OidcConfig, verify_id_token};
use serde::Deserialize;
use worker::*;

/// GET /auth/callback
pub async fn callback(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let env = &ctx.env;
    let url = req.url()?;
    let params: std::collections::HashMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let code = match params.get("code") {
        Some(c) => c.clone(),
        None => {
            let error = params.get("error").map(String::as_str).unwrap_or("unknown");
            let headers = Headers::new();
            headers.set("Location", &format!("/login?error={}", error))?;
            return Ok(Response::empty()?.with_status(302).with_headers(headers));
        }
    };

    let config = OidcConfig::from_env(env)?;
    let id_token = exchange_code(env, &config, &code).await?;
    let claims = verify_id_token(env, &id_token, &config).await?;

    UserStorage::ensure(env, &claims.subject, &claims.email).await?;

    let user = UserStorage::get(env, &claims.subject).await?;
    let dest = if user.map(|u| u.onboarding_completed).unwrap_or(false) {
        "/dashboard"
    } else {
        "/onboarding"
    };

    let headers = Headers::new();
    headers.set(
        "Set-Cookie",
        &format!("session={}; Path=/; HttpOnly; SameSite=Lax", id_token),
    )?;
    headers.set("Location", dest)?;
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

/// POST /api/auth/verify-turnstile
pub async fn verify_turnstile(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct Body {
        token: String,
    }
    #[derive(Deserialize)]
    struct TResult {
        success: bool,
        #[serde(rename = "error-codes")]
        error_codes: Option<Vec<String>>,
    }

    let body: Body = req
        .json()
        .await
        .map_err(|_| Error::RustError("Bad body".into()))?;
    let secret = ctx
        .env
        .secret("TURNSTILE_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let ip = req.headers().get("CF-Connecting-IP")?.unwrap_or_default();

    let form = format!(
        "secret={}&response={}&remoteip={}",
        ue(&secret),
        ue(&body.token),
        ue(&ip)
    );
    let headers = Headers::new();
    headers.set("Content-Type", "application/x-www-form-urlencoded")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(form.into()));
    let treq = Request::new_with_init(
        "https://challenges.cloudflare.com/turnstile/v0/siteverify",
        &init,
    )?;
    let result: TResult = Fetch::Request(treq).send().await?.json().await?;

    if result.success {
        Response::from_json(&serde_json::json!({ "verified": true }))
    } else {
        Response::error(result.error_codes.unwrap_or_default().join(", "), 403)
    }
}

async fn exchange_code(env: &Env, config: &OidcConfig, code: &str) -> Result<String> {
    let client_id = env
        .var("OIDC_AUDIENCE")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let client_secret = env
        .secret("OIDC_CLIENT_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let redirect_uri = env
        .var("OIDC_REDIRECT_URI")
        .map(|v| v.to_string())
        .unwrap_or_default();

    let form = format!(
        "grant_type=authorization_code&code={}&client_id={}&client_secret={}&redirect_uri={}",
        ue(code),
        ue(&client_id),
        ue(&client_secret),
        ue(&redirect_uri)
    );
    // Fetch token endpoint URL from OIDC discovery
    let disc_url = format!("{}/.well-known/openid-configuration", config.issuer);
    let disc: serde_json::Value = Fetch::Url(disc_url.parse()?).send().await?.json().await?;
    let token_ep = disc["token_endpoint"]
        .as_str()
        .ok_or_else(|| Error::RustError("Missing token_endpoint".into()))?;

    let headers = Headers::new();
    headers.set("Content-Type", "application/x-www-form-urlencoded")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(form.into()));
    let treq = Request::new_with_init(token_ep, &init)?;
    let tok: serde_json::Value = Fetch::Request(treq).send().await?.json().await?;
    tok["id_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| Error::RustError("No id_token in response".into()))
}

fn ue(s: &str) -> String {
    s.replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
        .replace(' ', "%20")
}
