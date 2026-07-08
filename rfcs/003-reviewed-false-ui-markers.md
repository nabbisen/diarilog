# RFC 003: UI markers for unreviewed translations

| Field | Value |
|---|---|
| Status | Proposed |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |
| Template | Standard |

## Summary

Surface the `reviewed: false` flag from `CrisisResources`, `HotlineInfo`, and dialog prompts to the user interface, so people viewing provisional translations (currently `ar`, `uk`, `es`) understand that the text has not yet been reviewed by a clinical expert and that they should fall back to the always-reviewed IASP international hotline if anything looks wrong.

## Motivation

After RFC 001-precursor work (v0.6), the backend already tracks which strings are expert-reviewed. From the user's point of view the situation is currently invisible: the dashboard renders provisional translations identically to reviewed ones. For a trauma-care service, this is a real safety gap. A user in crisis might dial a hotline number that has not been verified by a native-speaking clinician. We already mitigate this by always including the IASP fallback (RFC-equivalent test: `crisis_resources_always_include_international_fallback`), but the user has no way to know which entry to trust.

## Requirements

1. **R1 (functional, must)** — When the dashboard or any other page renders content sourced from a provisional translation, the user can see at a glance that the content is provisional.
2. **R2 (functional, must)** — When a list of crisis hotlines is shown, each entry indicates individually whether it is reviewed. A reviewed entry must not inherit a "provisional" appearance just because its sibling is provisional.
3. **R3 (functional, should)** — The user can follow a link from the marker to a short explanation of what "provisional translation" means and why the IASP fallback is the recommended choice.
4. **R4 (non-functional, must)** — The marker is recognizable to users with limited literacy and to screen-reader users. Color alone is not the carrier of meaning.
5. **R5 (non-functional, must)** — The marker does not stigmatize the language. Phrasing should be neutral ("translation pending review"), not deficit-framed ("incomplete", "unsafe").
6. **R6 (non-functional, must)** — No additional network round-trip or API surface is required at runtime; the existing aggregation API and the existing per-hotline `reviewed` field are enough.

## Design

### Data flow (already in place)

`workers/safety/src/classifier.rs::crisis_resources(lang)` already returns `CrisisResources { message_reviewed, hotlines: [HotlineInfo { reviewed, ... }, ...] }`. The dashboard SSR pulls this through the aggregation API; no change needed on the backend.

### Frontend rendering

Three new pieces in `packages/web-app`:

1. **A small badge component** — `pages/components/review_status.rs`, exposing `ReviewBadge(reviewed: bool, lang: String)`. When `reviewed == true`, returns `view! {}` (renders nothing). When `false`, renders an inline `<span class="badge badge--pending" role="status" aria-label="...">Pending review</span>`. The label string comes from `i18n::t(&lang, "review-status-pending-label")`.
2. **A help link target** — A new static page `/help/translation-review` served by bff-worker, with content sourced from `i18n` keys (`review-help-title`, `review-help-body-1` ...). The page should explain the review flow in plain language and link to `docs/i18n-review-flow.md` for technical readers.
3. **CSS** — A new `.badge--pending` class added to `BASE_CSS` in `workers/bff/src/ssr/layout.rs`. Visual style: small, muted-yellow background, text-color contrast meeting WCAG AA against the surrounding surface, with a leading symbol (e.g. `⚠` or text "[provisional]") so the meaning survives if CSS fails to load.

### Granularity rules

- **Per-string for crisis content** — Each `HotlineInfo` carries its own badge based on `reviewed`. The aggregated `CrisisResources.message_reviewed` controls the badge on the message text only.
- **Per-page banner for general UI text** — When the resolved language is in `SUPPORTED_LANGUAGES` but not in `TRANSLATED_LANGUAGES` (the user has been silently fallen back to English), the page displays a one-line banner at the top: "This page is shown in English while the {language_name} translation is being reviewed. Some text may be unfamiliar." Linked to the same `/help/translation-review` page.

### i18n keys to add

Add to `packages/web-app/locales/{ja,en}/main.ftl`:

```
review-status-pending-label = Translation pending expert review
review-status-pending-help-link = Learn more
review-help-title = About translation review
review-help-body-1 = ...
review-help-body-2 = ...
fallback-banner = This page is shown in English while the { $lang_name } translation is being reviewed.
```

Provisional translations for `ar`, `uk`, `es` are out of scope here — they will be added through the regular translation review flow (RFC 005) and are not blockers for landing this RFC.

### Where the badge appears in v0.7

For the first iteration:

- Dashboard `Recent journals` list — no badges (entries are user content, not platform translations).
- Dashboard `Active session` link — no badge.
- Crisis resources view (when shown) — badges per hotline plus, when applicable, a badge on the introductory message.
- Login page issuer banner — no badge.

This is intentionally narrow. We can extend later; the goal of the first cut is to cover the highest-stakes surface.

## Test plan

- **Unit (web-app)** — Add tests for `ReviewBadge` covering both `reviewed=true` (renders empty) and `reviewed=false` (renders span with the right `aria-label`).
- **Unit (web-app i18n)** — Verify the new keys resolve in `ja` and `en`. The existing `t_resolves_known_key` test pattern applies.
- **Unit (bff layout)** — One test asserting `BASE_CSS` contains a `.badge--pending` rule, so a future refactor cannot silently drop it.
- **SSR snapshot (manual for v0.7)** — Render the dashboard with a synthetic `DashboardResponse` containing a mixed-`reviewed` hotline list, eyeball the HTML output. A proper SSR snapshot test framework is not yet set up; deferred.

Expected new test count: **+3 unit tests** (74 total).

## Out of scope

- A user feedback form for reporting translation issues — separate RFC if we want it.
- Marking individual UI strings as `reviewed=false` at the FTL key level. The current model treats reviewed-ness as language-level (whole locale is reviewed or not). Per-key reviewed-ness is a reasonable future extension but unnecessary here.

## Open questions

- The `/help/translation-review` page would benefit from being multilingual once `ar`/`uk`/`es` translations land. For v0.7 (initial), English-only is acceptable since users currently arrive at the page only when they have already been fallen back to English.
- Should the badge be dismissible per-session (so a returning user is not nagged)? My instinct is no — the warning is small enough that ambient presence is fine, and dismissibility risks training users to ignore safety notices. Open to pushback.
