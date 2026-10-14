use std::fmt;

use serde::{Deserialize, Serialize};

use crate::DomainError;

/// A normalized website host such as `youtube.com`.
///
/// Accepts loose input (`https://www.YouTube.com/watch`) and keeps the bare
/// host. The `www.` variant is blocked alongside it by the blocker adapter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Site(String);

impl Site {
    pub fn parse(input: &str) -> Result<Self, DomainError> {
        let invalid = || DomainError::InvalidDomain(input.trim().to_string());

        let mut s = input.trim().to_ascii_lowercase();
        if let Some(i) = s.find("://") {
            s.drain(..i + 3);
        }
        if let Some(i) = s.find(['/', '?', '#', ':']) {
            s.truncate(i);
        }
        let s = s.trim_end_matches('.');
        let s = s.strip_prefix("www.").unwrap_or(s);

        let valid_label = |l: &str| {
            (1..=63).contains(&l.len())
                && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !l.starts_with('-')
                && !l.ends_with('-')
        };
        if s.len() > 253 || !s.contains('.') || !s.split('.').all(valid_label) {
            return Err(invalid());
        }
        Ok(Site(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Site {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Site {
    type Error = DomainError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Site::parse(&s)
    }
}

impl From<Site> for String {
    fn from(s: Site) -> Self {
        s.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_urls() {
        for input in [
            "youtube.com",
            "  YouTube.com ",
            "https://www.youtube.com/watch?v=x",
            "http://youtube.com:443",
            "youtube.com.",
        ] {
            assert_eq!(Site::parse(input).unwrap().as_str(), "youtube.com", "{input}");
        }
    }

    #[test]
    fn keeps_subdomains() {
        assert_eq!(Site::parse("m.reddit.com").unwrap().as_str(), "m.reddit.com");
    }

    #[test]
    fn rejects_garbage() {
        for input in ["", "localhost", "has space.com", "-bad.com", "a..b", "exa_mple.com"] {
            assert!(Site::parse(input).is_err(), "{input}");
        }
    }
}
