//! Wire contract between the UI and the background service.
//!
//! Newline-delimited JSON over a localhost TCP socket. These are plain DTOs;
//! every rule is enforced by the service, so a misbehaving client cannot
//! bypass strict mode.

use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub const SERVICE_ADDR: &str = "127.0.0.1:47821";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Snapshot,
    Start,
    Pause,
    Resume,
    Stop,
    Skip,
    SetStrict(bool),
    UpdateConfig(ConfigDto),
    AddSite(String),
    RemoveSite(String),
    AddApp(String),
    RemoveApp(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok(Snapshot),
    /// The request was refused; the snapshot shows the unchanged state.
    Refused { reason: String, snapshot: Snapshot },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseDto {
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusDto {
    Idle,
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigDto {
    pub focus_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    pub sessions_before_long_break: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsDto {
    pub today_sessions: u32,
    pub today_minutes: u32,
    pub total_sessions: u32,
    pub total_minutes: u32,
    pub last_7_days_minutes: [u32; 7],
    pub streak_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Phase running or paused, or the one that starts next when idle.
    pub phase: PhaseDto,
    pub status: StatusDto,
    pub remaining_secs: i64,
    pub total_secs: i64,
    pub completed_in_cycle: u32,
    pub strict: bool,
    /// Strict focus in progress: loosening actions are refused.
    pub locked: bool,
    pub blocking: bool,
    pub config: ConfigDto,
    pub sites: Vec<String>,
    pub apps: Vec<String>,
    pub stats: StatsDto,
    /// Latest enforcement problem (e.g. no admin rights), if any.
    pub enforcement_error: Option<String>,
    /// Increments every time a phase completes, so clients can notify once.
    pub completions: u64,
}

pub fn write_message<T: Serialize>(w: &mut impl Write, msg: &T) -> io::Result<()> {
    serde_json::to_writer(&mut *w, msg)?;
    w.write_all(b"\n")?;
    w.flush()
}

/// Reads one message. Returns `Ok(None)` when the peer closed the connection.
pub fn read_message<T: DeserializeOwned>(r: &mut impl BufRead) -> io::Result<Option<T>> {
    let mut line = String::new();
    if r.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    serde_json::from_str(&line).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
