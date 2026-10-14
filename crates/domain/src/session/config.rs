use serde::{Deserialize, Serialize};

use super::Phase;
use crate::DomainError;

/// Validated Pomodoro settings. Construct with [`TimerConfig::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawConfig", into = "RawConfig")]
pub struct TimerConfig {
    focus_minutes: u32,
    short_break_minutes: u32,
    long_break_minutes: u32,
    sessions_before_long_break: u32,
}

impl TimerConfig {
    pub fn new(
        focus_minutes: u32,
        short_break_minutes: u32,
        long_break_minutes: u32,
        sessions_before_long_break: u32,
    ) -> Result<Self, DomainError> {
        let check = |name: &str, v: u32, max: u32| {
            if (1..=max).contains(&v) {
                Ok(())
            } else {
                Err(DomainError::InvalidConfig(format!("{name} must be between 1 and {max}")))
            }
        };
        check("Focus length", focus_minutes, 240)?;
        check("Short break", short_break_minutes, 60)?;
        check("Long break", long_break_minutes, 120)?;
        check("Sessions before a long break", sessions_before_long_break, 12)?;
        Ok(Self {
            focus_minutes,
            short_break_minutes,
            long_break_minutes,
            sessions_before_long_break,
        })
    }

    pub fn focus_minutes(&self) -> u32 {
        self.focus_minutes
    }
    pub fn short_break_minutes(&self) -> u32 {
        self.short_break_minutes
    }
    pub fn long_break_minutes(&self) -> u32 {
        self.long_break_minutes
    }
    pub fn sessions_before_long_break(&self) -> u32 {
        self.sessions_before_long_break
    }

    pub fn duration_secs(&self, phase: Phase) -> i64 {
        let minutes = match phase {
            Phase::Focus => self.focus_minutes,
            Phase::ShortBreak => self.short_break_minutes,
            Phase::LongBreak => self.long_break_minutes,
        };
        i64::from(minutes) * 60
    }
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self::new(25, 5, 15, 4).expect("defaults are valid")
    }
}

#[derive(Serialize, Deserialize)]
struct RawConfig {
    focus_minutes: u32,
    short_break_minutes: u32,
    long_break_minutes: u32,
    sessions_before_long_break: u32,
}

impl TryFrom<RawConfig> for TimerConfig {
    type Error = DomainError;
    fn try_from(r: RawConfig) -> Result<Self, Self::Error> {
        TimerConfig::new(
            r.focus_minutes,
            r.short_break_minutes,
            r.long_break_minutes,
            r.sessions_before_long_break,
        )
    }
}

impl From<TimerConfig> for RawConfig {
    fn from(c: TimerConfig) -> Self {
        RawConfig {
            focus_minutes: c.focus_minutes,
            short_break_minutes: c.short_break_minutes,
            long_break_minutes: c.long_break_minutes,
            sessions_before_long_break: c.sessions_before_long_break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_and_out_of_range() {
        assert!(TimerConfig::new(0, 5, 15, 4).is_err());
        assert!(TimerConfig::new(25, 5, 15, 13).is_err());
        assert!(TimerConfig::new(241, 5, 15, 4).is_err());
    }

    #[test]
    fn deserialization_path_validates() {
        let raw = RawConfig {
            focus_minutes: 0,
            short_break_minutes: 5,
            long_break_minutes: 15,
            sessions_before_long_break: 4,
        };
        assert!(TimerConfig::try_from(raw).is_err());
    }
}
