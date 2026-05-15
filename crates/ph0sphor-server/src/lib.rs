//! PHOSPHOR server library.
//!
//! Exposed so integration tests can drive `run` against an ephemeral port
//! without going through the `main` binary's CLI.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod auth;
pub mod collectors;
pub mod config;
pub mod control;
pub mod net;
pub mod state;

pub use config::ServerConfig;
pub use net::{serve, ServerHandle};
pub use state::State;
