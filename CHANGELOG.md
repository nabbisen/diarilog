# Changelog

All notable changes to diarilog are documented here.

## Version scheme

diarilog uses [Semantic Versioning](https://semver.org).
**Until v1.0.0, the project is under active development: APIs, data schemas, and
deployment procedures may change between minor versions without a deprecation
period.** No production deployments are expected before v1.0.0.

`v0.MINOR.PATCH` where `MINOR` increments per meaningful capability milestone
and `PATCH` for documentation or configuration-only changes.

---

## [0.8.0] — 2026-06-08

### Summary

Consolidated 6 separate Cloudflare Workers into one. Local development now
works with a single `wrangler dev` from the project root.

### Architecture change

The gateway, bff, journal, identity, safety, and dialog workers have been
merged into a single Cloudflare Worker at the project root. All routing is
internal; Service Bindings are no longer used. The new source layout:

```
src/
  lib.rs              router and event handler
  auth.rs             JWT validation (was gateway middleware)
  handlers/           one file per domain (diary, identity, safety, dialog, …)
  storage/            D1 + R2 access (diary, identity, triggers)
  dialog/             AI interview logic (ai_client, prompts, session)
  safety/             Crisis detection (classifier, ai_client)
  ssr/                Leptos SSR (handlers, layout)
```

### Removed

- `workers/` directory (all 6 separate workers)
- `packages/sb-client` (Service Bindings clients)
- `scripts/dev.sh` (no longer needed)

### wrangler.toml

Root `wrangler.toml` now covers the entire application in one file.
`wrangler dev` works from the project root.

### Test count

85 passed (7 binaries). The 9 tests removed were in `workers/` which no longer
exists; all packages/* tests are preserved and pass.

## [0.7.0] — 2026-05-22  *(current)*

### Summary

RFC implementations: codebase rename to `diarilog`, CI/CD pipeline, emergency
erase UI, E2EE key model, and entry version history.

### RFC 002 — Rename to `diarilog`

All legacy `trauma-journal-*` identifiers replaced in `wrangler.toml` worker
names, service-binding references, and D1/R2/KV resource names.
Window globals renamed: `__TJP_ROUTE__` → `__DIARILOG_ROUTE__`, same for
`DATA` and `LANG`.

### RFC 008 — CI/CD (GitHub Actions)

Three workflow files created under `.github/workflows/`:

- `ci.yml` — runs on every PR and push to `main`: `cargo check`, `cargo test`
  with a baseline guard (fails if the total passed count drops), `cargo fmt`,
  advisory `cargo clippy` and `cargo audit`.
- `deploy-staging.yml` — deploys to staging on merge to `main` using
  `dorny/paths-filter@v3` selective deployment in the correct dependency order.
  Builds the bff-hydrate CSR bundle before deploying bff.
- `deploy-production.yml` — `workflow_dispatch` only; requires a `confirm_tag`
  input matching HEAD and reviewer approval via GitHub environment protection.

### RFC 010 — Emergency erase UI

Settings page (`/settings`) added with an emergency erase section.
Two-step confirmation: user types "ERASE" (or the localized equivalent "消去"
in Japanese) before the button becomes active. Plain HTML form — works before
hydration completes.

FTL keys added for both `en` and `ja`. `Route::Settings` and `Route::Onboarding`
added to the route enum.

### RFC 011 — Encryption boundary, key derivation, and multi-device access

New `packages/crypto` crate defines the serializable types shared between client
and server: `KdfParams` (Argon2id cost parameters), `WrappedDek`,
`EncryptedField`, `PassphraseChangeRequest`. All actual cryptographic operations
(Argon2id, AES-256-GCM) happen in the browser; the server stores opaque blobs.

`UserRecord` extended:
- `onboarding_completed: bool` — gates access to the dashboard.
- `kdf_salt`, `wrapped_dek`, `kdf_params_json` — key-derivation material.
- `display_name` changed from `String` to `Option<String>`.

`DiaryMeta` extended:
- `encrypted_title` and `encrypted_mood` replace cleartext `title` and
  `mood_score`.
- `version: u32` counter added (default 1).

D1 migration `0002_e2ee_key_model.sql` adds the new columns with `NULL` defaults
for backward compatibility.

Onboarding page (`/onboarding`) renders the passphrase-setup form and routes
users whose `onboarding_completed` flag is false.

`contracts/src/identity.rs` gains `SetupPassphraseRequest`, `OnboardingStatus`,
`SetupPassphraseResponse`.

### RFC 012 — Entry edit history and immutability

D1 migration `0003_diary_versions.sql` creates the `diary_versions` table
(cascade-delete on diary removal, back-filled with version 1 for all existing
entries).

`DiaryStorage.update()` now:
- Writes a new R2 object for each edit.
- Inserts a `diary_versions` row.
- Enforces a 20-version-per-entry soft cap (original version always preserved).

New endpoints in journal-worker and gateway:
- `GET  /api/diary/:id/versions`
- `GET  /api/diary/:id/versions/:n`
- `DELETE /api/diary/:id/versions/:n`

`JournalClient` (sb-client) gains `list_versions`, `get_version`,
`delete_version`.

### Test count

94 passed (12 binaries). Up from 71 at v0.6.

---

## [0.6.0] — 2026-05-04

### Summary

Phase 2 finishing work: i18n foundation, multi-language backend structure,
RFC scaffolding.

i18n foundation (`fluent-templates` + `unic-langid`), `ja`/`en` translations,
`Accept-Language` language resolution, `__DIARILOG_LANG__` hydration signal,
HTML `lang`/`dir` dynamic, `is_rtl()` detection.

Backend 5-language structure: `dialog/prompts.rs` module with 5-language
AI prompt templates (`reviewed` flags), `safety/classifier.rs` with balanced
`CrisisResources` for all 5 languages plus always-on IASP international
fallback. `HotlineInfo` gains `region` and `reviewed` fields;
`CrisisResources` gains `message_reviewed`.

`rfcs/` directory created with RFC 01–010.

71 passed (12 binaries).

---

## [0.5.0] — 2026-04-28

### Summary

Phase 2 foundation: 3-tier architecture, BFF aggregation API, SSR integration,
deployment documentation.

bff-worker added (internal, Leptos SSR + Workers Static Assets + aggregation
API). `GET /api/dashboard` fans out to identity, journal, and dialog in
parallel via `futures::future::join3`; partial degradation pattern.
`aggregate_for(env, cctx)` shared by API handler and SSR route so the
dashboard page pre-renders data server-side.

bff-hydrate built separately with wasm-pack (workspace.exclude).
`window.__DIARILOG_ROUTE__` / `__DIARILOG_DATA__` / `__DIARILOG_LANG__`
injected by SSR, read back by hydrate.

`docs/deployment/` expanded to 9 files:
`README.md`, `prerequisites.md`, `initial-setup.md`, `deploy-order.md`,
`multi-env.md`, `feature-flags.md`, `oidc-providers.md`, `rollback.md`,
`observability.md`.

`docs/hydration-verification.md` created.

---

## [0.4.0] — 2026-04-27

### Summary

Phase 1 MVP complete: 5-worker structure, OIDC authentication, Leptos SSR.

`gateway`, `bff`, `journal`, `identity`, `safety`, `dialog` workers deployed
via Service Bindings. `gateway` is the sole public worker; all others have
`workers_dev = false`.

OIDC (Authorization Code Flow + PKCE) replaces the originally planned
Cloudflare Access. RS256/RS384/RS512 only; JWKS cached in KV.
`X-User-Id` / `X-User-Email` / `X-Trace-Id` propagated internally;
gateway overwrites `X-User-*` to prevent spoofing.

Leptos v0.8 SSR in bff. `fluent-templates` + `unic-langid` added. Pages:
Index, Login, Dashboard, NotFound.

`packages/contracts` for cross-worker types. `packages/auth-core` for OIDC
validation. `packages/errors`, `packages/sb-client`.

`workers/safety`: keyword-based crisis detection, `CrisisResources`,
AI-classifier integration. `workers/dialog`: interview sessions, draft
generation, AI prompts.

---

## Version history before v0.4.0

Internal development milestones from early prototyping (single-worker
monolith through feature-flag-driven 5-worker split) are recorded in git
history but not documented here. The project was not yet in a state suitable
for external reference.
