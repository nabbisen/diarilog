# RFCs

This directory holds Request-for-Comments documents — the authoritative specifications that describe planned and accepted design decisions for diarilog. RFCs are how design decisions are handed off from architects to implementers without losing intent in chat-history rot.

The process itself is defined in [`001-rfc-process.md`](./001-rfc-process.md). Read that first if you have not seen this directory before.

## Index

| ID | Title | Status | Template | Summary |
|---|---|---|---|---|
| [001](./001-rfc-process.md) | RFC Process | Accepted | meta | Defines this directory, the templates, and the lifecycle. |
| [002](./002-rename-to-diarilog.md) | Rename application to `diarilog` | Implemented | Lightweight | Mostly mechanical rename of legacy `trauma-journal-*` and `__TJP_*` identifiers. |
| [003](./003-reviewed-false-ui-markers.md) | UI markers for unreviewed translations | Proposed | Standard | Surface the `reviewed: false` flag to users so they know which translations are provisional. |
| [004](./004-language-switcher.md) | Language switcher UI and `PUT /api/me` | Proposed | Standard + Security | In-app language picker, persisted server-side for authed users and in a cookie otherwise. |
| [005](./005-translation-review-criteria.md) | Translation review process and acceptance criteria | Proposed | Standard (operational) | Acceptance criteria and governance for flipping `reviewed: false` → `true`. |
| [006](./006-rtl-layout-arabic-font.md) | RTL layout and Arabic font delivery | Proposed | Standard | CSS logical properties + self-hosted Noto Sans Arabic subset. |
| [007](./007-data-export-import.md) | User-controlled data export and import | Proposed | Full | E2EE-preserving zip archive format and round-trip flow. |
| [008](./008-cicd-github-actions.md) | CI/CD with GitHub Actions | Implemented | Standard + Security | PR validation, staging-on-merge, manual production via workflow_dispatch. |
| [009](./009-offline-pwa.md) | Offline support (PWA, Service Worker, sync) | Proposed | Full | Multi-release plan for offline-first journaling. |
| [0010](./0010-emergency-erase-ui.md) | Emergency erase — gap analysis and UI | Implemented | Lightweight | Server-side already implemented; this RFC adds the UI and confirmation flow. |
| [0011](./0011-e2ee-boundary-and-key-model.md) | Encryption boundary, key derivation, and multi-device access | Partial | Full | Server-side types and schema done. Browser-side crypto (Argon2id + AES-GCM) not yet implemented. |
| [0012](./0012-entry-versioning.md) | Entry edit history and immutability | Implemented | Standard | Each edit creates a new version; original always preserved; 20-version cap. |

## How RFCs relate to the ROADMAP

`README.md` ROADMAP entries that have moved into "now planning" status get an RFC. ROADMAP entries that are still ideation (Phase 3 Workers, etc.) do not yet have RFCs. When an RFC ships, the ROADMAP item is checked off and a CHANGELOG entry is added; the RFC itself stays put for historical reference.

## Recommended reading order for a new implementer

If you are picking up implementation work, start here:

1. [`001`](./001-rfc-process.md) — what RFCs are and how to navigate them.
2. The repository [`README.md`](../README.md) ROADMAP for the bigger picture.
3. [`docs/deployment/`](../docs/deployment/) for how production lives.
4. The specific RFC for the work item assigned to you.

## Recommended ordering for implementation

The RFCs are not strict prerequisites of each other but there is a sensible order:

```
001 ✅ accepted
002 ✅ implemented (rename)
008 ✅ implemented (CI/CD)
0010 ✅ implemented (erase UI)
0011 🔶 partial   (server-side done; browser crypto remaining)
0012 ✅ implemented (entry versioning)

Next:
 ├─ 003, 004, 006 ─ frontend cluster, can ship in parallel
 ├─ 005 ─ operational (translation review process)
 ├─ 0011 remainder ─ browser-side Argon2id + AES-GCM
 ├─ 007 ─ export/import (depends on 0011 browser crypto)
 └─ 009 ─ offline PWA, multi-release
```
