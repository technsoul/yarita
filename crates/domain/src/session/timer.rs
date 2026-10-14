use serde::{Deserialize, Serialize};

use super::{Phase, TimerConfig};
use crate::events::DomainEvent;
use crate::{DomainError, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerState {
    Idle,
    Running { phase: Phase, started_at: Timestamp, ends_at: Timestamp },
    Paused { phase: Phase, started_at: Timestamp, remaining_secs: i64 },
}

/// Aggregate root for the Pomodoro cycle.
///
/// Invariant: while a strict focus session is active, nothing that would
/// weaken blocking is allowed (pause, stop, skip, turning strict off,
/// changing settings, removing blocklist entries).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PomodoroTimer {
    config: TimerConfig,
    strict: bool,
    completed_in_cycle: u32,
    next_phase: Phase,
    state: TimerState,
}

impl PomodoroTimer {
    pub fn new(config: TimerConfig) -> Self {
        Self {
            config,
            strict: false,
            completed_in_cycle: 0,
            next_phase: Phase::Focus,
            state: TimerState::Idle,
        }
    }

    // ---- queries ----------------------------------------------------------

    pub fn config(&self) -> TimerConfig {
        self.config
    }
    pub fn is_strict(&self) -> bool {
        self.strict
    }
    pub fn state(&self) -> TimerState {
        self.state
    }
    pub fn next_phase(&self) -> Phase {
        self.next_phase
    }
    pub fn completed_in_cycle(&self) -> u32 {
        self.completed_in_cycle
    }

    /// The phase currently running or paused, if any.
    pub fn active_phase(&self) -> Option<Phase> {
        match self.state {
            TimerState::Idle => None,
            TimerState::Running { phase, .. } | TimerState::Paused { phase, .. } => Some(phase),
        }
    }

    /// Blocking is enforced for the whole focus phase, paused or not.
    pub fn is_blocking(&self) -> bool {
        self.active_phase().is_some_and(Phase::is_focus)
    }

    /// True when strict mode currently forbids weakening blocking.
    pub fn is_locked(&self) -> bool {
        self.strict && self.is_blocking()
    }

    pub fn remaining_secs(&self, now: Timestamp) -> i64 {
        match self.state {
            TimerState::Idle => self.config.duration_secs(self.next_phase),
            TimerState::Running { ends_at, .. } => now.secs_until(ends_at).max(0),
            TimerState::Paused { remaining_secs, .. } => remaining_secs,
        }
    }

    pub fn total_secs(&self) -> i64 {
        self.config.duration_secs(self.active_phase().unwrap_or(self.next_phase))
    }

    /// Guard for any action that would loosen blocking (used by other
    /// aggregates' use cases too, e.g. removing a blocked site).
    pub fn ensure_can_loosen(&self) -> Result<(), DomainError> {
        if self.is_locked() { Err(DomainError::StrictModeActive) } else { Ok(()) }
    }

    // ---- commands ---------------------------------------------------------

    pub fn start(&mut self, now: Timestamp) -> Result<DomainEvent, DomainError> {
        if self.state != TimerState::Idle {
            return Err(DomainError::AlreadyRunning);
        }
        let phase = self.next_phase;
        self.state = TimerState::Running {
            phase,
            started_at: now,
            ends_at: now.plus_secs(self.config.duration_secs(phase)),
        };
        Ok(DomainEvent::PhaseStarted { phase, at: now })
    }

    pub fn pause(&mut self, now: Timestamp) -> Result<(), DomainError> {
        self.ensure_can_loosen()?;
        match self.state {
            TimerState::Running { phase, started_at, ends_at } => {
                self.state = TimerState::Paused {
                    phase,
                    started_at,
                    remaining_secs: now.secs_until(ends_at).max(0),
                };
                Ok(())
            }
            _ => Err(DomainError::NotRunning),
        }
    }

    pub fn resume(&mut self, now: Timestamp) -> Result<(), DomainError> {
        match self.state {
            TimerState::Paused { phase, started_at, remaining_secs } => {
                self.state = TimerState::Running {
                    phase,
                    started_at,
                    ends_at: now.plus_secs(remaining_secs),
                };
                Ok(())
            }
            _ => Err(DomainError::NotPaused),
        }
    }

    /// Abandon the current phase. The same phase is offered again next.
    pub fn stop(&mut self) -> Result<DomainEvent, DomainError> {
        self.ensure_can_loosen()?;
        let phase = self.active_phase().ok_or(DomainError::NotRunning)?;
        self.state = TimerState::Idle;
        Ok(DomainEvent::PhaseAbandoned { phase })
    }

    /// Move on to the following phase without counting this one.
    pub fn skip(&mut self) -> Result<DomainEvent, DomainError> {
        self.ensure_can_loosen()?;
        let phase = self.active_phase().unwrap_or(self.next_phase);
        self.state = TimerState::Idle;
        self.next_phase = self.phase_after(phase);
        Ok(DomainEvent::PhaseSkipped { phase })
    }

    /// Advance time. Completes the running phase once its end is reached.
    pub fn tick(&mut self, now: Timestamp) -> Option<DomainEvent> {
        let TimerState::Running { phase, started_at, ends_at } = self.state else {
            return None;
        };
        if now < ends_at {
            return None;
        }
        match phase {
            Phase::Focus => self.completed_in_cycle += 1,
            Phase::LongBreak => self.completed_in_cycle = 0,
            Phase::ShortBreak => {}
        }
        self.next_phase = self.phase_after(phase);
        self.state = TimerState::Idle;
        Some(DomainEvent::PhaseCompleted {
            phase,
            started_at,
            duration_secs: started_at.secs_until(ends_at),
        })
    }

