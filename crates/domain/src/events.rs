use crate::Timestamp;
use crate::session::Phase;

/// Facts the domain reports after a state change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    PhaseStarted { phase: Phase, at: Timestamp },
    PhaseCompleted { phase: Phase, started_at: Timestamp, duration_secs: i64 },
    PhaseAbandoned { phase: Phase },
    PhaseSkipped { phase: Phase },
}
