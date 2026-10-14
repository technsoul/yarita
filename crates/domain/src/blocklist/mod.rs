mod app_name;
mod site;

pub use app_name::AppName;
pub use site::Site;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::DomainError;

/// Aggregate holding what gets blocked during focus.
///
/// Removal rules under strict mode are enforced by the use case through
/// `PomodoroTimer::ensure_can_loosen`, since they span both aggregates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocklist {
    sites: BTreeSet<Site>,
    apps: BTreeSet<AppName>,
}

impl Blocklist {
    pub fn sites(&self) -> impl Iterator<Item = &Site> {
        self.sites.iter()
    }
    pub fn apps(&self) -> impl Iterator<Item = &AppName> {
        self.apps.iter()
    }

    pub fn add_site(&mut self, site: Site) -> Result<(), DomainError> {
        let label = site.to_string();
        if self.sites.insert(site) { Ok(()) } else { Err(DomainError::AlreadyBlocked(label)) }
    }

    pub fn remove_site(&mut self, site: &Site) -> bool {
        self.sites.remove(site)
    }

    pub fn add_app(&mut self, app: AppName) -> Result<(), DomainError> {
        let label = app.to_string();
        if self.apps.insert(app) { Ok(()) } else { Err(DomainError::AlreadyBlocked(label)) }
    }

    pub fn remove_app(&mut self, app: &AppName) -> bool {
        self.apps.remove(app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_are_rejected_after_normalization() {
        let mut list = Blocklist::default();
        list.add_site(Site::parse("youtube.com").unwrap()).unwrap();
        let dup = list.add_site(Site::parse("https://www.YouTube.com/watch?v=1").unwrap());
        assert!(matches!(dup, Err(DomainError::AlreadyBlocked(_))));
        assert_eq!(list.sites().count(), 1);
    }
}
