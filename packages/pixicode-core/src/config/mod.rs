//! Config system
//!
//! Loads `.pixicode/pixicode.jsonc` (and JSON variant) with JSONC comment
//! stripping, XDG base-dir support, and `PIXICODE_*` env-var overrides.

pub mod paths;
pub mod types;

pub use types::Config;
