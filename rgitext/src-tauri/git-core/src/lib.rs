//! Git CLI wrapper used by the rGitExt Tauri backend.
//!
//! This crate has no Tauri dependency so that it can be unit-tested on its own.

pub mod blame;
pub mod diff;
pub mod error;
pub mod executor;
pub mod graph;
pub mod models;
pub mod parse;
pub mod revisions;
pub mod tree;
pub mod validate;

pub use error::GitError;
pub use executor::{GitCommand, GitExecutor, GitOutput, LogSink, DEFAULT_TIMEOUT, NETWORK_TIMEOUT};
