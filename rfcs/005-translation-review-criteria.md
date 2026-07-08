# RFC 005: Translation review process and acceptance criteria

| Field | Value |
|---|---|
| Status | Proposed |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-22 |
| Template | Standard (operational) |
| Revision | r2 — split phone verification responsibility between maintainer and reviewer (was unrealistic to ask reviewers to place test calls) |

## Summary

`docs/i18n-review-flow.md` already describes the lifecycle of bringing a translation through expert review. This RFC complements that document by fixing the **acceptance criteria** — what conditions must be true before a translation is allowed to flip from `reviewed: false` to `reviewed: true` — and the **governance** of who can authorize that flip.

## Motivation

The existing flow doc tells you what steps to follow. It does not tell you when to consider a translation "done enough" to mark reviewed. Without explicit acceptance criteria the bar drifts: one reviewer might be satisfied with hotline numbers being checked once, another might require a full clinical sign-off. For a service where flipping a flag changes what a user in crisis sees, that variability is unacceptable.

A second gap: the flow document is internal. It does not specify who has the authority to merge a `reviewed: true` change. Anyone with repo write access could currently do it, regardless of whether they read any translation.

## Requirements

### Acceptance criteria for `reviewed: true`

A translation entry (a single `HotlineInfo`, a single `CrisisResources.message`, or one of the four prompt slots in `dialog/prompts.rs`) may be marked `reviewed: true` only when **all** of the following hold:

1. **R1** — A native or fluent speaker of the target language has read the entry in context (i.e. seen where it is rendered in the UI, not just the FTL value in isolation).
2. **R2** — That reviewer holds either:
   - a clinical credential relevant to mental health (psychiatrist, clinical psychologist, licensed counselor, registered psychotherapist, equivalent), **or**
   - documented operational experience at a recognized crisis intervention service (Lifeline-equivalent, IFRC mental health teams, MSF MHPSS, or comparable).
3. **R3** — For `HotlineInfo`: the maintainer has verified the phone number and URL within the last 12 months by **non-intrusive means**: checking the published business or administrative line if one is documented separately from the crisis line, confirming the operating organization still exists from its official website, and confirming the listed URL still resolves to the same organization. **Placing a test call to an active crisis line is forbidden**: doing so consumes a counselor's time that should be available to people in actual distress. The verification date and method are recorded in the commit message.
4. **R3a** — For `HotlineInfo`: the reviewer (per R1 and R2) has separately confirmed that the displayed name and any descriptive text are accurate, culturally appropriate, and respectful in the target language. The reviewer does not perform technical verification of the phone number; that responsibility sits with the maintainer per R3.
5. **R4** — For `CrisisResources.message` and dialog prompts: the reviewer has confirmed the text aligns with trauma-informed care principles (no judgement, no medical advice, calm tone, hope-affirming without being saccharine, no minimization of distress).
6. **R5** — A second reviewer of the same language has cross-checked the entry. The second reviewer does not need clinical credentials but must be a native or fluent speaker.
7. **R6** — Any conflict-of-interest disclosure: if either reviewer is associated with an organization listed in the hotlines, that fact is recorded in the commit message.

### Governance

- **Approver** — Only a maintainer designated as "i18n approver" in `MAINTAINERS.md` (file to be created) may merge a PR that flips `reviewed: false` to `true`. Initial approver: `nabbisen`. Adding an approver is itself a maintainer decision.
- **Reviewer pool** — Reviewers may be community contributors and need not have repo write access. Their attestation is captured in the PR description and signed by name (and credential, if comfortable).
- **Audit trail** — Every flip is its own commit, and the commit message records: (a) reviewer names or pseudonyms with consent, (b) credentials cited, (c) verification date **and method** for hotlines (per R3), (d) any conflict-of-interest disclosure.

### Periodic re-review

A `reviewed: true` mark is **not permanent**. Hotline numbers can be retired, URLs can change ownership, and clinical guidance evolves. The flag must be re-confirmed annually:

- Every January, an issue is opened listing every `reviewed: true` entry whose last verification commit is older than 11 months.
- For hotlines, R3 (maintainer technical verification) and R3a (reviewer cultural confirmation) are both repeated.
- For text content, the previous reviewers (or new ones meeting R2) confirm or revise.
- Entries that fail re-review revert to `reviewed: false` and lose their position in the published list — the IASP fallback covers the gap (RFC 001-precursor invariant).

## Operational changes to `docs/i18n-review-flow.md`

The existing doc has a "監修済み翻訳の履歴" (Reviewed translation history) table. That table should be augmented with two columns:

- `Last verified` — date of the most recent R3/R4 confirmation
- `Next review by` — date 12 months later, used to drive the annual re-review issue

The table currently has placeholder dates (`2026-XX-XX`); those must be filled in with actual commit dates the next time anyone touches the file. This RFC does not require backfilling them retroactively beyond what the implementer can recover from git history.

## Cost and sourcing

The author currently has no committed budget for paid clinical review. Two paths forward, not mutually exclusive:

- **Volunteer reviewers** through partner NPOs (Lifeline-equivalents, MSF MHPSS coordinators, regional clinical psychology associations).
- **Paid spot review** — a single paid clinical session per language to validate volunteer-produced translations, treating the volunteer review as the primary work and the paid session as a final attestation. Estimated cost is low because the work is bounded (a few pages of text per language).

Either way, R5 (two-reviewer rule) holds. The decision of which model applies per language is recorded in `MAINTAINERS.md`.

## Out of scope

- Continuous monitoring of hotline availability (live healthchecks against phone numbers, etc.). The annual re-review is the current commitment.
- Automated translation tooling. Machine translation seeds remain acceptable as a **starting point** for reviewer work but never as the final state of `reviewed: true`.
- Crowd-sourced translation platforms (Weblate, Crowdin, Pontoon). Worth considering once we have multiple active languages, but introduces a third trust boundary (the platform itself) and is therefore deferred.

## Open questions

- Should we publish the `MAINTAINERS.md` file with reviewer real names, or use opt-in pseudonyms? Pseudonyms reduce harassment risk for reviewers in regions where mental health work is stigmatized, but reduce auditability. Default proposal: opt-in real-name with pseudonym fallback, noted per-entry in the maintainers file.
- What is the SLA for re-review on a hotline that turns out to be defunct mid-year? Proposal: revert to `reviewed: false` immediately on report, file a hotfix PR within 7 days. Open to tightening.
