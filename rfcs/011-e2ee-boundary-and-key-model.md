# RFC 011: Encryption boundary, key derivation, and multi-device access

| Field | Value |
|---|---|
| Status | Partial |
| Author | nabbisen |
| Created | 2026-05-22 |
| Last updated | 2026-05-22 |
| Template | Full |
| Supersedes | (none, but resolves open questions in RFC 007 and RFC 009) |
| Implementation note | Server-side complete: `packages/crypto` types, `UserRecord` / `DiaryMeta` schema fields, D1 migrations, onboarding page and route. Browser-side crypto (Argon2id key derivation, AES-GCM encrypt/decrypt in WASM) not yet implemented. |

## Summary

Define exactly what "end-to-end encryption" means in diarilog: which fields are encrypted, which are metadata visible to the server, how the encryption key is derived from the user's passphrase, how the same user can access their data on multiple devices, and what happens when the passphrase changes. Resolves several gaps that External Design v2.1 surfaced.

## Background

The Requirements Specification states (R-PRIV-1): journal bodies, dialogue logs, and sensitive user settings are encrypted on the device; no key escrow. External Design v2 §4.1 lists a data model in which `UserRecord` (id, display_name, language, created_at) and `DiaryEntry` metadata (id, title, created_at, mood_score, body_ref) are stored in D1, while only the body lives in R2 as ciphertext.

The encryption boundary is currently implicit. "Sensitive user settings" is not defined. Titles, mood scores, language preference, and display name are stored cleartext in D1, which means a server-side compromise reveals them. For a trauma-care service where a diary title might be "I can't anymore" and a mood score might trace a deterioration over weeks, the gap matters.

A second gap: there is no defined story for how a user signs in on a second device and decrypts their existing data. The "no key escrow" position is incompatible with naive "same passphrase produces same key" if we want any resistance to offline attacks, and it is incompatible with "server holds a hint" if we take the no-escrow principle literally.

A third gap: RFC 009 places the passphrase-unlock UX in its v2.9 milestone (the third installment of offline implementation), but External Design v2.1 §3.2 puts passphrase setup in first-session onboarding at v2.6. The passphrase model has to exist before either can ship.

This RFC resolves all three.

## Requirements

### Boundary

1. **R1.** The system defines two classes of journal-side data: **content** (always client-side encrypted) and **metadata** (server-visible). The boundary is fixed by this RFC; future additions must be classified explicitly.
2. **R2.** Content includes: diary body, diary title, interview turn text (questions and answers), saved draft text, and any trigger word list the user has entered.
3. **R3.** Metadata includes: user id, hashed external identifier from OIDC, account creation timestamp, language preference, display name (if set; users may leave it blank), per-entry created_at and updated_at timestamps, body_ref (the R2 object key, which is itself a random identifier), mood_score *(see open question)*, and operational flags like `onboarding_completed`.
4. **R4.** The trust model documentation visible to users (on the index and About pages) must accurately describe what is and is not encrypted. We do not market "everything is encrypted" if the server sees titles and timestamps.

### Key derivation

5. **R5.** The encryption key used for content is derived from a passphrase the user supplies. The server never sees the passphrase or the derived key.
6. **R6.** Key derivation uses Argon2id with parameters appropriate for low-end mobile devices (target: 250 ms on a 2020-era mid-range phone), with a per-user salt.
7. **R7.** The salt is generated at first passphrase setup, stored in `UserRecord.kdf_salt`, and is **not secret**. Storing the salt server-side does not violate the no-escrow principle because the salt alone cannot derive the key.
8. **R8.** A passphrase-derived key encrypts a **separately generated, random data encryption key (DEK)** rather than encrypting content directly. The DEK is stored in `UserRecord.wrapped_dek` (ciphertext). This indirection makes passphrase change feasible without re-encrypting all content (see R12).

### Multi-device access

