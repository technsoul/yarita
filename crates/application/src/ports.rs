//! Outbound ports the use cases need from the outside world.

use focus_domain::Timestamp;
use focus_domain::blocklist::{AppName, Site};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortError(pub String);

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub trait Clock: Send {
    fn now(&self) -> Timestamp;
    fn utc_offset_secs(&self) -> i64;
}

/// Blocks websites system-wide. Implementations must be idempotent: they are
/// called every tick and should only touch the system when something changed.
pub trait SiteBlocker: Send {
    fn block(&mut self, sites: &[Site]) -> Result<(), PortError>;
    fn unblock_all(&mut self) -> Result<(), PortError>;
}

/// Stops blocked apps from running. Same idempotency rule as [`SiteBlocker`].
pub trait AppBlocker: Send {
    /// Blocks `apps` (and only those). Returns the names of processes it closed.
    fn enforce(&mut self, apps: &[AppName]) -> Result<Vec<String>, PortError>;
    /// Undoes anything `enforce` left in place, such as launch blocks.
    fn release_all(&mut self) -> Result<(), PortError>;
}
