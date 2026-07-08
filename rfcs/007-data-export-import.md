# RFC 007: User-controlled data export and import

| Field | Value |
|---|---|
| Status | Proposed |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |
| Template | Full |

## Summary

Give users a way to export all of their data from diarilog into a portable archive, and to import that archive into another diarilog instance (or back into the same one after a reset). The archive preserves the end-to-end encryption: it is encrypted with the user's existing key material, and the export endpoint never sees plaintext.

## Background

Two adjacent ROADMAP items are conflated in casual discussion and need disambiguation up front:

- **Self-export** — The user, while authenticated, downloads their own data. Out of trauma-care contexts the typical motivation is data portability ("I want to leave the service"). For diarilog there is also a safety-side motivation: a user under threat may want a copy on a separate device they control, in case they need to wipe the primary device (the emergency-erase feature).
- **Other-service import** — Pulling diary entries written elsewhere (Day One, journaling apps, plain text files) into diarilog. This is a much larger problem because it requires per-service adapters and inevitably loses fidelity. **This RFC scopes only to self-export and re-import of a diarilog archive.** Other-service import is mentioned in ROADMAP and gets its own future RFC.

Some constraints set by earlier decisions that this RFC must respect:

- Diary bodies are stored encrypted at rest in R2; the server never holds the decryption key. (Phase 1, unchanged.)
- The user's key material has no escrow. If they lose it, the data is unrecoverable. (Phase 1, unchanged.)
- The export must not require the server to break those properties.

## Motivation

Three concrete user stories drive the requirement:

1. **Recovery** — A user is preparing to wipe their device (emergency-erase or simple device replacement) and wants to be sure their journal can be brought back later. Currently they have no way to do this.
2. **Portability** — A user wants to evaluate whether they trust a diarilog instance run by an NPO over one run by their employer. Without export, switching is destructive.
3. **Audit / personal record** — A user wants a long-term archival copy outside the live service, perhaps for therapeutic review, perhaps because they distrust any cloud service with permanent retention. The export gives them that copy in a format whose lifetime they control.

## Requirements

1. **R1 (functional, must)** — An authenticated user can request an export of all their data. The export covers: user profile (`UserRecord`), all diary entries (encrypted body + metadata), all interview sessions, all draft suggestions retained, language and trigger preferences.
2. **R2 (functional, must)** — The exported archive is a single file the user can download and store offline.
3. **R3 (functional, must)** — The exported archive can be imported into a fresh diarilog account, restoring all entries with their original timestamps and content. Re-importing into the *same* account that produced it must be idempotent (no duplicates).
4. **R4 (functional, must)** — Export and import preserve E2EE: the archive's diary bodies remain encrypted with the user's key. The user must hold the key to make sense of the archive after export.
5. **R5 (functional, should)** — The format includes a manifest with a version number, so future archive-format changes can be migrated. The archive format is documented separately (`docs/archive-format.md`, created in v0.7 implementation work).
6. **R6 (non-functional, must)** — Export of a typical user (years of daily entries, single-digit MB total) completes within the bff worker request budget (Cloudflare worker CPU time limits). Larger users are handled by streaming.
7. **R7 (non-functional, should)** — The format is human-inspectable enough that a knowledgeable user can verify integrity outside the application. Use widely available primitives (zip, JSON, base64) rather than a bespoke binary format.
8. **R8 (security, must)** — Export and import require fresh authentication. A long-lived session token alone is not enough; the user must re-confirm via OIDC step-up.
9. **R9 (security, must)** — The export endpoint logs the action with a hash of the user id and a timestamp, but **not** any export content. Failure modes (rate limit, validation error) are also logged.

## Design

### Archive format

A **zip** file containing:

```
diarilog-export-<user_id_hash>-<utc_iso_date>.zip
├── manifest.json                  // version, user_id_hash, created_at, counts
├── profile.json                   // UserRecord (excluding email if user opts out)
├── preferences.json               // language, triggers, settings
├── diaries/
│   ├── meta.json                  // array of {id, title, created_at, mood_score, body_ref}
│   └── bodies/
│       ├── <body_ref_1>.bin       // encrypted ciphertext copied from R2 verbatim
│       └── ...
├── sessions/
│   ├── meta.json                  // interview sessions
│   └── turns/
│       └── <session_id>.json      // turns within each session
└── drafts.json                    // saved draft suggestions
```

