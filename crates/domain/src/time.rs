use serde::{Deserialize, Serialize};

/// Seconds since the Unix epoch (UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub fn plus_secs(self, secs: i64) -> Self {
        Timestamp(self.0 + secs)
    }

    pub fn secs_until(self, later: Timestamp) -> i64 {
        later.0 - self.0
    }

    /// Day number in local time, given the local UTC offset in seconds.
    pub fn local_day(self, utc_offset_secs: i64) -> i64 {
        (self.0 + utc_offset_secs).div_euclid(86_400)
    }
}
