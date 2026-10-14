use std::fmt;

use serde::{Deserialize, Serialize};

use crate::DomainError;

/// A normalized executable name, matched against running processes.
///
/// `Discord.exe`, `discord` and `Discord.app` all become `discord`, so one
/// entry works on Windows, macOS and Linux.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AppName(String);

impl AppName {
    pub fn parse(input: &str) -> Result<Self, DomainError> {
        let normalized = Self::normalize(input);
        if normalized.is_empty() || normalized.contains(['/', '\\']) {
            return Err(DomainError::InvalidAppName(input.trim().to_string()));
        }
        Ok(AppName(normalized))
    }

    /// Does a running process with this name belong to the blocked app?
    pub fn matches_process(&self, process_name: &str) -> bool {
        Self::normalize(process_name) == self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn normalize(s: &str) -> String {
        let s = s.trim().to_lowercase();
        let s = s.strip_suffix(".exe").or_else(|| s.strip_suffix(".app")).unwrap_or(&s);
        s.trim().to_string()
    }
}

impl fmt::Display for AppName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for AppName {
    type Error = DomainError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        AppName::parse(&s)
    }
}

impl From<AppName> for String {
    fn from(a: AppName) -> Self {
        a.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_across_platforms() {
        let app = AppName::parse("Discord.exe").unwrap();
        assert!(app.matches_process("discord.exe"));
        assert!(app.matches_process("Discord"));
        assert!(!app.matches_process("discordptb.exe"));
    }

    #[test]
    fn rejects_paths_and_empty() {
        assert!(AppName::parse("  ").is_err());
        assert!(AppName::parse(".exe").is_err());
        assert!(AppName::parse(r"C:\Games\steam.exe").is_err());
    }
}
