# RFC 002: Rename application to `diarilog`

| Field | Value |
|---|---|
| Status | Implemented |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |
| Template | Lightweight |

## Summary

Replace all in-repo references to the legacy app name (`trauma-journal-platform` / `TJP`) with the new app name `diarilog`. This is a mostly mechanical rename driven by a finalized branding decision; no behavior changes.

## Motivation

The development guideline v2 (kept outside the repo) finalized the app name as `diarilog`. Existing code, configuration, and identifiers still use the legacy `trauma-journal-*` and `__TJP_*` strings. Leaving the codebase in a half-renamed state is confusing for new contributors and produces grep noise.

## Plan

The rename touches three categories of references. Implementer should do them in this order so that nothing references a not-yet-renamed identifier.

### 1. Build configuration

- `Cargo.toml` (workspace root): no package name to change at the workspace level, but verify nothing references `trauma-journal-*`.
- Each `workers/*/Cargo.toml`: change `name = "trauma-journal-foo"` to `name = "diarilog-foo"`.
- Each `packages/*/Cargo.toml`: same treatment if any of them carry the legacy prefix.
- `workers/*/wrangler.toml`: change worker `name`, service binding `service` references, and the resource names (D1 database name, R2 bucket name, KV namespace title) for **dev/staging only**. Production resource names should be discussed separately because renaming a live D1 database in Cloudflare requires migration, not a config edit.

### 2. Source identifiers

- `__TJP_ROUTE__`, `__TJP_DATA__`, `__TJP_LANG__` (window globals injected by SSR and read by hydrate): rename to `__DIARILOG_ROUTE__`, `__DIARILOG_DATA__`, `__DIARILOG_LANG__`. Both the SSR injection (`workers/bff/src/ssr/layout.rs`) and the hydrate readers (`packages/web-app/src/lib.rs`) must change in the same commit, otherwise hydration breaks.
- Any other `TJP` constants in source code — search with `git grep -i tjp` and rename together.

### 3. Documentation

- `README.md` hero and Quick Start sections.
- `CHANGELOG.md` going forward — historical entries keep their original wording.
- `docs/**/*.md` — update where the new name reads more naturally, but leave historical sections alone (e.g. "Phase 1 used the codename TJP" is fine).

## Production resources (deferred)

Renaming live Cloudflare resources (D1, R2, KV in production) is **out of scope for this RFC**. Doing it safely needs a separate plan that includes data migration. The dev/staging rename in this RFC is a precondition for that follow-up; production resources keep their old names until then. The discrepancy is acceptable because production resource names are not user-visible.

## Verification

After the rename, the existing checks should still pass with the same test count:

```
cargo check --workspace --offline
cargo check --workspace --all-targets --offline
cargo test --workspace --offline --tests
```

Expected: 71 passed (unchanged from v0.6).

A successful hydration smoke test (`docs/hydration-verification.md`) should also still pass; pay attention to step 3, which inspects the injected globals — those are now `__DIARILOG_*`.

## Open questions

- Should the legacy aliases (`__TJP_*`) be kept as deprecated for one release, in case any external integration scrapes them? My read is no — there is no public consumer yet — but flagging in case the implementer disagrees.
