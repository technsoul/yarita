//! Runs several blockers as one, so each OS mechanism stays its own adapter.

use yarita_application::ports::{AppBlocker, PortError, SiteBlocker};
use yarita_domain::blocklist::{AppName, Site};

/// Every blocker runs even if an earlier one fails; the first error is returned.
fn run_all<B: ?Sized, T>(
    blockers: &mut [Box<B>],
    mut f: impl FnMut(&mut B) -> Result<T, PortError>,
) -> (Vec<T>, Option<PortError>) {
    let mut out = Vec::new();
    let mut first_error = None;
    for blocker in blockers {
        match f(blocker) {
            Ok(v) => out.push(v),
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    (out, first_error)
}

pub struct SiteBlockers(pub Vec<Box<dyn SiteBlocker>>);

impl SiteBlocker for SiteBlockers {
    fn block(&mut self, sites: &[Site]) -> Result<(), PortError> {
        run_all(&mut self.0, |b| b.block(sites)).1.map_or(Ok(()), Err)
    }

    fn unblock_all(&mut self) -> Result<(), PortError> {
        run_all(&mut self.0, |b| b.unblock_all()).1.map_or(Ok(()), Err)
    }

    fn bypassing_apps(&self) -> Vec<AppName> {
        let mut apps: Vec<AppName> = self.0.iter().flat_map(|b| b.bypassing_apps()).collect();
        apps.sort();
        apps.dedup();
        apps
    }
}

pub struct AppBlockers(pub Vec<Box<dyn AppBlocker>>);

impl AppBlocker for AppBlockers {
    fn enforce(&mut self, apps: &[AppName]) -> Result<Vec<String>, PortError> {
        match run_all(&mut self.0, |b| b.enforce(apps)) {
            (_, Some(e)) => Err(e),
            (closed, None) => Ok(closed.concat()),
        }
    }

    fn release_all(&mut self) -> Result<(), PortError> {
        run_all(&mut self.0, |b| b.release_all()).1.map_or(Ok(()), Err)
    }
}
