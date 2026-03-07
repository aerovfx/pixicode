/// Core crate version (for desktop/CLI display).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod acp;
pub mod agent;
pub mod bus;
pub mod config;
pub mod db;
pub mod git;
pub mod log;
pub mod plugin;
pub mod providers;
pub mod server;
pub mod session;
pub mod tools;
