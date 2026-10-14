//! OS and storage adapters implementing the domain and application ports.

mod browser_policy;
mod clock;
mod composite;
mod hosts_blocker;
mod json_store;
mod launch_blocker;
pub mod paths;
mod process_blocker;
#[cfg(windows)]
mod registry;

#[cfg(windows)]
pub use browser_policy::BrowserPolicyBlocker;
pub use clock::SystemClock;
pub use composite::{AppBlockers, SiteBlockers};
pub use hosts_blocker::HostsFileBlocker;
pub use json_store::JsonStore;
#[cfg(windows)]
pub use launch_blocker::LaunchBlocker;
pub use launch_blocker::BLOCKED_FLAG;
pub use process_blocker::ProcessBlocker;
#[cfg(windows)]
pub use registry::Registry;