`manifest.json` shape:

```json
{
  "format_version": 1,
  "exporter_version": "diarilog v0.7.0",
  "user_id_hash": "sha256:...",
  "created_at": "2026-05-04T12:34:56Z",
  "counts": { "diaries": 312, "sessions": 14, "drafts": 8 }
}
```

The diary body files are byte-for-byte copies of the R2 objects. The server never decrypts them; the user's key, applied client-side after import, is the only thing that can.

### Export flow

1. User on settings page clicks "Export my data".
2. Frontend posts to `POST /api/export/start`.
3. bff returns 202 with a `job_id`. The export is asynchronous because zipping potentially many R2 objects is not bounded enough to fit in a single request.
4. bff submits a job to a new internal worker `export-worker` (or, for v0.7, a synchronous in-bff path with an explicit size cap; see below).
5. `export-worker` reads identity / journal / dialog data via Service Bindings, streams them into a zip, and writes the resulting zip to a temporary R2 prefix scoped to the job.
6. Frontend polls `GET /api/export/status/<job_id>` until status is `ready`.
7. When ready, the response carries a one-time signed URL pointing to the R2 object. The URL expires after 10 minutes.
8. After download, or after 24 hours, the temporary R2 object is deleted. (A scheduled-events worker handles cleanup; expiry is enforced by the scheduled worker, not by R2 lifecycle alone.)

For v0.7 (initial implementation), a **synchronous variant** is acceptable when the user has fewer than 200 diary entries: bff streams the zip directly in the response. The async path is the right answer once we exceed worker CPU/memory limits, and the architecture above is the planned shape.

### Import flow

