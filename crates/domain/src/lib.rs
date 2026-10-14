//! Pure domain model for the focus timer.
//!
//! No I/O, no OS calls, no framework code. Everything here is deterministic
//! and driven by an explicit `Timestamp`, which keeps the rules testable.

pub mod blocklist;
pub mod error;
pub mod events;
pub mod repositories;
pub mod session;
pub mod stats;
pub mod time;

pub use error::DomainError;
pub use time::Timestamp;