    pub fn set_strict(&mut self, strict: bool) -> Result<(), DomainError> {
        if !strict {
            self.ensure_can_loosen()?;
        }
        self.strict = strict;
        Ok(())
    }

    /// New settings apply from the next phase; the running one keeps its end time.
    pub fn update_config(&mut self, config: TimerConfig) -> Result<(), DomainError> {
        self.ensure_can_loosen()?;
        self.config = config;
        Ok(())
    }

    fn phase_after(&self, phase: Phase) -> Phase {
        match phase {
            Phase::Focus if self.completed_in_cycle >= self.config.sessions_before_long_break() => {
                Phase::LongBreak
            }
            Phase::Focus => Phase::ShortBreak,
            Phase::ShortBreak | Phase::LongBreak => Phase::Focus,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: Timestamp = Timestamp(1_000_000);

    fn timer() -> PomodoroTimer {
        PomodoroTimer::new(TimerConfig::new(25, 5, 15, 2).unwrap())
    }

    fn finish(t: &mut PomodoroTimer, start: Timestamp) -> Timestamp {
        t.start(start).unwrap();
        let end = start.plus_secs(t.total_secs());
        assert!(matches!(t.tick(end), Some(DomainEvent::PhaseCompleted { .. })));
        end
    }

    #[test]
    fn focus_runs_and_completes_into_short_break() {
        let mut t = timer();
        t.start(T0).unwrap();
        assert!(t.is_blocking());
        assert_eq!(t.remaining_secs(T0.plus_secs(60)), 24 * 60);
        assert_eq!(t.tick(T0.plus_secs(24 * 60)), None);
        let ev = t.tick(T0.plus_secs(25 * 60));
        assert_eq!(
            ev,
            Some(DomainEvent::PhaseCompleted { phase: Phase::Focus, started_at: T0, duration_secs: 1500 })
        );
        assert!(!t.is_blocking());
        assert_eq!(t.next_phase(), Phase::ShortBreak);
    }

    #[test]
    fn long_break_after_configured_sessions_then_cycle_resets() {
        let mut t = timer();
        let mut now = finish(&mut t, T0); // focus 1
        now = finish(&mut t, now); // short break
        now = finish(&mut t, now); // focus 2
        assert_eq!(t.next_phase(), Phase::LongBreak);
        finish(&mut t, now);
        assert_eq!(t.completed_in_cycle(), 0);
        assert_eq!(t.next_phase(), Phase::Focus);
    }

    #[test]
    fn pause_freezes_remaining_time() {
        let mut t = timer();
        t.start(T0).unwrap();
        t.pause(T0.plus_secs(100)).unwrap();
        assert_eq!(t.remaining_secs(T0.plus_secs(10_000)), 1400);
        t.resume(T0.plus_secs(5000)).unwrap();
        assert_eq!(t.remaining_secs(T0.plus_secs(5000)), 1400);
        assert!(t.is_blocking(), "paused focus still blocks");
    }

    #[test]
    fn strict_focus_cannot_be_weakened() {
        let mut t = timer();
        t.set_strict(true).unwrap();
        t.start(T0).unwrap();
        assert!(t.is_locked());
        assert_eq!(t.pause(T0), Err(DomainError::StrictModeActive));
        assert_eq!(t.stop(), Err(DomainError::StrictModeActive));
        assert_eq!(t.skip(), Err(DomainError::StrictModeActive));
        assert_eq!(t.set_strict(false), Err(DomainError::StrictModeActive));
        assert_eq!(t.update_config(TimerConfig::default()), Err(DomainError::StrictModeActive));
        assert!(t.is_blocking());
    }

    #[test]
    fn strict_mode_unlocks_after_focus_completes() {
        let mut t = timer();
        t.set_strict(true).unwrap();
        finish(&mut t, T0);
        assert!(!t.is_locked());
        t.set_strict(false).unwrap();
    }

    #[test]
    fn strict_mode_allows_ending_breaks() {
        let mut t = timer();
        t.set_strict(true).unwrap();
        let now = finish(&mut t, T0);
        t.start(now).unwrap(); // short break
        t.skip().unwrap();
        assert_eq!(t.next_phase(), Phase::Focus);
    }

    #[test]
    fn strict_can_be_turned_on_mid_focus() {
        let mut t = timer();
        t.start(T0).unwrap();
        t.set_strict(true).unwrap();
        assert!(t.is_locked());
    }

    #[test]
    fn stop_offers_same_phase_again() {
        let mut t = timer();
        t.start(T0).unwrap();
        t.stop().unwrap();
        assert_eq!(t.state(), TimerState::Idle);
        assert_eq!(t.next_phase(), Phase::Focus);
        assert_eq!(t.completed_in_cycle(), 0);
    }

    #[test]
    fn completion_after_downtime_is_detected_on_next_tick() {
        let mut t = timer();
        t.start(T0).unwrap();
        // Service was down for hours; first tick after restart still completes.
        assert!(t.tick(T0.plus_secs(10 * 3600)).is_some());
    }

    #[test]
    fn cannot_start_twice() {
        let mut t = timer();
        t.start(T0).unwrap();
        assert_eq!(t.start(T0), Err(DomainError::AlreadyRunning));
    }
}