9. **R9.** A user signing in on a second device, after OIDC authentication, is prompted for their passphrase. The salt and wrapped DEK are fetched from the server; the passphrase derives the key-encryption key (KEK); the KEK unwraps the DEK; the DEK decrypts content.
10. **R10.** No additional server-stored "hint," "recovery code," or "verification token" is held. The server can confirm that the unwrap succeeded (because the unwrapped DEK plaintext has a known format with a magic prefix), but cannot itself perform the unwrap.

### Passphrase change

11. **R11.** A user may change their passphrase. The flow re-derives a new KEK from the new passphrase (using a fresh salt), re-wraps the existing DEK with the new KEK, and updates `UserRecord.kdf_salt` and `UserRecord.wrapped_dek` atomically.
12. **R12.** The content (R2 bodies, D1 metadata fields that are encrypted) is **not** re-encrypted during passphrase change. Only the DEK wrapping changes. This keeps passphrase change O(1) rather than O(entries).

### Session lifetime

13. **R13.** Once unwrapped, the DEK lives in browser memory for the active session only. Closing the tab or signing out discards it. The DEK is never written to localStorage or sessionStorage.
14. **R14.** The Service Worker (RFC 009) may hold the DEK in worker-scoped memory for the SW lifetime, to allow background encryption of outbox writes while the user is on another tab. The DEK is discarded when the SW is terminated by the browser.

### Onboarding completion state

15. **R15.** `UserRecord.onboarding_completed: bool` is set to `false` at account creation. It transitions to `true` only after the user has set a passphrase and confirmed it. The OIDC callback redirects to onboarding instead of dashboard whenever this flag is false.
16. **R16.** Onboarding is resumable. A user who closes the browser between OIDC sign-in and passphrase setup returns to the passphrase step on next sign-in. No content can be written until passphrase setup is complete.

## Design

### Data model additions to `UserRecord`

```rust
pub struct UserRecord {
    pub id: String,
    pub display_name: Option<String>,        // metadata, may be empty
    pub language: String,
    pub created_at: DateTime<Utc>,
    pub onboarding_completed: bool,          // NEW
    pub kdf_salt: Vec<u8>,                   // NEW: 16 bytes, base64 in JSON
    pub wrapped_dek: Vec<u8>,                // NEW: AES-GCM-wrapped DEK, base64 in JSON
    pub kdf_params: KdfParams,               // NEW: Argon2 cost params (versioned)
}

pub struct KdfParams {
    pub algorithm: KdfAlgorithm,             // Argon2id only, but versioned for future
    pub memory_kib: u32,                     // e.g. 19 MiB
    pub iterations: u32,                     // e.g. 2
    pub parallelism: u32,                    // e.g. 1
}
```

Old `UserRecord` rows (from before this RFC ships) have `onboarding_completed = false`, no salt, no wrapped DEK. The migration treats them as if they were new accounts whose first sign-in after the upgrade triggers onboarding. **This means existing users (if any) will be asked to set up a passphrase on next sign-in.** Since the system has no production users yet, this is acceptable; if it did, a migration RFC would be needed.

### Diary entry data model

```rust
pub struct DiaryEntry {
    pub id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub body_ref: String,                    // R2 object key (random)
    pub encrypted_title: Vec<u8>,            // NEW: AES-GCM ciphertext of title, with nonce prepended
    pub encrypted_mood: Option<Vec<u8>>,     // NEW: optional encrypted mood score
}
```

`encrypted_title` carries a 12-byte nonce followed by AES-256-GCM ciphertext. The DEK is the key. Empty title is encoded as encryption of an empty string (still produces a non-empty ciphertext due to the auth tag).

`encrypted_mood` is optional: users may or may not record a mood score per entry.

The R2 body object is similarly nonce-prepended AES-GCM ciphertext, keyed by the DEK. The R2 path itself is unchanged.

### Why GCM rather than a more modern AEAD

AES-256-GCM is hardware-accelerated on essentially every device built since 2015. ChaCha20-Poly1305 would be roughly equivalent in security with slightly better software-only performance. Either is acceptable. AES-GCM wins on broad library support — `aes-gcm` is in the Rust crypto ecosystem with WASM compatibility verified. A future RFC may switch; the `KdfParams.algorithm` versioning extends to the symmetric algorithm as well (a parallel `ContentEncryptionAlgorithm` field, not added now to avoid premature complexity).

