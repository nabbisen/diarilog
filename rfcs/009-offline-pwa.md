# RFC 009: Offline support (PWA, Service Worker, sync)

| Field | Value |
|---|---|
| Status | Proposed |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |
| Template | Full |

## Summary

Make diarilog usable without a network connection: enable journal creation and reading offline, queue changes locally, and synchronize them on reconnect. Ship as a Progressive Web App in v0.x; native app wrappers are a possible follow-up.

## Background

The original requirements document targeted populations in conflict zones, refugee camps, and other contexts where connectivity is intermittent. Phase 1 acknowledged "PWA / native both possible" but did not implement either. This RFC commits to PWA first, with the understanding that:

- A pure PWA on iOS has known restrictions (Safari's storage-clearing behavior, install-prompt limitations, push notification gaps until iOS 16.4+). These are acceptable trade-offs given that no native app is ready and a partial-offline PWA is better than nothing.
- Native wrappers (Capacitor, Tauri Mobile) are reasonable v3.x work but out of scope here.

E2EE design constraints carry over from RFC 007: the user's key is on the device. Offline operation does not change that — entries are encrypted client-side regardless of connectivity, and the encrypted blobs sync when the network returns.

## Motivation

Three user contexts where offline-capable journaling is more than nice-to-have:

1. **Conflict zones / displacement** — Connectivity in transit camps, border crossings, or active emergency zones is sporadic. A user wanting to write through trauma cannot wait for connectivity to load the page, let alone post each entry.
2. **Privacy-conscious daily use** — A user may keep their phone on airplane mode for hours of focused journaling, surfacing only briefly to sync. The current app would be unusable in that mode.
3. **Crisis moments** — A user feeling worst at 3 AM with poor cellular signal must be able to open the app, see their previous entries, see crisis-resource hotlines from cache, and write without waiting.

## Requirements

### Core offline functions

1. **R1 (functional, must)** — When the device is offline, the user can open the app, see the dashboard with their most recent entries, read those entries, and create new entries. New entries are saved locally and queued for sync.
2. **R2 (functional, must)** — Crisis resources (`CrisisResources`, hotlines) must be available offline. The IASP fallback is the minimum guaranteed.
3. **R3 (functional, must)** — On reconnect, queued entries sync to the server. The user is informed visually that sync occurred (success or failure).
4. **R4 (functional, should)** — A user opening the app for the first time on a new device while offline should see a clear "log in once online" message, not a confusing error.

### Sync correctness

5. **R5 (functional, must)** — A queued entry is never silently lost. If sync fails, it remains queued and is retried.
6. **R6 (functional, must)** — Server is the conflict-resolution authority for the same entry. If the same diary entry was edited offline on two devices, server state wins on whichever syncs last; the other version is preserved as a separate entry with a "merged-from-conflict" flag, never deleted.
7. **R7 (functional, must)** — The local-only entries are visible in the dashboard with a small marker indicating "not yet synced".

### Storage and security

8. **R8 (security, must)** — Entries stored locally are encrypted at rest using the same key the server stores ciphertext for. If the device is compromised the attacker faces the same E2EE wall as a server compromise.
9. **R9 (security, must)** — The Service Worker cache does not include responses to authenticated endpoints unless the response was already encrypted at rest (i.e. diary bodies are fine to cache as ciphertext; raw user records are not).
10. **R10 (non-functional, must)** — The app's first-load size in offline-cache is small enough to install over a mobile data connection in under 30 seconds at 3G speeds.
11. **R11 (non-functional, should)** — The Service Worker self-updates on connectivity restoration so users do not run permanently outdated versions.

## Design

### High-level pieces

```
┌────────────────────────────────┐
│ Browser (PWA installed)       │
│                                │
│ Service Worker (sw.js)         │
│ ├─ shell cache (HTML/CSS/WASM) │
│ ├─ data cache (read-through)  │
│ └─ outbox queue (writes)      │
│                                │
│ IndexedDB (encrypted)          │
│ ├─ diary entries (cache)       │
│ ├─ crisis resources (cache)    │
│ └─ pending operations          │
└────────────────────────────────┘
              │
              │ when online
              ▼
        bff-worker → core
```

### Service Worker scope and strategies

Single SW registered at `/sw.js`, scope `/`. Cache strategies per resource type:

| Resource | Strategy | Reason |
|---|---|---|
| Static assets (`/_assets/*`) | Cache-first with stale-while-revalidate | Small set, changes only on deploy |
| HTML for primary routes (`/`, `/login`, `/dashboard`) | Network-first with cache fallback | SSR'd content can be stale-tolerant |
| `GET /api/dashboard` | Network-first with cache fallback, max age 24h | Aggregation API; staleness is acceptable to the user as long as it is shown |
| `GET /api/diary/<id>` | Cache-first | Diaries do not change unless the user edits |
| `POST /api/diary` (create) | **Background sync queue** | Enqueued offline, replayed on reconnect |
| `PUT /api/me` | Network-only | Settings; no value in offline support |

The Service Worker uses Workbox patterns but the implementation is a small hand-written `sw.js` rather than the full Workbox runtime to keep the offline-cache size down.

### Local storage layout (IndexedDB)

Database `diarilog`, version 1. Object stores:

- `entries` — cached diary entries, key = entry id. Body field is ciphertext (mirrors R2).
- `entries_meta` — metadata (id, title, created_at, mood_score, sync_state). `sync_state` ∈ `{synced, pending, conflict}`.
- `outbox` — queued operations not yet sent. Each record: `{ op_id, op_type, payload, retry_count, last_error, created_at }`.
- `crisis_resources` — language-keyed cached snapshot of `crisis_resources(lang)`.
- `keys` — local copy of the user's encryption key derivation parameters (NOT the master passphrase, which is never stored).

### Sync protocol

On reconnect (or on a periodic 30-second tick when online):

1. Read all entries in `outbox` ordered by `created_at`.
2. For each, send to the appropriate API endpoint.
3. On 2xx, remove from outbox; update `entries_meta.sync_state` for the related entry to `synced`.
4. On 4xx (validation/auth), mark the operation as `failed` and surface an alert; do not retry the same op without user action.
5. On 5xx or network error, increment `retry_count` and retry with exponential backoff (max 5 retries, then mark failed).
6. After processing the outbox, do a "pull" pass: fetch `/api/dashboard`, compare with local cache, merge differences.

### Conflict handling

Two cases:

- **Local-only entry not yet synced** — straightforward; sync as a fresh `POST /api/diary`. New server id is associated.
- **Edited entry that was also edited on another device** — server wins; local version is reinserted as a new entry with a `merged_from: <original_id>` field and `sync_state: synced`. The user sees both copies in the dashboard with a visual indication of the conflict.

This is a "last-write-wins on the same edit, preserve-as-fork on conflict" policy. It guarantees no data loss while keeping the implementation tractable.

### App shell and `manifest.webmanifest`

Add a `manifest.webmanifest` served by bff at `/manifest.webmanifest`. Includes app name (`diarilog`), short_name, theme color, icons (512/192/maskable), display mode `standalone`, scope `/`, start_url `/`.

The icon set uses a simple lockup mark; design is out of scope for this RFC, the implementer can use a placeholder until brand assets are decided.

### Update strategy

The Service Worker is versioned by its file content hash. On registration, browsers detect the file change and install the new SW in the background; activation happens on the next navigation after all controlled tabs are closed, to prevent mixing versions.

A small visual indicator ("App updated — refresh to apply") is shown in the header when the new SW is installed but waiting to activate. Manual refresh applies immediately.

### Encryption at rest in IndexedDB

The user's master key is derived (PBKDF2 / Argon2) from a passphrase entered at first unlock per session. The derived key is held in a service worker variable for the session lifetime; on unload, it is cleared. IndexedDB ciphertext blobs are usable across sessions; the user re-derives the key by entering the passphrase again.

This means:

- An attacker with read access to IndexedDB on a stolen device sees ciphertext only.
- The user must enter a passphrase to unlock the app per session. This is a UX cost; we accept it given the threat model (trauma content is among the most sensitive personal data).
- Forgotten passphrase = unrecoverable data. Same property as the live service. RFC 007 (export) is the user-controlled mitigation.

## Test plan

- **Unit (web-app)** — Tests for the outbox enqueuer, serializer, and the conflict policy decision tree using mocked IndexedDB.
- **Integration (browser harness)** — Headless Chrome with offline mode toggled. Scripted scenario: load app online, go offline, write 3 entries, return online, verify all 3 reach the server.
- **Service Worker lifecycle test** — Install old SW, ship new SW, navigate, verify update-pending banner and post-refresh activation.
- **Storage limit test** — Simulate quota-exceeded errors during cache writes; verify graceful degradation (the app still functions, even if cache cannot grow further).
- **Manual on real devices** — iOS Safari, Android Chrome, Firefox desktop. Each gets a basic offline-write-then-sync run.

Expected new test count: **+15 unit tests**, plus integration scaffolding that may not contribute to the workspace `cargo test` count (browser tests live separately).

## Security considerations

### Threat model additions

| Threat | Mitigation |
|---|---|
| Stolen device reveals plaintext entries | IndexedDB body fields are ciphertext; passphrase-required unlock |
| Other site on the same origin reads IndexedDB | Same-origin policy; but service worker scope = `/`, no third-party content |
| Malicious browser extension reads passphrase as user types | Out of scope; users with hostile extensions cannot be protected by web-app code |
| Quota exhaustion DoS | Hard cap on outbox size (default 1000 entries); user is warned to sync |
| Service Worker hijack via stale cache after security fix | SW versioned by content; activation policy ensures old SW does not serve fixed-out vulnerabilities for long |
| User logs out but cached entries remain | Logout clears IndexedDB entirely; survival across logout is not promised |

### Step-up vs offline

RFC 007 (export/import) requires step-up OIDC for sensitive operations. Step-up is necessarily online; offline mode must not allow export/import. The PWA simply hides those controls when offline.

## Alternatives considered

- **No offline support, PWA only as install convenience** — Rejected; defeats the primary motivation (refugee/conflict-zone usability).
- **Full offline-first sync engine (CouchDB-style)** — Rejected as overkill. Diarilog's data model is single-user-owned with very simple write patterns; the outbox + last-write-wins design is sufficient.
- **Native app wrapper instead of PWA** — Deferred. PWA gets us 80% of the benefit at 20% of the work and serves as a stepping stone if a native app proves desirable later.
- **Encrypt only diary bodies in IndexedDB, leave metadata cleartext** — Rejected. Metadata (titles, timestamps) is sensitive in a trauma-care context; "I wrote a journal at 2 AM titled 'I can't anymore'" should not be readable from a stolen device.

## Migration / rollout

- **v0.8** — Service Worker, app shell caching, basic read-only offline mode. Outbox stub but no sync yet. PWA installable.
- **v0.9** — Outbox sync, conflict handling, in-app sync-status UI.
- **v0.10** — IndexedDB encryption-at-rest, passphrase-unlock UX. (This is the highest-effort piece and benefits from being decoupled from the rest.)
- **v3.x** — Native wrappers, push notifications.

This is a multi-release effort; the RFC commits to the design but not to a single delivery date.

## Open questions

- iOS Safari aggressively clears site data after 7 days of non-use. Is the recommendation to warn users to open the PWA at least weekly? Or should we explore App Store distribution (Capacitor) as the answer for iOS specifically? Lean toward the latter for a future v1.x; for v0.x users on iOS get a "may need to re-sync after long absence" caveat.
- Background sync API has limited support (Chromium only); Safari/Firefox do not yet support it. The fallback is "sync on next app open while online" which is workable but means a closed-tab device will not push entries. Document the limitation in user-facing release notes.
- The passphrase UX has a real risk of users picking weak passphrases. Should we enforce minimum entropy at unlock time? Lean yes (zxcvbn-style estimator, refuse below score 3).
- Push notifications for crisis-resource updates or sync failures: out of scope here, but the SW infrastructure makes it possible. Defer to a separate RFC.
