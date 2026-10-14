use std::fmt;

use focus_domain::DomainError;
use focus_domain::blocklist::{AppName, Blocklist, Site};
use focus_domain::events::DomainEvent;
use focus_domain::repositories::{
    BlocklistRepository, FocusRecordRepository, RepositoryError, TimerRepository,
};
use focus_domain::session::{Phase, PomodoroTimer, TimerConfig};
use focus_domain::stats::{FocusRecord, StatsSummary};
use focus_protocol::{ConfigDto, Snapshot};

use crate::mapping;
use crate::ports::{AppBlocker, Clock, SiteBlocker};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Domain(DomainError),
    Storage(RepositoryError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Domain(e) => e.fmt(f),
            AppError::Storage(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for AppError {}

impl From<DomainError> for AppError {
    fn from(e: DomainError) -> Self {
        AppError::Domain(e)
    }
}

impl From<RepositoryError> for AppError {
    fn from(e: RepositoryError) -> Self {
        AppError::Storage(e)
    }
}

pub struct Dependencies {
    pub timers: Box<dyn TimerRepository>,
    pub blocklists: Box<dyn BlocklistRepository>,
    pub records: Box<dyn FocusRecordRepository>,
    pub sites: Box<dyn SiteBlocker>,
    pub apps: Box<dyn AppBlocker>,
    pub clock: Box<dyn Clock>,
}

/// The application service. One instance lives inside the background service.
pub struct FocusApp {
    deps: Dependencies,
    timer: PomodoroTimer,
    blocklist: Blocklist,
    records: Vec<FocusRecord>,
    enforcement_error: Option<String>,
    completions: u64,
}

impl FocusApp {
    /// Loads saved state. A session that was running before a crash or reboot
    /// resumes here, and the next `tick` re-applies (or lifts) blocking.
    pub fn load(deps: Dependencies) -> Result<Self, AppError> {
        let timer = deps.timers.load()?.unwrap_or_else(|| PomodoroTimer::new(TimerConfig::default()));
        let blocklist = deps.blocklists.load()?.unwrap_or_default();
        let records = deps.records.all()?;
        let mut app = Self { deps, timer, blocklist, records, enforcement_error: None, completions: 0 };
        app.tick();
        Ok(app)
    }

    // ---- use cases --------------------------------------------------------

    pub fn start(&mut self) -> Result<(), AppError> {
        let now = self.deps.clock.now();
        self.change_timer(|t| t.start(now).map(drop))
    }

    pub fn pause(&mut self) -> Result<(), AppError> {
        let now = self.deps.clock.now();
        self.change_timer(|t| t.pause(now))
    }

    pub fn resume(&mut self) -> Result<(), AppError> {
        let now = self.deps.clock.now();
        self.change_timer(|t| t.resume(now))
    }

    pub fn stop(&mut self) -> Result<(), AppError> {
        self.change_timer(|t| t.stop().map(drop))
    }

    pub fn skip(&mut self) -> Result<(), AppError> {
        self.change_timer(|t| t.skip().map(drop))
    }

    pub fn set_strict(&mut self, strict: bool) -> Result<(), AppError> {
        self.change_timer(|t| t.set_strict(strict))
    }

    pub fn update_config(&mut self, dto: ConfigDto) -> Result<(), AppError> {
        let config = TimerConfig::new(
            dto.focus_minutes,
            dto.short_break_minutes,
            dto.long_break_minutes,
            dto.sessions_before_long_break,
        )?;
        self.change_timer(|t| t.update_config(config))
    }

    pub fn add_site(&mut self, input: &str) -> Result<(), AppError> {
        let site = Site::parse(input)?;
        self.change_blocklist(|b| b.add_site(site))
    }

    pub fn remove_site(&mut self, input: &str) -> Result<(), AppError> {
        self.timer.ensure_can_loosen()?;
        let site = Site::parse(input)?;
        self.change_blocklist(|b| {
            b.remove_site(&site);
            Ok(())
        })
    }

    pub fn add_app(&mut self, input: &str) -> Result<(), AppError> {
        let app = AppName::parse(input)?;
        self.change_blocklist(|b| b.add_app(app))
    }

    pub fn remove_app(&mut self, input: &str) -> Result<(), AppError> {
        self.timer.ensure_can_loosen()?;
        let app = AppName::parse(input)?;
        self.change_blocklist(|b| {
            b.remove_app(&app);
            Ok(())
        })
    }

    /// Called about once a second: advances the timer and enforces blocking.
    pub fn tick(&mut self) {
        let now = self.deps.clock.now();
        let mut timer = self.timer.clone();
        if let Some(event) = timer.tick(now) {
            if let DomainEvent::PhaseCompleted { phase: Phase::Focus, started_at, duration_secs } = event {
                let record = FocusRecord { started_at, duration_secs };
                match self.deps.records.append(record) {
                    Ok(()) => self.records.push(record),
                    Err(e) => self.enforcement_error = Some(e.to_string()),
                }
            }
            // Keep the in-memory transition even if saving fails, so a
            // finished session never keeps blocking; retry the save next time.
            if let Err(e) = self.deps.timers.save(&timer) {
                self.enforcement_error = Some(e.to_string());
            }
            self.timer = timer;
            self.completions += 1;
        }
        self.enforce();
    }

    /// Strict focus in progress: the UI must stay open.
    pub fn is_locked(&self) -> bool {
        self.timer.is_locked()
    }

    pub fn snapshot(&self) -> Snapshot {
        let now = self.deps.clock.now();
        let t = &self.timer;
        let summary = StatsSummary::compute(&self.records, now, self.deps.clock.utc_offset_secs());
        Snapshot {
            phase: mapping::phase(t.active_phase().unwrap_or(t.next_phase())),
            status: mapping::status(t.state()),
            remaining_secs: t.remaining_secs(now),
            total_secs: t.total_secs(),
            completed_in_cycle: t.completed_in_cycle(),
            strict: t.is_strict(),
            locked: t.is_locked(),
            blocking: t.is_blocking(),
            config: mapping::config(t.config()),
            sites: self.blocklist.sites().map(ToString::to_string).collect(),
            apps: self.blocklist.apps().map(ToString::to_string).collect(),
            stats: mapping::stats(summary),
            enforcement_error: self.enforcement_error.clone(),
            completions: self.completions,
        }
    }

    // ---- helpers ----------------------------------------------------------

    /// Apply a change to a copy, persist it, then commit. Keeps memory and
    /// disk consistent if either the rule or the save fails.
    fn change_timer(
        &mut self,
        f: impl FnOnce(&mut PomodoroTimer) -> Result<(), DomainError>,
    ) -> Result<(), AppError> {
        let mut timer = self.timer.clone();
        f(&mut timer)?;
        self.deps.timers.save(&timer)?;
        self.timer = timer;
        self.enforce();
        Ok(())
    }

    fn change_blocklist(
        &mut self,
        f: impl FnOnce(&mut Blocklist) -> Result<(), DomainError>,
    ) -> Result<(), AppError> {
        let mut list = self.blocklist.clone();
        f(&mut list)?;
        self.deps.blocklists.save(&list)?;
        self.blocklist = list;
        self.enforce();
        Ok(())
    }

    fn enforce(&mut self) {
        let result = if self.timer.is_blocking() {
            let sites: Vec<Site> = self.blocklist.sites().cloned().collect();
            let apps: Vec<AppName> = self.blocklist.apps().cloned().collect();
            self.deps.sites.block(&sites).and_then(|()| self.deps.apps.enforce(&apps).map(drop))
        } else {
            // Run both so one failure doesn't leave the other side blocked.
            let sites = self.deps.sites.unblock_all();
            let apps = self.deps.apps.release_all();
            sites.and(apps)
        };
        self.enforcement_error = result.err().map(|e| e.0);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::{Arc, Mutex};

    use focus_domain::Timestamp;
    use focus_protocol::StatusDto;

    use super::*;
    use crate::ports::PortError;

    #[derive(Clone, Default)]
    struct World {
        now: Arc<AtomicI64>,
        blocked_sites: Arc<Mutex<Vec<String>>>,
        killed: Arc<Mutex<Vec<String>>>,
        launch_blocked: Arc<Mutex<Vec<String>>>,
        timer: Arc<Mutex<Option<PomodoroTimer>>>,
        blocklist: Arc<Mutex<Option<Blocklist>>>,
        records: Arc<Mutex<Vec<FocusRecord>>>,
    }

    impl Clock for World {
        fn now(&self) -> Timestamp {
            Timestamp(self.now.load(Ordering::SeqCst))
        }
        fn utc_offset_secs(&self) -> i64 {
            0
        }
    }
    impl SiteBlocker for World {
        fn block(&mut self, sites: &[Site]) -> Result<(), PortError> {
            *self.blocked_sites.lock().unwrap() = sites.iter().map(ToString::to_string).collect();
            Ok(())
        }
        fn unblock_all(&mut self) -> Result<(), PortError> {
            self.blocked_sites.lock().unwrap().clear();
            Ok(())
        }
    }
    impl AppBlocker for World {
        fn enforce(&mut self, apps: &[AppName]) -> Result<Vec<String>, PortError> {
            let names: Vec<String> = apps.iter().map(ToString::to_string).collect();
            self.killed.lock().unwrap().extend(names.clone());
            *self.launch_blocked.lock().unwrap() = names.clone();
            Ok(names)
        }
        fn release_all(&mut self) -> Result<(), PortError> {
            self.launch_blocked.lock().unwrap().clear();
            Ok(())
        }
    }
    impl TimerRepository for World {
        fn load(&self) -> Result<Option<PomodoroTimer>, RepositoryError> {
            Ok(self.timer.lock().unwrap().clone())
        }
        fn save(&self, t: &PomodoroTimer) -> Result<(), RepositoryError> {
            *self.timer.lock().unwrap() = Some(t.clone());
            Ok(())
        }
    }
    impl BlocklistRepository for World {
        fn load(&self) -> Result<Option<Blocklist>, RepositoryError> {
            Ok(self.blocklist.lock().unwrap().clone())
        }
        fn save(&self, b: &Blocklist) -> Result<(), RepositoryError> {
            *self.blocklist.lock().unwrap() = Some(b.clone());
            Ok(())
        }
    }
    impl FocusRecordRepository for World {
        fn all(&self) -> Result<Vec<FocusRecord>, RepositoryError> {
            Ok(self.records.lock().unwrap().clone())
        }
        fn append(&self, r: FocusRecord) -> Result<(), RepositoryError> {
            self.records.lock().unwrap().push(r);
            Ok(())
        }
    }

    fn app(world: &World) -> FocusApp {
        FocusApp::load(Dependencies {
            timers: Box::new(world.clone()),
            blocklists: Box::new(world.clone()),
            records: Box::new(world.clone()),
            sites: Box::new(world.clone()),
            apps: Box::new(world.clone()),
            clock: Box::new(world.clone()),
        })
        .unwrap()
    }

    #[test]
    fn focus_blocks_then_unblocks_and_records_stats() {
        let w = World::default();
        let mut a = app(&w);
        a.add_site("https://www.youtube.com").unwrap();
        a.add_app("Steam.exe").unwrap();
        assert!(w.blocked_sites.lock().unwrap().is_empty(), "no blocking while idle");

        a.start().unwrap();
        assert_eq!(*w.blocked_sites.lock().unwrap(), ["youtube.com"]);
        assert!(w.killed.lock().unwrap().contains(&"steam".to_string()));

        assert_eq!(*w.launch_blocked.lock().unwrap(), ["steam"]);

        w.now.store(25 * 60, Ordering::SeqCst);
        a.tick();
        assert!(w.blocked_sites.lock().unwrap().is_empty());
        assert!(w.launch_blocked.lock().unwrap().is_empty(), "app blocks lifted after focus");
        let s = a.snapshot();
        assert_eq!(s.status, StatusDto::Idle);
        assert_eq!(s.stats.total_sessions, 1);
        assert_eq!(s.completions, 1);
    }

    #[test]
    fn strict_focus_refuses_loosening_the_blocklist() {
        let w = World::default();
        let mut a = app(&w);
        a.add_site("reddit.com").unwrap();
        a.set_strict(true).unwrap();
        a.start().unwrap();
        assert_eq!(a.remove_site("reddit.com"), Err(AppError::Domain(DomainError::StrictModeActive)));
        assert_eq!(a.stop(), Err(AppError::Domain(DomainError::StrictModeActive)));
        a.add_site("x.com").unwrap(); // tightening is fine
        assert_eq!(a.snapshot().sites, ["reddit.com", "x.com"]);
    }

    #[test]
    fn session_survives_restart() {
        let w = World::default();
        {
            let mut a = app(&w);
            a.add_site("reddit.com").unwrap();
            a.set_strict(true).unwrap();
            a.start().unwrap();
        } // service crashes
        w.blocked_sites.lock().unwrap().clear();
        w.now.store(60, Ordering::SeqCst);
        let a = app(&w);
        let s = a.snapshot();
        assert!(s.locked);
        assert_eq!(s.remaining_secs, 24 * 60);
        assert_eq!(*w.blocked_sites.lock().unwrap(), ["reddit.com"], "blocking re-applied on load");
    }
}
