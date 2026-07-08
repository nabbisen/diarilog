//! Worker 間 Service Bindings 境界を跨ぐ型定義。
//!
//! 方針:
//! - ここには「境界に出現する型」のみを置く
//! - D1 レコード内部表現や、各 Worker 固有の内部モデルは含めない
//! - `serde` 互換であること、`Clone` + `Debug` を持つこと

pub mod bff;
pub mod common;
pub mod dialog;
pub mod diary;
pub mod identity;
pub mod safety;