1. User on settings page clicks "Import data" and selects a zip.
2. Frontend uploads the file to `POST /api/import/upload` (multipart). bff places it in a job-scoped R2 prefix.
3. bff submits a job to `import-worker`.
4. `import-worker` validates the manifest (format version, user_id_hash matches the authenticated user), then iterates through the archive contents:
   - profile.json → identity-worker (`UPDATE` if exists, `INSERT` otherwise)
   - diaries/meta.json + bodies/*.bin → journal-worker (`UPSERT` by body_ref to make re-import idempotent — R4)
   - sessions/* → dialog-worker
   - drafts.json → dialog-worker
5. Frontend polls `GET /api/import/status/<job_id>`.
6. The temporary R2 upload is deleted on completion regardless of outcome.

### New endpoints summary

| Method | Path | Auth | Notes |
|---|---|---|---|
| `POST` | `/api/export/start` | Step-up OIDC | Starts export job |
| `GET`  | `/api/export/status/<job_id>` | Session | Polled for completion |
| `POST` | `/api/import/upload` | Step-up OIDC | Multipart |
| `GET`  | `/api/import/status/<job_id>` | Session | Polled |

### New workers

- **export-worker** — Internal, Service Bindings to identity/journal/dialog. R2 write to temp prefix.
- **import-worker** — Internal, Service Bindings to identity/journal/dialog. R2 read from temp prefix.

These can be deferred to v0.8 if the synchronous variant is enough for v0.7. RFC accepts both paths; recommended is to start synchronous, migrate when needed.

## Test plan

- **Unit (export-worker)** — Format the manifest correctly, traverse a stub set of 3 diaries, produce a zip whose entries match the spec.
- **Unit (import-worker)** — Reject archives with mismatched user_id_hash, version 0, missing manifest, malformed JSON. Accept a well-formed archive and verify Service Binding calls dispatched in the expected order.
- **Round-trip integration test (manual for v0.7)** — Export a synthetic account, import the result into a fresh account, confirm counts match.
- **Negative test (manual)** — Attempt to import a tampered archive (modified body, modified manifest count) and confirm the import is rejected before any state is changed.

Expected new test count: **+10 unit tests** spread across new workers.

## Security considerations

### Threat model

| Threat | Mitigation |
|---|---|
| Attacker obtains the export URL after the user downloads | One-time signed URL, expires in 10 minutes, deleted on download |
| Attacker forges an import to overwrite victim's data | Step-up OIDC required; manifest must contain the authenticated user's id hash |
| Attacker uploads enormous file to exhaust storage | bff enforces a hard cap on `/api/import/upload` (default 100 MB), 413 above |
| Attacker uploads zip-bomb | import-worker decompresses with a hard ratio cap (10x compressed size); if exceeded, abort and delete |
| Server compromise leaks export jobs in flight | Job-scoped R2 prefix is encrypted at rest by Cloudflare; user's body files stay E2EE; manifest reveals only metadata |
| Server admin reads export contents | Body files remain ciphertext; metadata is visible (timestamps, titles if user chose unencrypted titles, mood scores) |

### Step-up OIDC

`POST /api/export/start` and `POST /api/import/upload` reject ID tokens issued more than 5 minutes ago, forcing the user to re-authenticate. Implementation: gateway checks the `iat` claim and returns 401 with a `WWW-Authenticate: Bearer error="step-up-required"` header. Frontend handles this by initiating a fresh OIDC redirect with `prompt=login`.

### Logging

`docs/deployment/observability.md` already forbids logging diary content. This RFC adds: do not log archive contents, do not log filenames inside the archive, do not log raw user_id (only the hash). Log only: action, user_id_hash, job_id, timestamp, outcome, and (for failures) the failure category.

### Key escrow

Export and import do not introduce any key escrow. The encrypted body files in R2 remain encrypted with the user's existing key. After import on a *new* account, the bodies are unreadable until the user provides their key on the new device. This is a feature: it preserves the E2EE invariant.

A user who has lost their key cannot recover anything. This RFC does not change that. A future RFC might propose optional user-controlled escrow (e.g. recovery-phrase-encrypted key copy), but the absence of escrow is a deliberate part of the threat model and must remain the default.

## Alternatives considered

- **Server-side decryption for export** — Rejected. Breaks the core E2EE invariant. The reason users trust diarilog with sensitive content is that the server cannot read it, and we should not make exceptions even for "convenience".
- **Email the export** — Rejected for the same reason as the file leak threat: email is a separate trust boundary and we have not established that pipeline. Also, body files would still be ciphertext that the email recipient cannot read without the key, defeating the convenience.
- **Cross-instance federation protocol** — Rejected for now. Too much surface area, no clear partner instance, and the zip-archive approach is good enough for the documented user stories.
- **Keep export synchronous forever** — Rejected for users with very long histories. The async-with-job-id design is the right shape; we just allow synchronous for the small-history case in v0.7 to keep the first cut compact.

## Migration / rollout

- **v0.7** — Ship the synchronous export path and the synchronous import path. Document the format. Manual round-trip test required as part of release.
- **v0.8** — Add async export-worker and import-worker once we observe a real user that exceeds synchronous limits, or after `format_version: 1` is stable enough that we are confident in batching.
- **v0.9+** — Consider other-service import adapters as separate, optional features.

There is no migration of existing data; this is purely additive.

## Open questions

- Are diary titles considered sensitive enough to require client-side encryption? Currently they are stored in D1 metadata cleartext. If we want titles to be encrypted, that is a much larger D1 schema change and probably its own RFC. For this RFC, we accept the existing trade-off: titles are exposed in the manifest's `meta.json`.
- Should export include the raw R2 keys (so re-import can preserve them) or new keys (so the imported account does not collide)? Current design: re-import generates new R2 keys but keeps the `body_ref` mapping in `meta.json` as the join key. This means the same archive imported twice into the same account does not duplicate (idempotent on `body_ref`); imported into a new account it gets fresh R2 keys.
- Step-up OIDC requires support from the configured provider. Auth0, Keycloak, and Azure AD support `prompt=login` reliably; Google's behavior is conservative. Implementer should confirm the chain works against the provider matrix in `docs/deployment/oidc-providers.md`.
