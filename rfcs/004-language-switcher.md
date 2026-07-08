# RFC 004: Language switcher UI and `PUT /api/me`

| Field | Value |
|---|---|
| Status | Proposed |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |
| Template | Standard + Security |

## Summary

Let users explicitly choose their display language, persist that choice (server-side for authenticated users, client-side for anonymous users), and propagate it across all subsequent renders. Adds a small switcher UI in the page header and the `PUT /api/me` endpoint that backs the persistence.

## Motivation

The current language resolution chain (RFC 01-precursor v2.4) goes UserRecord → cookie → Accept-Language → `en`. Steps 1 and 2 are present in the chain but no UI sets them, so in practice users are stuck with whatever Accept-Language their browser sends. A user whose browser is configured for English but who would rather read Japanese has no way to switch.

## Requirements

1. **R1 (functional, must)** — A language switcher control is reachable from every primary page (Index, Login, Dashboard).
2. **R2 (functional, must)** — Selecting a different language updates the visible language immediately (next render at the latest) and persists across page navigations and sessions.
3. **R3 (functional, must)** — For anonymous users, the choice persists in a cookie; for authenticated users, the choice persists in `UserRecord.language` via `PUT /api/me`.
4. **R4 (functional, must)** — The switcher offers all 5 `SUPPORTED_LANGUAGES`, not just `TRANSLATED_LANGUAGES`. Choosing an untranslated language results in English fallback rendering plus the provisional banner from RFC 003.
5. **R5 (functional, should)** — When a user is authenticated, their `UserRecord.language` takes precedence over the cookie. After login, the cookie should be reconciled to the server value.
6. **R6 (non-functional, must)** — The switcher must be operable by keyboard alone and announce its current value to screen readers.
7. **R7 (security, must)** — `PUT /api/me` is authenticated, validates the language code against `SUPPORTED_LANGUAGES`, and rejects anything else with 400.
8. **R8 (security, must)** — The cookie is `HttpOnly=false` (the SSR layer needs to read it via `Cookie` header but the client also needs to be able to write it before login completes), `SameSite=Lax`, and **does not** carry credentials. It only carries a 2-letter language code, so XSS exposure of the cookie does not leak anything sensitive.

## Design

### UI

A new component `pages/components/lang_switcher.rs`, exposing `LangSwitcher(current: String)`. Visual: a `<select>` (no JS frameworks, just the browser native control) with an `<option>` per supported language, the value being the language code, the label being the language's endonym (`日本語`, `English`, `العربية`, `Українська`, `Español`).

The component is mounted in the page header (a new shared `<header>` block in `BASE_CSS` and the layout) so that it appears once across the site rather than being duplicated per page.

The form submits as a normal HTML form to `POST /api/lang` (a small thin endpoint on bff that does not require Leptos state). Why a form rather than a JS handler: the language switcher must work before hydration completes (the user might switch language while the page is still SSR-only). Native form submission also gives us free no-JS support.

### Endpoint shapes

**`POST /api/lang`** (bff-worker)

- Auth: optional (works for anonymous users)
- Body: form-encoded `lang=<code>` (no JSON, since the form posts directly)
- Validation: `lang` must be in `SUPPORTED_LANGUAGES`. Otherwise 400.
- Effects:
  - Sets a `Set-Cookie: diarilog_lang=<code>; Path=/; Max-Age=31536000; SameSite=Lax` header.
  - If authenticated (`X-User-Id` header is present), additionally calls `IDENTITY` service binding to update `UserRecord.language`.
- Response: 303 redirect back to the `Referer` header (or `/` if missing). 303 ensures the browser issues a GET, not a repeated POST, after the redirect.

**`PUT /api/me`** (gateway → identity)

- Auth: required (gateway rejects unauthenticated requests as it does for `/api/dashboard`).
- Body: JSON `{"language": "ja"}` (extensible to other profile fields later).
- Validation: same as above for `language`.
- Effect: persists to D1 via identity-worker.
- Response: `200 { "user": <UserRecord> }`.

