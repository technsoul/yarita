use focus_domain::session::{Phase, TimerConfig, TimerState};
use focus_domain::stats::StatsSummary;
use focus_protocol::{ConfigDto, PhaseDto, StatsDto, StatusDto};

pub fn phase(p: Phase) -> PhaseDto {
    match p {
        Phase::Focus => PhaseDto::Focus,
        Phase::ShortBreak => PhaseDto::ShortBreak,
        Phase::LongBreak => PhaseDto::LongBreak,
    }
}

pub fn status(s: TimerState) -> StatusDto {
    match s {
        TimerState::Idle => StatusDto::Idle,
        TimerState::Running { .. } => StatusDto::Running,
        TimerState::Paused { .. } => StatusDto::Paused,
    }
}

pub fn config(c: TimerConfig) -> ConfigDto {
    ConfigDto {
        focus_minutes: c.focus_minutes(),
        short_break_minutes: c.short_break_minutes(),
        long_break_minutes: c.long_break_minutes(),
        sessions_before_long_break: c.sessions_before_long_break(),
    }
}

pub fn stats(s: StatsSummary) -> StatsDto {
    StatsDto {
        today_sessions: s.today_sessions,
        today_minutes: s.today_minutes,
        total_sessions: s.total_sessions,
        total_minutes: s.total_minutes,
        last_7_days_minutes: s.last_7_days_minutes,
        streak_days: s.streak_days,
    }
}