### Server-visible aggregation impact

`/api/dashboard` currently returns `recent_diaries` with `title` and `mood_score`. After this RFC, the server returns `encrypted_title` and `encrypted_mood`; the client decrypts with the in-memory DEK before rendering.

The aggregation API itself does not change shape, only the field types. The `body_ref` field continues to be metadata (just a random R2 key).

This means a user who has not yet unwrapped their DEK in the current session cannot see entry titles even after the dashboard responds. The dashboard renders a placeholder until the unlock completes:

```
[ Recent entries ]
  ⚪ (locked entry) — 2026-05-21
  ⚪ (locked entry) — 2026-05-19
  ...

  Enter your passphrase to view your entries →
```

Once the passphrase is entered and the DEK is unwrapped, the titles fill in.

### First-session onboarding (resolving the v2.1 §3.2 gap)

```
[OIDC callback]
   │
   ▼
[Server: load UserRecord. onboarding_completed?]
   │
   ├─ true  → /dashboard (normal flow)
   └─ false → /onboarding/passphrase
              │
              ▼
        [User chooses a passphrase, re-enters to confirm]
              │
              ▼
        [Client: generate DEK (32 random bytes), derive KEK from passphrase + new salt,
                 wrap DEK with KEK, send wrapped_dek + salt + kdf_params to server]
              │
              ▼
        [Server: store on UserRecord, set onboarding_completed = true]
              │
              ▼
        [Continue to language confirmation, then dashboard]
```

If the user closes the tab after OIDC but before passphrase setup, the next sign-in routes them back to `/onboarding/passphrase`. The account exists but cannot be used to write anything; the dashboard is unreachable until onboarding completes.

### Subsequent-session unlock

```
[Sign-in via OIDC]
   │
   ▼
[Server: load UserRecord, return salt + wrapped_dek + kdf_params with the session]
   │
   ▼
[Client: prompt for passphrase]
   │
   ▼
[Client: derive KEK from passphrase + salt, attempt to unwrap DEK]
   │
   ├─ Unwrap succeeds (auth tag valid, DEK plaintext has expected magic) → in-memory DEK ready
   └─ Unwrap fails → "Passphrase did not match" — retry, no server round-trip needed
```

The server confirms nothing about passphrase correctness; only the client knows. This is the point of the no-escrow design.

### Passphrase change

```
[Settings → Change passphrase]
   │
   ▼
[Client: enter current passphrase, derive current KEK, unwrap existing DEK]
   │
   ▼
[Client: enter new passphrase twice, generate new salt, derive new KEK,
         wrap the same DEK with the new KEK]
   │
   ▼
[POST /api/me/passphrase  { new_salt, new_wrapped_dek, new_kdf_params }]
   │
   ▼
[Server: atomic UPDATE UserRecord SET kdf_salt=?, wrapped_dek=?, kdf_params=?]
   │
   ▼
[Done — content is unchanged on disk, only the wrapper changed]
```

If the server-side UPDATE fails, the client retries; the old wrapper is still valid until the new one replaces it. No window where the user is locked out.

### Why DEK indirection rather than direct key derivation

A simpler design would derive the content encryption key directly from the passphrase and use it on R2 ciphertext. Passphrase change in that design requires re-encrypting every R2 object and every D1 ciphertext field. For a user with thousands of entries this is a problem. The DEK indirection keeps passphrase change cheap and also leaves room for future features (e.g. per-entry sub-keys, key rotation) without breaking the model.

### Session-key handling in offline mode

RFC 009 §5 already names the right mechanism: the DEK lives in Service Worker memory for the SW lifetime. When the user closes all tabs and the SW is later terminated by the browser, the DEK is gone. Next session requires passphrase entry again.

For PWA users who keep the app open across many days, the DEK persists across days as long as the SW does. This is a trade-off — a long-lived DEK on a stolen unlocked device is at risk. A future RFC may add inactivity-based DEK eviction (e.g. clear after 4 hours of no input) but this RFC does not require it.

