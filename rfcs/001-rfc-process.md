# RFC 001: RFC Process

| Field | Value |
|---|---|
| Status | Accepted |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |

## Summary

This RFC defines the RFC (Request for Comments) process used in this repository. RFCs are the authoritative specification documents that hand off design decisions from architects to implementers.

## Motivation

The project has reached a point where design decisions are no longer the bottleneck — execution capacity is. Several themes from the ROADMAP need to land in code, and those themes require shared agreement on requirements before someone starts typing. RFCs make that agreement explicit and durable, so an implementer joining mid-stream does not have to reconstruct intent from scattered chat history.

## Where RFCs live

All RFCs live under `rfcs/` at the repository root, named `NNNN-short-slug.md` where `NNNN` is a four-digit zero-padded ID assigned in order of creation. The slug should be a hyphenated noun phrase that lets readers skim the directory and recognize topics at a glance.

## Lifecycle

Each RFC moves through these states, recorded in the `Status` field of the front-matter:

- **Draft** — Author is still iterating; reviewers may comment but the document is not stable.
- **Proposed** — Author considers the document complete and is requesting review and acceptance.
- **Accepted** — Reviewers have agreed. Implementation may start. The RFC is the source of truth for the work.
- **Implemented** — All work described by this RFC has shipped. The RFC is preserved for historical reference.
- **Rejected** — Decision was made not to proceed. The RFC is preserved as a record of the trade-offs considered.
- **Superseded by NNNN** — A later RFC replaces this one. The link to the successor must be in the `Status` field.

Edits after acceptance should be limited to corrections and clarifications. Substantive changes get a new RFC that supersedes the old one.

## Templates

There are three templates, sized to the weight of the decision being made.

### Lightweight

Use this when the decision is small, mostly mechanical, or the design is essentially a series of well-known steps. Sections beyond the front-matter:

- **Summary** — Two or three sentences. What is this RFC about.
- **Motivation** *(optional if the title is self-explanatory)* — Why we are doing this now.
- **Plan** — Bulleted or short prose description of the actual changes.
- **Open questions** *(optional)* — Anything the author noticed but did not resolve.

### Standard

Use this for medium-sized work that touches more than one file or has user-visible behavior. Add to the lightweight template:

- **Requirements** — Numbered functional and non-functional requirements. Each one should be testable.
- **Design** — How the change is structured: data model changes, API shapes, control flow, file layout. Replace the lightweight "Plan" section with this.
- **Test plan** — What will be tested and how (unit, integration, manual). Refer to existing testing conventions; do not redefine them.

### Full

Use this for work that crosses trust boundaries, alters the threat model, or commits the project to a long-lived design. Add to the standard template:

- **Background** — Context a new reader needs that is not obvious from the codebase.
- **Security considerations** — Threat model changes, sensitive-data handling, auth and trust boundaries.
- **Alternatives considered** — Designs that were rejected, and why.
- **Migration / rollout** — How to ship the change without breaking existing deployments.

## Language

RFCs are written in **English**, matching the rest of `docs/`.

## Relationship to other documents

- `README.md` keeps a high-level ROADMAP. RFCs are the detailed expansion of ROADMAP entries that have moved into the "now planning" state.
- `docs/` continues to host operational documentation (deployment, hydration verification, i18n review flow). When an RFC is implemented, the operational docs are updated to reflect the result; the RFC itself is not edited.
- The development guideline (`開発指示書-diarilog-v2.md`, kept outside the repo) describes overall conventions. RFCs do not restate those conventions, they assume them.

## Numbering

RFCs are numbered in creation order, starting from 0001 (this document). IDs are not reused, even for rejected RFCs.

| ID range | Reserved for |
|---|---|
| 0001 | Meta (this RFC) |
| 0002–0099 | Phase 2 finishing work |
| 0100+ | Phase 3 and beyond |

This is a soft convention; the next available number is the only hard requirement.
