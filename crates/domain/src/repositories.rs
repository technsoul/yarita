//! Persistence contracts for the aggregates. Implemented in infrastructure.

use crate::blocklist::Blocklist;
use crate::session::PomodoroTimer;
use crate::stats::FocusRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError(pub String);

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "storage error: {}", self.0)
    }
}

impl std::error::Error for RepositoryError {}

pub trait TimerRepository: Send {
    fn load(&self) -> Result<Option<PomodoroTimer>, RepositoryError>;
    fn save(&self, timer: &PomodoroTimer) -> Result<(), RepositoryError>;
}

pub trait BlocklistRepository: Send {
    fn load(&self) -> Result<Option<Blocklist>, RepositoryError>;
    fn save(&self, blocklist: &Blocklist) -> Result<(), RepositoryError>;
}

pub trait FocusRecordRepository: Send {
    fn all(&self) -> Result<Vec<FocusRecord>, RepositoryError>;
    fn append(&self, record: FocusRecord) -> Result<(), RepositoryError>;
}