### Backup phrase: explicitly rejected

A standard mitigation for lost passphrases is to print a "recovery phrase" at setup that the user writes down somewhere safe. Effectively this is a second passphrase, derived to a second wrapped DEK.

This RFC **rejects** the recovery phrase pattern. Reasons:

- It produces a second long-lived secret that the user is told is "in case you lose the other one" — but most users treat it identically to the original passphrase, storing it in the same place. The actual recovery rate in practice is low.
- It creates a false sense of security: the user feels protected against loss without actually being so.
- A user under coercion may be forced to disclose both. Having two equally-powerful secrets doubles the attack surface against forced disclosure.

The Requirements Specification's position (no key escrow, lost passphrase means lost data) is preserved. The honest framing is what gets communicated to the user during setup.

### What the user is told at setup

The passphrase setup screen says:

> Choose a passphrase. We will use this passphrase to encrypt your journal so that no one, not even us, can read it. If you lose this passphrase, your data cannot be recovered. There is no reset link. There is no support recovery. Please write your passphrase down somewhere safe before continuing.

Then a separate confirmation step: the user is asked to enter the passphrase a second time, **after** an interstitial that requires them to wait 60 seconds and re-derive it from memory rather than from clipboard. This is unusual but it catches users who would otherwise paste a generated passphrase, lose the clipboard contents, and not realize until weeks later. The "60-second wait" is intentional friction. A user who cannot survive 60 seconds without seeing their passphrase will likely lose it.

If this UX detail proves too obstructive in real testing, a follow-up RFC can adjust. The default is the high-friction version.

### How the trust model copy must change

The index page's three-bullet trust model currently reads (per External Design v2.1 §5.1):

> end-to-end encryption, no ads, you own your data

After this RFC, the wording must be honest about the boundary:

> The content of your entries is encrypted on your device. We can see when you wrote, what language you wrote in, and a display name if you set one. We cannot read your entries themselves. No ads. You can export and erase your data at any time.

The shorter version may live in the hero; the longer version on the About page.

## Test plan

- **Unit (web-app crypto module).** Round-trip: encrypt and decrypt with a random DEK; verify ciphertext is non-deterministic (different nonces on repeat); verify auth tag rejects tampered ciphertext.
- **Unit (web-app KDF).** Argon2id derivation produces stable output for stable input; varies with salt; passes the cost-parameter target (verifiable by measuring real wall-clock time, with a wide tolerance to allow for slow CI runners).
- **Unit (web-app DEK wrap/unwrap).** Wrap-then-unwrap is identity; wrong KEK produces unwrap failure (auth tag).
- **Unit (web-app DEK validation).** A successfully unwrapped DEK begins with a known 4-byte magic; this lets the client distinguish "passphrase right but corrupted DEK" from "passphrase wrong." Test both cases.
- **Unit (web-app onboarding state machine).** First-session flow proceeds through passphrase → language → dashboard. Resume from passphrase step works if interrupted.
- **Unit (identity worker).** `onboarding_completed`, `kdf_salt`, `wrapped_dek`, `kdf_params` round-trip through D1 correctly.
- **Integration (synthetic).** Onboard a synthetic user, write an encrypted entry, sign out, sign in again, decrypt entry, verify content matches.
- **Negative.** Tampered `wrapped_dek` in `UserRecord` produces unwrap failure on next unlock; entry remains opaque on the server side.
- **Migration test.** A pre-RFC `UserRecord` (no salt, no wrapped_dek, `onboarding_completed = false`) routes correctly to onboarding on next sign-in.

Expected new test count: **+16 unit tests**, plus integration scaffolding.

## Security considerations

### What is encrypted, what is not

After this RFC:

