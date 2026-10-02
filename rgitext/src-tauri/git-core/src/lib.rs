//! Git CLI wrapper used by the rGitExt Tauri backend.
//!
//! This crate has no Tauri dependency so that it can be unit-tested on its own.

pub mod error;
pub mod executor;
pub mod models;
pub mod parse;
pub mod validate;

pub use error::GitError;
pub use executor::{GitCommand, GitExecutor, GitOutput, LogSink, DEFAULT_TIMEOUT, NETWORK_TIMEOUT};
