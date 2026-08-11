//! TPT Anima — unified meta-crate.
//!
//! Re-exports the four Anima layers behind Cargo feature flags so downstream
//! crates can depend on a single line:
//!
//! ```toml
//! tpt-anima = { version = "0.1", features = ["sse", "inp"] }
//! ```
//!
//! By default all four layers are enabled. Disable the default features and
//! opt into only what you need to shrink the dependency graph:
//!
//! ```toml
//! tpt-anima = { version = "0.1", default-features = false, features = ["ck"] }
//! ```

#![allow(clippy::single_component_path_imports)]

#[cfg(feature = "sse")]
pub use tpt_anima_sse as sse;

#[cfg(feature = "inp")]
pub use tpt_anima_inp as inp;

#[cfg(feature = "ck")]
pub use tpt_anima_ck as ck;

#[cfg(feature = "zkpa")]
pub use tpt_anima_zkpa as zkpa;