| Field | Encrypted? | Notes |
|---|---|---|
| Diary body | yes (AES-256-GCM) | In R2 |
| Diary title | yes | In D1, ciphertext |
| Mood score | yes (optional field) | In D1, ciphertext if present |
| Interview turn text (Q and A) | yes | In R2 or D1 depending on size |
| Saved draft text | yes | In D1, ciphertext |
| Trigger word list | yes | In D1, ciphertext |
| User id | no (random identifier) | In D1 |
| OIDC subject hash | no | In D1, hashed already |
| Display name | no (optional, blank by default) | In D1 |
| Language preference | no | In D1 — used for SSR language routing |
| Created_at / updated_at | no | In D1 |
| `body_ref` | no (random R2 key) | In D1 |
| `kdf_salt`, `wrapped_dek`, `kdf_params` | partial | These are the wrapped DEK and its salt; salt is not secret, wrapped DEK is not useful without the passphrase |
| `onboarding_completed` | no | In D1 |

### Threat model

- **Server compromise (read-only).** Attacker reads D1, R2, KV. They learn: user ids, timestamps, language preferences, display names, and the wrapped DEKs + salts (which let them try offline passphrase attacks). They do not learn: entry content, titles, mood scores, drafts, triggers.
- **Server compromise (write).** Attacker writes to D1 and R2. They can replace a user's wrapped DEK, locking the user out (denial of service). They cannot read existing content. They can write new ciphertext objects but cannot make them decrypt to attacker-chosen plaintext under the user's key.
- **Offline attack on stolen wrapped DEK + salt.** Attacker has the salt and the wrapped DEK and brute-forces the passphrase. Argon2id with the cost parameters in R6 makes this expensive (target: 250 ms per attempt on a phone, several seconds on a beefy GPU). A passphrase below 5 random words is at meaningful risk; the setup UI enforces minimum strength (see open question on the strength check).
- **Coerced disclosure.** User is forced to enter passphrase. Attacker reads all content. This RFC does not mitigate (no design can — the user has the key). The emergency-erase flow (RFC 010) and the offline mode (RFC 009 IndexedDB encryption) are the partial mitigations.
- **Stolen unlocked device.** Attacker has the DEK in browser memory. This RFC does not mitigate beyond the session-lifetime rule (R13). A future RFC may add inactivity timeout.

### What the server can still learn from metadata

Even with content encrypted, an attacker reading D1 learns:

