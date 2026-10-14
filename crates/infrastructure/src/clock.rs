use focus_application::ports::Clock;
use focus_domain::Timestamp;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp(chrono::Utc::now().timestamp())
    }

    fn utc_offset_secs(&self) -> i64 {
        i64::from(chrono::Local::now().offset().local_minus_utc())
    }
}
