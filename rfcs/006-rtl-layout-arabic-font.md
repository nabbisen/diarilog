# RFC 006: RTL layout and Arabic font delivery

| Field | Value |
|---|---|
| Status | Proposed |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |
| Template | Standard |

## Summary

Make the UI render correctly when `lang == "ar"` (and other RTL languages added later: `he`, `fa`, `ur`). Two halves: visual layout flipping driven by the `dir="rtl"` attribute that bff already emits, and reliable Arabic glyph rendering via a self-hosted font.

## Motivation

The bff layer already emits the right `<html lang dir>` attributes (RFC 01-precursor v2.4). The CSS, however, does not respond to `dir="rtl"`. Margins, paddings, alignments, the position of icons relative to text — everything is hardcoded for LTR. An Arabic-speaking user would currently see correct character shaping but a mirrored-wrong layout, which is worse than English-only would be.

Separately, Arabic glyph rendering depends on the user's device having an Arabic font installed. Default coverage is good on iOS and Android, weaker on desktop Linux and older Windows. Self-hosting a small Arabic font subset removes that variability.

This RFC unblocks the Arabic translation work that follows RFC 005.

## Requirements

1. **R1 (functional, must)** — When `dir="rtl"` is set on `<html>`, all primary page surfaces (Index, Login, Dashboard) lay out right-to-left: text aligns to the right, lists indent to the right, navigation reads right-to-left.
2. **R2 (functional, must)** — Bidirectional text (mixed LTR and RTL in the same string, e.g. an English URL inside Arabic prose) renders correctly via the browser's bidi algorithm. We do not override it.
3. **R3 (functional, must)** — Arabic glyphs render correctly on browsers without a system Arabic font, by falling back to a self-hosted webfont served from the bff Workers Static Assets bundle.
4. **R4 (non-functional, must)** — The font addition does not push the static asset bundle past Cloudflare's free-tier limits or noticeably degrade first-paint latency for non-Arabic users (i.e. font is loaded only when needed).
5. **R5 (non-functional, should)** — The same approach generalizes to other RTL languages (`he`, `fa`, `ur`) without additional CSS work; only the font may differ.

## Design

### CSS strategy

Switch the layout primitives in `BASE_CSS` (`workers/bff/src/ssr/layout.rs`) from physical properties to logical properties:

- `margin-left` / `margin-right` → `margin-inline-start` / `margin-inline-end`
- `padding-left` / `padding-right` → `padding-inline-start` / `padding-inline-end`
- `text-align: left` / `right` → `text-align: start` / `end`
- `border-left` / `border-right` → `border-inline-start` / `border-inline-end`
- `float: left` / `right` → `float: inline-start` / `inline-end`

Logical properties are supported by all evergreen browsers and are the cleanest solution: the same CSS rule produces correct layout in both LTR and RTL contexts because the browser resolves "inline-start" based on the document's `dir`.

For the few cases where a true horizontal direction is needed (icons, scroll indicators, drop shadows that should not flip), keep the physical property and override under `[dir="rtl"]` selectors.

### Font hosting

Add a single Arabic font: **Noto Sans Arabic** (SIL Open Font License 1.1).

- File: `workers/bff/dist/static/_assets/fonts/NotoSansArabic-subset.woff2`
- Subset: Arabic block (U+0600–U+06FF), Arabic Supplement (U+0750–U+077F), Latin basic (U+0020–U+007F) for inline LTR snippets. Generated with `pyftsubset`. Target file size: under 80 KB.
- Build step: a new `scripts/build-fonts.sh` that subsets the upstream Noto file and writes to the static assets directory. Run during release prep, not on every build.
- Licensing: place a `LICENSE-NotoSansArabic.txt` next to the font file. Update root `NOTICE` to reference Noto.

Loading strategy in CSS:

```css
@font-face {
  font-family: "Noto Sans Arabic";
  src: url("/_assets/fonts/NotoSansArabic-subset.woff2") format("woff2");
  font-display: swap;
  unicode-range: U+0600-06FF, U+0750-077F;
}

html[lang="ar"] body {
  font-family: "Noto Sans Arabic", system-ui, sans-serif;
}
```

The `unicode-range` descriptor means the browser only fetches the file when it actually needs to render a character in that range. Non-Arabic users pay zero bytes for the font.

### Direction-specific overrides

A small `[dir="rtl"]` block in `BASE_CSS` for the cases logical properties cannot reach:

- Mirroring of icons that have inherent direction (back-arrow icons, etc. — currently none in the codebase, but reserve the convention).
- The `<select>` dropdown chevron in the language switcher (RFC 004) — keep the chevron on the trailing edge regardless of direction.

### Hydration and SSR consistency

`HydrationConfig.dir` (already in v0.6) is set by SSR. The hydrate side does not need to read it; the browser renders from the HTML attribute directly. No code change needed in `packages/web-app` for direction handling.

## Test plan

- **Unit (bff layout)** — Tests asserting that `BASE_CSS` contains the logical-property rules and the `@font-face` declaration. The aim is to detect a future refactor that accidentally drops them.
- **Unit (web-app i18n)** — `is_rtl("ar")` already covered. Add a snapshot of which languages return RTL: `ar`, `he`, `fa`, `ur` true, all others false. Already in `i18n.rs::tests::rtl_detection` partially — extend.
- **Manual visual check (deferred to v0.8 or later)** — Take a browser screenshot of the dashboard with `Accept-Language: ar` after Arabic translations from RFC 005 land. Compare to the LTR version side-by-side. There is no automated visual regression framework in the project yet; this remains manual.

Expected new test count: **+2 unit tests**.

## Out of scope

- Hebrew, Persian, or Urdu fonts. Add when those languages move toward `reviewed: true`.
- A separate Arabic stylesheet bundle for users on very slow connections. Premature; the unicode-range optimization should suffice for v0.8.
- Right-aligned form inputs as a stylistic choice. Browsers handle text direction inside `<input>` automatically; no special work needed.

## Open questions

- Does the font subset cover the diacritics used in formal Arabic (fatha, kasra, damma, sukun, shadda)? Need to verify with the linguist when RFC 005 reviewers are engaged. If not, expand the subset.
- Should we also subset for Persian/Urdu Arabic-script extensions (U+0750–U+077F, U+FB50–U+FDFF, U+FE70–U+FEFF) preemptively, even though those languages are not yet in scope? Trade-off: ~20 KB more font weight against the cost of a future regeneration. Lean toward including them now to keep the build script simple.