- When the user signed up
- When the user last wrote
- How frequently the user writes (each entry's `created_at`)
- What language the user writes in
- The user's display name (if any)
- Whether the user has an active interview session

For a determined adversary doing traffic analysis, this is meaningful. We do not claim to defeat traffic analysis. The trust model copy says "we can see when you wrote, what language you wrote in" — this is honest.

A future RFC may explore reducing metadata exposure (e.g. encrypting `created_at` at day granularity rather than second granularity, or storing `language` in `wrapped_dek` so the server cannot route by language; this conflicts with SSR routing and would need careful design).

### Why the server still routes by language

Language is metadata because SSR (in bff) selects the translation file based on `UserRecord.language`. Encrypting language would either force language selection client-side after decryption (re-render after unlock — visible flicker, breaks the "JS-disabled basic readability" property) or require language selection from `Accept-Language` only (which we already do for anonymous users, but logged-in users have an explicit preference).

We accept this trade-off and document it.

## Alternatives considered

- **No encryption beyond what we have today.** The current design encrypts only the body. Rejected because the Requirements Specification's E2EE language is misleading without title encryption, given how revealing a title can be in this domain.
- **Encrypt everything including metadata.** Would require client-side rendering of dates and language selection, breaking SSR and JS-disabled accessibility. Rejected.
- **Direct passphrase-to-content key derivation, no DEK indirection.** Simpler but makes passphrase change O(content). Rejected.
- **Server-side recovery phrase / escrow.** Provides recovery at the cost of the no-escrow principle. Rejected (see "Backup phrase: explicitly rejected" above).
- **Hardware-backed keys (WebAuthn, Passkeys).** Tempting but introduces a hard dependency on platform support that varies widely across the populations diarilog targets. A user on a low-end Android in a refugee camp may not have a working hardware authenticator. Deferred — perhaps a future RFC offers Passkeys as an optional alternative for users whose devices support it.
- **PBKDF2 instead of Argon2id.** PBKDF2 is more universally available but weaker against GPU/ASIC attacks. Argon2id is available in WASM-compatible Rust crates (`argon2` works). Argon2id is the right call for new designs.

## Migration / rollout

### Sequencing relative to other RFCs

This RFC must ship **before** RFC 007 (export) flips to encrypting titles in the archive, and **before** RFC 009 v0.10 (which depends on the passphrase model for its IndexedDB encryption). Recommended order:

1. RFC 011 (this RFC) — v0.7
2. RFC 009 v0.8 (Service Worker, app shell) — can ship in parallel since SW does not need the DEK yet
3. RFC 007 (export) — depends on RFC 011 to know what archive ciphertext to include
4. RFC 009 v0.10 (IndexedDB encryption) — depends on RFC 011

### Migration of existing users

No production users exist yet, so the migration is trivial: all existing rows have `onboarding_completed = false` after the schema migration, and the next sign-in routes them to passphrase setup.

If this RFC is delayed past production launch, a migration would be needed: existing users would need to be prompted to set up a passphrase on next sign-in, and their existing cleartext titles/mood scores would be encrypted at that point. The migration RFC would address whether to leave the cleartext fields in place for compatibility or to delete them after encryption.

### Schema migration

A new D1 migration:

```sql
ALTER TABLE users ADD COLUMN onboarding_completed BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN kdf_salt BLOB;
ALTER TABLE users ADD COLUMN wrapped_dek BLOB;
ALTER TABLE users ADD COLUMN kdf_params_json TEXT;

ALTER TABLE diaries ADD COLUMN encrypted_title BLOB;
ALTER TABLE diaries ADD COLUMN encrypted_mood BLOB;
-- existing title and mood_score columns retained for one release for safety,
-- removed in the release that follows once migration is verified
```

### Documentation updates

After this RFC ships:

- **Requirements Specification §4.3 R-PRIV-1** is amended to name the encryption boundary explicitly (link to this RFC for the table).
- **External Design v2 §4** is amended with the new fields and the metadata-vs-content distinction.
- **External Design v2.1 §3.2** is amended to align with the onboarding flow described here.
- **External Design v2.1 §5.1** is amended with the more honest trust-model copy.

## Open questions

- **Argon2id cost parameters.** R6 names 250 ms on a 2020-era mid-range phone as the target. The exact parameters (memory, iterations, parallelism) should be tuned during implementation. A wider review by someone with crypto-implementation experience is welcome before merge.
- **Passphrase strength minimum.** Not specified in this RFC. Recommend zxcvbn or similar with a minimum score of 3, but the exact threshold is implementation-time. Setting it too high frustrates users; too low gives weak protection against offline attacks.
- **Mood score encryption: necessary?** Mood scores are integers 1–5. They carry less semantic content than titles. An attacker who learns "user mood was 1 on Tuesday" learns something but not as much as learning a title. The cost of encryption is small. Default: encrypt them. Open to discussion if there's a use case (anonymized aggregate stats? But those would require server-readable mood scores, which conflicts).
- **Display name: encrypt?** Currently cleartext per R3. A user might set their display name to something sensitive ("Survivor"). Argument for: respect the user's choice. Argument against: dashboard rendering, SSR personalization (e.g. "Welcome, X") becomes client-side-only. Default: cleartext. Open to revision.
- **Per-entry sub-keys.** A future feature might be "tags" or "categories" that the user sets per entry. Should tags be encrypted? Probably yes, by the same DEK. Out of scope here.
- **The 60-second wait at passphrase setup.** Unusual. If usability testing shows it pushes users away rather than reinforcing the seriousness, swap for a different friction mechanism (e.g. re-enter passphrase after navigating away and back). Open.
- **Web Crypto API vs Rust crypto crates.** The browser's `crypto.subtle` provides AES-GCM and PBKDF2 but not Argon2id. We will need a WASM Argon2id (from the `argon2` crate) and may want to use `crypto.subtle` for AES-GCM for performance. The mix-and-match adds complexity; an alternative is "all in WASM" using `aes-gcm` from RustCrypto. Defer to implementation, but note the choice in the implementation PR.
