use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// The action would weaken blocking while a strict focus session runs.
    StrictModeActive,
    AlreadyRunning,
    NotRunning,
    NotPaused,
    InvalidDomain(String),
    InvalidAppName(String),
    InvalidConfig(String),
    AlreadyBlocked(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::StrictModeActive => {
                write!(f, "Strict mode is on: this is locked until the focus session ends")
            }
            DomainError::AlreadyRunning => write!(f, "A session is already running"),
            DomainError::NotRunning => write!(f, "No session is running"),
            DomainError::NotPaused => write!(f, "The session is not paused"),
            DomainError::InvalidDomain(s) => write!(f, "Not a valid website: {s}"),
            DomainError::InvalidAppName(s) => write!(f, "Not a valid app name: {s}"),
            DomainError::InvalidConfig(s) => write!(f, "Invalid settings: {s}"),
            DomainError::AlreadyBlocked(s) => write!(f, "{s} is already on the list"),
        }
    }
}

impl std::error::Error for DomainError {}
