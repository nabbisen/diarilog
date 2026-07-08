# diarilog

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust 1.91](https://img.shields.io/badge/rust-1.91-orange.svg)](rust-toolchain.toml)
[![Cloudflare Workers](https://img.shields.io/badge/platform-Cloudflare%20Workers-F38020.svg)](https://workers.cloudflare.com/)

An AI-assisted journaling platform for people carrying extreme stress or trauma — including those facing mental health challenges, victims of crime, and people displaced by conflict.

---

## What it does

Writing through difficult experiences has strong support in trauma care, but staring at a blank page can itself be a barrier. diarilog reduces that barrier: an AI asks gentle, open-ended questions; the user answers in their own words; a private journal entry takes shape from that dialogue.

Three principles hold across every design decision:

**Trauma-informed care before feature velocity.** A mistranslated crisis hotline, a judgmental AI prompt, an unexpected mood-trend chart surfaced at the wrong moment — these can cause real harm. Safety is not a phase.

**The user owns the data.** Journal bodies are encrypted on the device before they reach the server. The server stores ciphertext it cannot read. Emergency erase wipes everything, on device and server, irreversibly, in one tap.

**Edge-complete, no central server.** Six Rust workers compiled to WASM run on the Cloudflare edge. No application server to operate, no region to choose, predictable latency for users on slow or intermittent connections.

---

## Architecture

```
Browser / PWA
     │ HTTPS
     ▼
gateway-worker               ← sole public endpoint; OIDC auth, routing
     │
     │  Service Bindings — internal only, not reachable from outside
     │
     ├──▶ bff-worker          Leptos SSR · Workers Static Assets · /api/dashboard
     │         │
     │         ├──▶ identity-worker    user profiles, onboarding (D1)
     │         ├──▶ journal-worker     diary CRUD + version history (R2, D1)
     │         └──▶ dialog-worker      interview sessions, AI prompts
     │                   │
     │                   └──▶ safety-worker    crisis detection, AI classifier
     │
     └──▶ core workers directly for authenticated write paths
```

The browser receives full SSR HTML on first load. A WASM hydration bundle is then served from the edge via the Workers Static Assets binding — no Worker invocation, no request billing for static files.

### Storage

| Purpose | Storage |
|---|---|
| Encrypted journal bodies | R2 — server never decrypts |
| User profiles, diary metadata, sessions | D1 (SQLite) |
| OIDC JWKS cache, user preferences | KV |

---

## Technical stack

| Layer | Choice |
|---|---|
| Language | Rust 2024 Edition |
| Runtime | Cloudflare Workers (`wasm32-unknown-unknown`) |
| UI framework | Leptos 0.8 (SSR + WASM hydration) |
| i18n | `fluent-templates` + Project Fluent |
| AI | Cloudflare Workers AI — `@cf/meta/llama-3.1-8b-instruct` |
| Auth | OIDC Authorization Code + PKCE (any compliant provider) |
| Bot protection | Cloudflare Turnstile |

---

## Quick start

### One-time setup

```bash
rustup target add wasm32-unknown-unknown   # wasm32 stdlib
apt-get install -y pkg-config libssl-dev   # worker-build dependency
cargo install worker-build                 # Rust → WASM build tool
npm install -g wrangler                    # Cloudflare CLI
```

### Run locally

```bash
wrangler dev        # → http://localhost:8787
```

One command. No `wrangler login` needed.
See [`docs/local-development.md`](docs/local-development.md) for OIDC setup.

### Tests

```bash
cargo test --workspace --tests
# 94 tests, no Cloudflare account needed
```

---

## Repository layout

```
diarilog/
├── packages/
│   ├── contracts/     types at worker boundaries
│   ├── auth-core/     OIDC validation, JWKS cache
│   ├── crypto/        E2EE key types: KdfParams, WrappedDek, EncryptedField
│   ├── errors/        ApiError, ErrorCode
│   ├── sb-client/     typed Service Bindings clients
│   └── web-app/       Leptos components, i18n, route enum
│       └── locales/   Project Fluent .ftl files (ja, en)
├── workers/
│   ├── gateway/       public edge — auth, routing
│   ├── bff/           SSR, static assets, dashboard aggregation
│   ├── bff-hydrate/   CSR WASM bundle (wasm-pack, workspace.exclude)
│   ├── journal/       diary CRUD, version history
│   ├── identity/      user profiles, onboarding
│   ├── safety/        crisis detection, CrisisResources
│   └── dialog/        interview sessions, AI prompts
├── migrations/        D1 SQL migrations (0001–0003)
├── rfcs/              design specs for each work item (RFC 001–012)
├── docs/
│   ├── local-development.md
│   ├── hydration-verification.md
│   ├── i18n-review-flow.md
│   └── deployment/    prerequisites, setup, deploy order, rollback, observability
└── scripts/
    ├── build-all-workers.sh
    ├── build-bff-ssr.sh
    └── build-bff-hydrate.sh
```

---

## API

All routes go through `gateway-worker`. Every `/api/*` route requires a valid
OIDC ID token in `Authorization: Bearer <token>`, except `/api/health`.

| Method | Path | Notes |
|---|---|---|
| `GET` | `/api/health` | No auth |
| `GET` | `/api/dashboard` | Aggregated: user + recent diaries + active session |
| `GET` | `/api/me` | Profile |
| `PUT` | `/api/me` | Update display name or language |
| `POST` | `/api/diary` | Create entry; body must be AES-GCM encrypted |
| `GET` | `/api/diary` | List entries |
| `GET` | `/api/diary/:id` | Entry detail + encrypted body |
| `PUT` | `/api/diary/:id` | Edit entry — creates a new version |
| `DELETE` | `/api/diary/:id` | Soft-delete |
| `GET` | `/api/diary/:id/versions` | Version list (metadata, no bodies) |
| `GET` | `/api/diary/:id/versions/:n` | Specific version with body |
| `DELETE` | `/api/diary/:id/versions/:n` | Delete a prior version |
| `POST` | `/api/interview/start` | Start interview session |
| `POST` | `/api/interview/answer` | Submit answer, receive next question |
| `GET` | `/api/interview/:session_id` | Get session by ID |
| `POST` | `/api/suggest` | Generate draft suggestions |
| `GET` | `/api/triggers` | List trigger keywords |
| `POST` | `/api/triggers` | Add trigger keyword |
| `DELETE` | `/api/triggers/:id` | Remove trigger keyword |
| `POST` | `/api/auth/verify-turnstile` | Verify Turnstile token at registration |
| `POST` | `/api/sync` | Offline sync |
| `POST` | `/api/erase` | Emergency erase — irreversible |

---

## Multilingual support

Supported languages: `ja`, `en`, `ar`, `uk`, `es`.

| Area | Status |
|---|---|
| UI — ja, en | ✅ |
| AI prompts — all 5 languages | ✅ ja/en reviewed; ar/uk/es provisional |
| Crisis resources — all 5 languages | ✅ ja/en reviewed; ar/uk/es provisional + IASP fallback |
| UI — ar, uk, es | ⏳ pending expert clinical review |
| RTL layout (Arabic) | ⏳ after translations land |
| Language switcher UI | ⏳ RFC 004 |

Every crisis resource in a provisional language includes the IASP international
hotline list (`reviewed: true`) as a guaranteed fallback — a unit test enforces
this invariant so no code change can accidentally remove it.

Review criteria and the annual re-review cycle are in
`rfcs/005-translation-review-criteria.md` and `docs/i18n-review-flow.md`.

---

## ROADMAP

### v0.7 — current

RFC 002 rename · RFC 008 CI/CD · RFC 010 emergency erase UI ·
RFC 012 entry version history ·
RFC 011 E2EE key model *(server-side complete; browser crypto in progress)*

### v0.8

RFC 003 `reviewed: false` UI badges ·
RFC 004 language switcher + `PUT /api/me` ·
RFC 006 RTL CSS + Noto Sans Arabic

### v0.9

RFC 007 user-controlled export / import (E2EE zip archive) ·
RFC 009 part 1 — PWA install, Service Worker, app shell caching

### v0.10

RFC 009 part 2 — IndexedDB E2EE, offline outbox sync, conflict resolution ·
ar / uk / es translations after expert review

### v1.0 — release criteria

- E2EE passphrase flow working end-to-end in a real browser
- Crisis resources for ar, uk, es reviewed by a clinical expert
- At least one live Cloudflare deployment with a real user
- Offline PWA working at a basic read/write level

### Beyond v1.0

Anonymous pattern matching · organization dashboard · professional referral
flow · additional languages

---

## Design references

Implementation specs live in [`rfcs/`](rfcs/README.md). Each RFC covers
requirements, design, test plan, and (for larger items) security
considerations and migration path. RFCs 001–012 are written; the next batch
will follow the same process.

---

## License

Apache-2.0. See [LICENSE](LICENSE).

---

> **Disclaimer.** diarilog is a self-care support tool, not medical treatment.
> If you or someone you know is in crisis, please reach out to a local helpline
> or emergency services. The [IASP crisis centre directory](https://www.iasp.info/resources/Crisis_Centres/)
> covers centres worldwide.
