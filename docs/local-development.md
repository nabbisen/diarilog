# Local Development

## `cargo run` does not apply

Workers are WebAssembly libraries (`crate-type = ["cdylib"]`).
Use `wrangler dev` instead.

## `cargo test` — unit tests

```bash
cargo test --workspace --tests
```

85 tests across 7 crates. No Cloudflare account needed.

## `wrangler dev` — run in the browser

### One-time prerequisites

```bash
rustup target add wasm32-unknown-unknown
apt-get install -y pkg-config libssl-dev
cargo install worker-build
npm install -g wrangler
```

### Run

```bash
wrangler dev        # → http://localhost:8787
```

A root `wrangler.toml` is provided. `wrangler dev` from the project root
starts the single consolidated worker. No login needed.

### Authentication

The worker requires OIDC. Add credentials to `.dev.vars` in the project
root (not committed):

```ini
OIDC_ISSUER=https://your-tenant.auth0.com
OIDC_AUDIENCE=your-client-id
TURNSTILE_SECRET=1x0000000000000000000000000000000AA
```

See `docs/deployment/oidc-providers.md` for per-provider setup.

### Apply migrations locally

```bash
wrangler d1 migrations apply diarilog-db --local
```

## Build the client-side hydration bundle (optional)

The app renders server-side HTML without the hydration bundle. Pages are
readable and navigable; client-side interactivity requires the bundle.

```bash
bash scripts/build-bff-hydrate.sh
```

Output goes to `dist/`. Run once before `wrangler dev` if you need full
interactivity, or after making changes to `packages/web-app` UI code.

Prerequisites: `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-pack`.
