#!/usr/bin/env bash
# Build the client-side WASM hydration bundle.
#
# Source: hydrate/   (wasm-pack project, web-app hydrate feature)
# Output: dist/      (served as static assets by Workers Assets binding)
#
# Prerequisites (one-time):
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-pack
#
# Usage:
#   bash scripts/build-bff-hydrate.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HYDRATE_DIR="$ROOT/hydrate"
DIST_DIR="$ROOT/dist"

command -v wasm-pack >/dev/null 2>&1 || {
    echo "ERROR: wasm-pack not found. Install with: cargo install wasm-pack" >&2
    exit 1
}

mkdir -p "$DIST_DIR"

echo "Building hydration bundle..."
cd "$HYDRATE_DIR"
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
wasm-pack build \
    --target web \
    --out-dir "$DIST_DIR" \
    --out-name "web-app" \
    --release

echo ""
echo "Output: $DIST_DIR"
ls -lh "$DIST_DIR/web-app.js" "$DIST_DIR/web-app_bg.wasm" 2>/dev/null || true
echo "Done."
