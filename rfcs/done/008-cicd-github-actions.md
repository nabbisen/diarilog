# RFC 008: CI/CD with GitHub Actions

| Field | Value |
|---|---|
| Status | Implemented |
| Author | nabbisen |
| Created | 2026-05-04 |
| Last updated | 2026-05-04 |
| Template | Standard + Security |

## Summary

Set up GitHub Actions workflows for: PR-time validation (cargo check, cargo test, formatting), staging deployment on merge to `main`, and manual production deployment via workflow dispatch. Use selective deployment so workers without changes are skipped.

## Motivation

Deploying six workers manually in the right order, with the right `--env` flag, is error-prone. The order itself is non-trivial (`identity → safety → journal → dialog → bff → gateway`, see `docs/deployment/deploy-order.md`). Beyond convenience, automated CI catches regressions before they reach staging — currently no automated gate exists between a PR and a developer running `wrangler deploy`.

## Requirements

1. **R1 (functional, must)** — Every PR runs `cargo check --workspace --offline`, `cargo check --workspace --all-targets --offline`, and `cargo test --workspace --offline --tests` in CI. The PR cannot merge if any step fails.
2. **R2 (functional, must)** — On merge to `main`, the workflow deploys to **staging** in the correct dependency order. Failures abort the chain and the operator is notified.
3. **R3 (functional, should)** — Only workers whose source has changed are deployed (selective deployment), unless the workflow is manually triggered with a "deploy all" flag.
4. **R4 (functional, should)** — Production deployment is **manual** (`workflow_dispatch`) and requires an approver other than the PR author. Same selective rules apply.
5. **R5 (non-functional, must)** — The workflow runs in under 15 minutes for the typical case (test + selective deploy of 1–2 workers). Caching of cargo registry and `target/` is required.
6. **R6 (security, must)** — Cloudflare API tokens, OIDC client secrets, and Turnstile secrets are stored in GitHub Secrets, scoped per-environment.
7. **R7 (security, must)** — No secret value appears in workflow logs. The default `set -x` discipline applies; values are passed through `env:` blocks not echoed in shell.

## Design

### Files

```
.github/
└── workflows/
    ├── ci.yml          ← runs on PRs and on push to main, before deploy
    ├── deploy-staging.yml  ← runs on push to main after ci.yml succeeds
    └── deploy-production.yml  ← workflow_dispatch only
```

### `ci.yml` jobs

Single job, `validate`:

1. Checkout
2. Install Rust 1.91 with `wasm32-unknown-unknown` target
3. Cache `~/.cargo/registry` and `target` keyed on `Cargo.lock` hash
4. `cargo check --workspace --offline`
5. `cargo check --workspace --all-targets --offline`
6. `cargo test --workspace --offline --tests` — fail if test count regresses below the v0.6 baseline of 71 (script comparison)
7. `cargo fmt --all -- --check`
8. `cargo clippy --workspace --offline -- -D warnings` — once we adopt a clippy baseline; soft gate at first

Why `--offline`: the network-dependent crate fetch happens once at cache miss; subsequent runs are deterministic. The cache key includes `Cargo.lock` so any dependency change properly invalidates.

### `deploy-staging.yml`

Two jobs, run in series:

**Job 1: `detect-changes`**

Uses `dorny/paths-filter@v3` to compute booleans:

- `gateway: workers/gateway/**`
- `bff:     workers/bff/** | packages/web-app/** | packages/contracts/** | packages/sb-client/** | packages/errors/**`
- `journal: workers/journal/**`
- `identity: workers/identity/**`
- `safety:  workers/safety/**`
- `dialog:  workers/dialog/** | workers/safety/**`  (dialog calls safety, redeploy when safety contract changes)
- `bff_hydrate: workers/bff-hydrate/** | packages/web-app/**`

bff has the broadest dependency set because the SSR HTML embeds web-app; any web-app change must trigger a bff redeploy.

**Job 2: `deploy`**

Per-worker steps that run only if the corresponding boolean is true. The order is hard-coded to match `docs/deployment/deploy-order.md`:

```yaml
- name: deploy identity
  if: needs.detect-changes.outputs.identity == 'true'
  run: cd workers/identity && wrangler deploy --env staging
  env:
    CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN_STAGING }}
- name: deploy safety
  if: needs.detect-changes.outputs.safety == 'true'
  ...
```

