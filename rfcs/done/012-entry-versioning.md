# RFC 012: Entry edit history and immutability

| Field | Value |
|---|---|
| Status | Implemented |
| Author | nabbisen |
| Created | 2026-05-22 |
| Last updated | 2026-05-22 |
| Template | Standard |

## Summary

Decide how diarilog handles the act of editing a previously-saved diary entry: does the new version overwrite the old, do we retain history, and if we retain it, how is it surfaced to the user. The default position taken by this RFC is **retain history server-side, do not surface it as a normal feature, expose it only through explicit "see earlier versions" navigation and through the export archive**.

## Motivation

External Design v2.1 §5.5 says editing past entries is "allowed and even encouraged" — updating one's narrative is part of processing trauma. But neither the previous documents nor the implementation specify whether editing **overwrites** the prior text or **appends** a new version.

The choice is not cosmetic. Two scenarios point in opposite directions:

- A user edits a years-old entry during a difficult moment, rewriting it in a worse state, and later wants to see what they originally wrote. Overwrite loses that.
- A user wants to leave a difficult past behind by literally rewriting it, and feels that visible history works against the therapeutic intent. Append-only undermines that.

These pull against each other. We need to pick a default and surface it correctly to users.

## Requirements

1. **R1.** When a user saves an edit to an existing entry, the system preserves the prior version. The visible "current" entry is the latest edit. Older versions are not deleted unless the user explicitly removes them (R5) or erases the account (R6).
2. **R2.** The default reading experience shows only the current version. A user reading their own history does not see "edited from earlier text" unless they explicitly navigate to version history.
3. **R3.** Each version carries its own `created_at` (the original entry's creation time, propagated) and `edited_at` (the time of that specific edit). The very first version's `edited_at` equals `created_at`.
4. **R4.** Editing an entry does not retrigger crisis detection on the previous version. Detection runs against the new text only.
5. **R5.** The user can explicitly delete prior versions of a single entry through a "see earlier versions" surface (see §Design). This is a per-version delete, not a per-entry delete; deleting all versions of an entry is equivalent to deleting the entry.
6. **R6.** Emergency erase (RFC 010) deletes all versions of all entries, server- and client-side.
7. **R7.** The export archive (RFC 007) includes all versions, ordered, so that a user who values their full history can carry it with them on leaving.
8. **R8 (non-functional).** Version retention does not unboundedly grow R2 costs. A soft cap of 20 versions per entry is set; beyond that, the oldest non-original version is dropped (the very first version is always retained). The cap is generous enough to never be reached in typical use.

## Design

### Data model

A new table in D1:

```sql
CREATE TABLE diary_versions (
    id TEXT PRIMARY KEY,                     -- unique id for the version
    diary_id TEXT NOT NULL,                  -- foreign key to diaries.id
    version_number INTEGER NOT NULL,         -- 1, 2, 3, ... incremented per edit
    edited_at DATETIME NOT NULL,             -- when this version was saved
    body_ref TEXT NOT NULL,                  -- R2 key for this version's ciphertext body
    encrypted_title BLOB NOT NULL,           -- this version's title (per RFC 011)
    encrypted_mood BLOB,                     -- this version's mood, if any
    FOREIGN KEY (diary_id) REFERENCES diaries(id) ON DELETE CASCADE
);

CREATE INDEX idx_diary_versions_diary ON diary_versions(diary_id, version_number);
```

The existing `diaries` table is kept and represents the **current** state: the `body_ref`, `encrypted_title`, `encrypted_mood` columns continue to mirror the latest version for read-time efficiency. The `updated_at` column tracks the latest edit time. The `diary_versions` table records the full history.

This redundancy is deliberate: the dashboard read path stays simple (one row per entry from `diaries`), and the history view does a secondary lookup only when the user asks for it.

### Write path

When a user saves an edit:

1. The client encrypts the new body, title, and mood with the DEK as usual.
2. The client `PUT`s to `/api/diary/<id>`. The bff worker, via Service Bindings to journal-worker, performs:
   - Insert a new `diary_versions` row with the next version number, the new ciphertext body uploaded to R2 with a fresh `body_ref`.
   - Update the `diaries` row to point to the new `body_ref`, `encrypted_title`, `encrypted_mood`, and bump `updated_at`.
   - If the version count for this entry now exceeds 20, delete the oldest non-original version (both the R2 object and the row).
3. The response is the new current entry, identical in shape to a regular `GET`.

Both steps (1) and (2) happen in a single D1 transaction. R2 writes are not transactional with D1; we accept that a partial failure leaves an orphan ciphertext in R2 that the cleanup process described in §Operational handling will sweep up.

### Read path

`GET /api/diary/<id>` returns the current entry — no change from today.

`GET /api/diary/<id>/versions` is **new**: returns a list of versions, each with `version_number`, `edited_at`, `encrypted_title`, but **not** the body. Bodies are fetched per-version on demand via `GET /api/diary/<id>/versions/<version_number>`.

The two-step pattern (list, then fetch on demand) avoids paying R2 bandwidth for versions the user does not actually open.

### UI surface

In External Design v2.1 §5.5 (Entry detail view), add a small inline footer:

```
Last edited: 2026-05-19 14:32                                   [ See earlier versions ]
```

"See earlier versions" is **a link, not a primary action**, deliberately understated. It opens a new view:

```
Earlier versions of "Today's entry"
─────────────────────────────────────
  Current  · 2026-05-19 14:32     [ open ]
  v2       · 2026-05-12 09:15     [ open ]  [ delete ]
  Original · 2026-05-10 22:48     [ open ]
─────────────────────────────────────
```

The original (version 1) cannot be deleted as long as later versions exist; this preserves the user's first writing as a baseline. If the user deletes all later versions, the original becomes the current and is then deletable through the regular per-entry delete flow.

Opening a prior version shows the body read-only with a banner:

```
You are viewing an earlier version from 2026-05-10. To edit, return to the current version.
```

There is no "restore this version" button. A user who wants to bring back earlier text can copy it and paste it into a new edit. The reason is design integrity: a one-click "restore" makes the system feel like a content management system rather than a journal. Restoring text from years ago should be a deliberate act.

### Trauma-care framing

The visibility of edit history is gentle but honest. Two design choices reflect that:

- **Earlier versions are reachable but not announced.** The dashboard does not say "you edited this last week." The "See earlier versions" link is in the entry detail view only, in small type at the bottom. A user who wants to forget that they edited is not reminded.
- **Original version is protected.** A user under coercion who is forced to "clean up" their journal cannot accidentally delete the original. They would have to delete each later version and then the original separately. This is friction in the right direction.

### Interaction with offline (RFC 009)

The Service Worker outbox enqueues edits as full new versions, not deltas. On reconnect, each enqueued edit becomes a new version in the order it was queued. The conflict resolution from RFC 009 (server wins, losing copy becomes a separate entry) still applies at the entry level; per-version conflict resolution is not needed because each device's edit becomes its own version on the server.

### Interaction with the encryption model (RFC 011)

Each version's title, mood, and body are independently encrypted with the same DEK. Passphrase change (RFC 011 R11) does not require re-encrypting versions because only the DEK wrapping changes; the DEK that decrypts version 1 is the same DEK that decrypts version 5.

### Operational handling

**Orphan R2 objects.** If a D1 transaction succeeds but the R2 cleanup of the dropped 21st-oldest-version fails, the R2 object becomes orphaned. A weekly maintenance worker (out of scope for this RFC, planned alongside RFC 007's job-cleanup worker) sweeps orphaned objects by listing all R2 keys under a prefix and checking which ones are still referenced in D1.

**Storage cost.** 20 versions per entry, average entry ~5 KiB encrypted, average user ~300 entries → ~30 MiB per user. Acceptable. A heavy editor reaching the 20-version cap repeatedly might use somewhat more, but the cap bounds it. Pricing in R2 puts this well within the free tier even for thousands of users.

### What we do not do

- **No diff visualization.** Showing "this word was added, this word was removed" between versions is intrusive in a journaling context. The user sees full text per version, side by side at most, and forms their own comparison.
- **No automatic version naming.** Versions are numbered, not titled. The user does not write a "commit message" for each edit.
- **No collaboration features.** Diarilog is single-user-owned. Versions exist for the user's relationship with their own writing, not for shared editing.
- **No timed retention.** Versions persist forever (subject to R8 cap), not "deleted after 30 days." A user who wants to delete an old version does so explicitly.

## Test plan

- **Unit (journal-worker).** Save edit → new version row created, current row updated, version count incremented. Hit the 20-version cap → oldest non-original deleted.
- **Unit (journal-worker).** Delete a version → row removed, R2 object scheduled for deletion. Delete the version that the `diaries` row points to (current) → behavior: prevent this in API; the user must edit forward, not delete current.
- **Unit (journal-worker).** Per-entry version listing returns versions in descending version_number order, without bodies.
- **Unit (web-app).** Entry detail view shows "Last edited" footer with the right timestamp. Footer is absent when version_count == 1.
- **Unit (web-app).** "See earlier versions" view renders the version list correctly, including the "Original" label on version 1 and the disabled delete button when only the original remains.
- **Integration (synthetic).** Edit an entry 25 times; verify the 20-version cap is enforced and the original is preserved.
- **Integration (synthetic).** Edit offline, then come online → version is created correctly on sync.
- **Integration (export).** Export includes all versions in the archive; manifest counts match D1.
- **Erase test.** Emergency erase deletes all versions of all entries, both D1 rows and R2 objects.

Expected new test count: **+12 unit and integration tests**.

## Migration / rollout

The change is additive to the schema (new `diary_versions` table, new endpoints). Existing entries are treated as a single-version entry: at first read after the schema migration, the journal-worker checks whether a `diary_versions` row exists for the entry; if not, it backfills version 1 from the current `diaries` row. The backfill happens lazily (on read) rather than as a batch migration, so cold entries pay no migration cost until accessed.

Since the system has no production users yet, the backfill is essentially a no-op. If production users exist by the time this ships, the lazy backfill handles them gracefully.

### Sequencing

This RFC depends on RFC 011 (encryption model) for the `encrypted_title` and `encrypted_mood` field types in `diary_versions`. RFC 011 should ship first or simultaneously. If RFC 012 ships before RFC 011, the new fields would temporarily be cleartext and would need to be migrated when RFC 011 lands — possible but adds work.

Recommended order: RFC 011 first, then RFC 012.

## Open questions

- **R8 cap value.** 20 versions per entry was chosen as "generous enough to never be reached in typical use." If a user is genuinely editing the same entry 20+ times, that may be a usage pattern we should support without capping. Open to raising to 50 or removing the cap. Storage cost is the only argument for the cap.
- **Per-version delete vs entry-level delete.** R5 lets users delete individual versions. Some users may want a simpler model: "delete this entry's history entirely, keeping only the current version." That is achievable by deleting versions one by one, but a single "clear history" action would be friendlier. Decision needed at implementation time.
- **Should the version count be visible on the dashboard entry list?** Currently no — only on the entry detail view. Showing "(edited 3 times)" in the list could be useful for users wanting to revisit edited entries, but is also a small piece of social pressure in the other direction ("this entry has been edited a lot, must be important"). Default: not shown. Open.
- **Crisis detection on edit.** R4 says detection runs only against the new text. But what if a user *removes* concerning content from an entry on edit — should the prior version still be flagged in some way (e.g. a one-time check during the next regular write session, after the user has had time)? This is delicate. For now: no special handling. The detection runs at write time only. If we observe in operation that this leads to missed signals, a future RFC may revisit.