`PUT /api/me` is the lower-level endpoint and `POST /api/lang` is the form-friendly wrapper that uses it. An authenticated user pressing the switcher hits `POST /api/lang`, which sets the cookie immediately and fires-and-forgets a `PUT /api/me` internally. (Fire-and-forget is acceptable because the cookie already reflects the new state; the D1 update is for cross-device persistence.)

### Resolver update

`workers/bff/src/ssr/handlers.rs::resolve_lang` is extended to read the cookie:

```
1. UserRecord.language  (from /api/dashboard.user when present)
2. cookie diarilog_lang
3. Accept-Language
4. "en"
```

A small helper `read_cookie(req, "diarilog_lang") -> Option<String>` is added to a new `workers/bff/src/cookies.rs`. The cookie value must be validated against `SUPPORTED_LANGUAGES` before being used; an attacker-controlled cookie containing junk should fall through to step 3.

### Reconciliation after login

When a user logs in and their `UserRecord.language` differs from the current cookie, the `POST /api/lang` flow above is invoked from the post-login handler with the user's stored language. This makes step 1 of the resolver chain effective on the next request without a special code path elsewhere.

## Test plan

- **Unit (bff cookies)** — Tests for `read_cookie` covering: header present with the key, header present without the key, header absent, multiple cookies separated by `; `, value with whitespace.
- **Unit (bff resolve_lang)** — The current `resolve_lang` is not unit-testable because it takes `worker::Request`. Refactor it to take a `&str` for `Accept-Language` and an `Option<&str>` for the cookie, plus tests for each precedence rule.
- **Unit (contracts validation)** — A small `is_supported_language(&str) -> bool` helper added to `packages/web-app/src/i18n.rs` and tested against all 5 codes plus `"xx"` and `""`.
- **Unit (identity)** — Test `update_user_language(user_id, lang)` for valid and invalid language codes.
- **Manual (deferred)** — End-to-end through the form submission. Same level of manual coverage as RFC 003.

Expected new test count: **+8 unit tests** (~82 total after RFC 003 + 0004).

## Security considerations

- **Validation at every boundary** — Both `POST /api/lang` and `PUT /api/me` reject unknown language codes. The cookie reader also re-validates on read.
- **Cookie scope** — Path `/`, no `Domain` attribute (defaults to current host), `SameSite=Lax`. Not `Secure` only because dev environments use HTTP; the production wrangler config should override to `Secure`.
- **CSRF on `POST /api/lang`** — The form is on the same origin, and the action only changes a language preference (no privilege escalation, no destructive operation, no sensitive disclosure). Adding a CSRF token here is overkill. The `PUT /api/me` route, which carries a bearer token already, is the privileged path; an attacker forging a cookie at most makes the victim see a different language.
- **`PUT /api/me` rate limiting** — A user repeatedly toggling language is harmless, but worth a soft rate limit (10/min) at the gateway to keep abuse off identity-worker. Implementation can reuse whatever rate-limiting story we settle on for other write endpoints; not blocking this RFC.

## Out of scope

- Persisting language preference encrypted (it is not sensitive enough).
- A full settings page. The switcher in the header is enough for now; a dedicated settings page can come with the next batch of profile fields.
- Detecting the user's preferred language from system locale on mobile devices. This is a PWA topic, deferred to RFC 009.

## Open questions

- Should the switcher show only `TRANSLATED_LANGUAGES` (currently `ja`, `en`), or all `SUPPORTED_LANGUAGES`? The requirement above says "all 5", reasoning: the user may consciously want to be fallen back rather than be denied the option entirely. Open to swapping if the trauma-care framing argues otherwise (e.g. offering Arabic and then showing English content might be more disorienting than not offering it).
- The endonyms above include three scripts. The font stack must include glyphs for all of them; this overlaps with RFC 006 (RTL + Arabic font).
