//! Browser-side hydration bundle.
//!
//! このクレート自体には実装はない。`web-app` を `hydrate` feature で取り込むと、
//! `web_app::hydrate_main` (`#[wasm_bindgen(start)]`) が WASM モジュールロード時に
//! 自動的に呼ばれ、SSR された DOM をハイドレートする。
//!
//! ## ビルド方法
//!
//! ```bash
//! cd workers/bff-hydrate
//! wasm-pack build --target web --out-dir ../../dist/web-app --release
//! ```
//!
//! 生成物 (`dist/web-app/web_app_bg.wasm` + `web_app.js`) を bff-worker の
//! `dist/static/_assets/` に配置する。Workers Static Assets バインディングが
//! Cloudflare のエッジから直接配信する。
//!
//! ## ビルドスクリプト連携
//!
//! `scripts/build-bff-hydrate.sh` が wasm-pack ビルドと
//! `dist/static/_assets/` への配置を一括で実行する。

// web-app を再エクスポート (デバッグ用、本来は不要)
pub use web_app;
