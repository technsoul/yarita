use serde::{Deserialize, Serialize};

use crate::Timestamp;

/// One completed focus session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusRecord {
    pub started_at: Timestamp,
    pub duration_secs: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatsSummary {
    pub today_sessions: u32,
    pub today_minutes: u32,
    pub total_sessions: u32,
    pub total_minutes: u32,
    /// Focus minutes per day, oldest first, the last entry is today.
    pub last_7_days_minutes: [u32; 7],
    /// Consecutive days with at least one session, ending today (or yesterday
    /// if today has none yet).
    pub streak_days: u32,
}

impl StatsSummary {
    pub fn compute(records: &[FocusRecord], now: Timestamp, utc_offset_secs: i64) -> Self {
        let today = now.local_day(utc_offset_secs);
        let mut s = StatsSummary::default();
        let mut days: Vec<i64> = Vec::with_capacity(records.len());

        for r in records {
            let minutes = (r.duration_secs / 60) as u32;
            let day = r.started_at.local_day(utc_offset_secs);
            s.total_sessions += 1;
            s.total_minutes += minutes;
            if day == today {
                s.today_sessions += 1;
                s.today_minutes += minutes;
            }
            let age = today - day;
            if (0..7).contains(&age) {
                s.last_7_days_minutes[6 - age as usize] += minutes;
            }
            days.push(day);
        }

        days.sort_unstable();
        days.dedup();
        let mut expected = if days.last() == Some(&today) { today } else { today - 1 };
        for &d in days.iter().rev() {
            if d == expected {
                s.streak_days += 1;
                expected -= 1;
            } else if d < expected {
                break;
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;

    fn rec(day: i64, mins: i64) -> FocusRecord {
        FocusRecord { started_at: Timestamp(day * DAY + 3600), duration_secs: mins * 60 }
    }

    #[test]
    fn summarizes_today_week_and_streak() {
        let today = 20_000;
        let records = [
            rec(today, 25),
            rec(today, 25),
            rec(today - 1, 50),
            rec(today - 2, 25),
            rec(today - 4, 25), // breaks the streak
            rec(today - 30, 25),
        ];
        let s = StatsSummary::compute(&records, Timestamp(today * DAY + 7200), 0);
        assert_eq!(s.today_sessions, 2);
        assert_eq!(s.today_minutes, 50);
        assert_eq!(s.total_sessions, 6);
        assert_eq!(s.last_7_days_minutes, [0, 0, 25, 0, 25, 50, 50]);
        assert_eq!(s.streak_days, 3);
    }

    #[test]
    fn streak_survives_until_today_gets_a_session() {
        let today = 20_000;
        let records = [rec(today - 1, 25), rec(today - 2, 25)];
        let s = StatsSummary::compute(&records, Timestamp(today * DAY), 0);
        assert_eq!(s.streak_days, 2);
    }

    #[test]
    fn respects_local_offset() {
        // 23:30 UTC on day N is 01:30 on day N+1 at UTC+2.
        let r = FocusRecord { started_at: Timestamp(20_000 * DAY + 23 * 3600 + 1800), duration_secs: 1500 };
        let now = Timestamp(20_001 * DAY + 1800);
        let s = StatsSummary::compute(&[r], now, 2 * 3600);
        assert_eq!(s.today_sessions, 1);
    }
}
