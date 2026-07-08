# RFC 010: Emergency erase — gap analysis and UI

| Field | Value |
|---|---|
| Status | Implemented |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |
| Template | Lightweight |

## Summary

Server-side emergency erase is already implemented across gateway, journal, identity, and dialog. The client-side UI to trigger it is not. This RFC documents what exists, identifies the gap, and specifies a one-tap erase UI plus the safeguards that prevent accidental triggering.

## Motivation

The MVP requirement is satisfied at the data layer: every core worker has an `erase_all(user_id)` path, and the gateway has a `/api/erase` endpoint that orchestrates them. From the user's perspective, none of this is reachable. A user in a threatened situation who wants to wipe their data has to find the API endpoint or rely on a separate mechanism (clearing the browser).

The gap is small but the consequences of getting it wrong are large in both directions: too easy to trigger means data loss from a misclick; too hard to trigger means the safety feature does not work when it is needed most.

## Plan

### 1. Inventory the existing server-side implementation

The implementer should first read these files to confirm the state matches this summary; if any divergence is found, this RFC is amended.

- `workers/gateway/src/handlers/erase.rs` — orchestration entry point (`erase_all`)
- `workers/gateway/src/services/storage.rs::erase_gateway_owned` — KV cleanup
- `workers/journal/src/handlers/erase.rs` and `.../storage.rs::erase_all_user_data` — R2 + D1 erasure
- `workers/identity/src/handlers/profile.rs::erase_all` and `.../storage.rs::erase` — D1 erasure
- `workers/dialog/src/handlers/erase.rs` and `.../session.rs::erase` — sessions, drafts

Document any unexpected behaviour. Specifically check:

- Whether `safety` worker has any user-bound state to erase (likely no, since safety logs to observability rather than user records).
- Whether the gateway endpoint is exposed in the routing table or only accessible internally.

### 2. UI for triggering erase

Add a settings page section "Emergency erase" with two-step confirmation:

- Step 1: button labeled "Erase all my data" with explanatory text. Click opens a modal.
- Step 2: modal lists exactly what will be erased (diary entries, sessions, profile, settings) and asks the user to type the literal word `ERASE` (or its localized equivalent — see open questions) into a text input. Submit button disabled until the word matches exactly.

The two-step pattern is a deliberate friction. The threat model is "user under duress wants to wipe data fast" but also "user clicks the wrong button at 3 AM and loses years of journals". Typing a confirmation word satisfies both: it takes seconds, but cannot happen by accident.

After successful erase, the user is signed out and returned to the index page. A short message confirms: "Your data has been erased."

### 3. Optional: panic gesture

A separate "panic erase" mode that does not require typing the confirmation word, intended for genuine duress. Trade-off: faster, but more accidental triggers.

For v0.7, **not implementing the panic mode**. Document it as a v0.8+ consideration. The two-step modal is the only path.

### 4. Reachability while signed out

A user whose primary device is compromised but who can reach a different device should be able to erase remotely after authenticating from the second device. The endpoint already supports this (any authenticated session can call it). The UI just needs to be reachable from the post-login dashboard. Already part of step 2.

### 5. Audit trail

Erase events are logged with `user_id_hash` (per `docs/deployment/observability.md`) and a timestamp, but **no content**. The log entry is a permanent record that an erase happened; it does not reconstruct what was erased.

## Out of scope

- Server-side delayed erase (e.g. 24-hour grace period for "I changed my mind"). The current behaviour is immediate. Adding a grace period is a UX call that intersects with the duress threat model — a delayed erase that an attacker can cancel is worse than no grace period. Decision: keep immediate erase.
- Cryptographic shredding (overwriting with random bytes before delete). R2 deletion is sufficient for the threat model and Cloudflare's underlying storage handles physical shredding asynchronously. Documented for users who ask.
- Backup-from-bug recovery. There is no path back from erase by design. Backup is a user responsibility (RFC 007 export). Documenting this prominently is part of the UI copy.

## Open questions

- The literal confirmation word `ERASE` does not translate one-for-one. Should we accept localized equivalents (`削除`, `محو`, etc.), keep it as the English `ERASE` regardless of UI language (universal codeword), or accept either? Lean toward accepting either (case-insensitive match against a small set per language) to avoid leaving a non-English speaker stranded.
- Should the server require step-up OIDC re-authentication for `/api/erase`, similar to RFC 007? Argument for: catastrophic action; consistent with export. Argument against: a user under duress losing ID-token freshness is exactly when they cannot complete an OIDC redirect. Lean against step-up here, but document the asymmetry with RFC 007.
- Mobile app context (RFC 009): the PWA needs a similarly-friction UI for offline-initiated erase. Offline erase clears local IndexedDB immediately and queues the server erase as an outbox op. Resolved within RFC 009; cross-referenced here.