bff-hydrate is special: it runs `scripts/build-bff-hydrate.sh` and copies output into `workers/bff/dist/static/_assets/` before bff deploys. So the implicit ordering is:

```
identity, safety, journal, dialog (any order)
  → bff-hydrate build
  → bff deploy
  → gateway deploy
```

### `deploy-production.yml`

Same skeleton as staging, but:

- Triggered only by `workflow_dispatch`.
- Requires `environment: production` declaration, which makes GitHub require a designated reviewer's approval before any step runs.
- Selective deployment defaults to enabled; an input flag `deploy_all: bool` reruns everything.
- Inputs include `confirm_message: string` that the operator must fill with the version tag they intend to deploy. The first job validates the message matches the latest tag on main; mismatch fails fast.

### Secrets layout

GitHub Secrets, scoped to environments `staging` and `production` separately:

| Secret | Scope |
|---|---|
| `CLOUDFLARE_API_TOKEN_STAGING` | repo, used by staging workflow |
| `CLOUDFLARE_API_TOKEN_PRODUCTION` | env `production`, requires reviewer |
| `OIDC_ISSUER`, `OIDC_AUDIENCE` (if used as wrangler vars) | env-scoped |
| `TURNSTILE_SECRET` | env-scoped |

Production secrets are gated by environment protection rules. The reviewer approval acts as a second pair of eyes on production deploys.

### Caching specifics

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: cargo-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      cargo-${{ runner.os }}-
```

The fallback `restore-keys` lets a Cargo.lock change reuse most of the previous cache while picking up new dependencies. This brings clean-cache PR builds under 15 minutes.

## Test plan

- **Workflow lint** — Run `actionlint` locally before merging the workflow files; add it to `ci.yml` itself once the workflow is stable.
- **Dry-run staging** — On the first PR that adds these workflows, deploy to a throwaway staging Cloudflare account and verify each worker reaches Cloudflare.
- **Test count guard** — Add a small bash step in `ci.yml` that parses `cargo test` output, sums passed counts, and fails if the total is less than 71. Update the threshold each time the baseline grows.
- **Manual** — Run the production workflow against staging once (using staging secrets) to verify the dispatch + approval flow before pointing it at production.

There are no new code tests; the workflows themselves are infrastructure.

## Security considerations

- **Secret leakage in logs** — All secret references go through `env:` blocks. No `echo $VAR` debugging is allowed in committed workflow files. Workflow logs for failed runs are reviewed and any leak is treated as a token rotation event.
- **Compromised PR running with secrets** — `pull_request` triggers run with the *base branch's* workflow definitions and *no secrets* by default. Forks cannot exfiltrate. The deploy workflows trigger on `push` to `main` only, so a malicious PR cannot deploy until it has been merged by a maintainer.
- **Token scope** — Cloudflare API tokens are scoped to the minimum required: write access to specific zones and worker resources. They are not account-wide. Token creation procedure is documented in `docs/deployment/initial-setup.md` and amended as part of this RFC.
- **Reviewer requirement** — Production environment protection rule lists `nabbisen` as the sole reviewer initially; expanded as the maintainer team grows. A merge-and-deploy by the same person to staging is fine; production always needs a second person.

## Out of scope

- Self-hosted runners. GitHub-hosted runners are sufficient for this workload.
- Preview deployments per PR. Tempting but expensive in Cloudflare Workers seats and complicates cleanup. Defer until there is a real reviewer process that wants them.
- Automated dependency update PRs (Dependabot, Renovate). Worth doing eventually but a separate concern.
- Release tagging automation. Manual `git tag` for now; a small script can be added once the workflow lands.

## Open questions

- The `bff_hydrate` build step produces non-Rust artifacts that need to ship with bff. The cleanest solution is to make the staging deploy job itself run `scripts/build-bff-hydrate.sh` rather than bundling artifacts as workflow outputs. Verify the script works in the GitHub runner environment (specifically: wasm-pack availability) — if not, install wasm-pack as a workflow step.
- What is the policy when a deploy fails halfway? E.g. `journal` succeeds, `dialog` fails. The dependency chain is preserved (dialog being broken does not break journal, and journal callers will see the old dialog). The current design accepts this partial state; the operator must fix forward. Open to a stricter rollback approach but that adds complexity.
- Should we make `cargo audit` part of `ci.yml`? It is fast and catches known-vuln dependencies. Lean toward yes, soft gate (warn but do not fail) initially.
