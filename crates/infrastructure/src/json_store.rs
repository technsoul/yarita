//! File-based repositories: one JSON file per aggregate, plus an append-only
//! JSON Lines file for focus records.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use yarita_domain::blocklist::Blocklist;
use yarita_domain::repositories::{
    BlocklistRepository, FocusRecordRepository, RepositoryError, TimerRepository,
};
use yarita_domain::session::PomodoroTimer;
use yarita_domain::stats::FocusRecord;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct JsonStore {
    dir: PathBuf,
}

impl JsonStore {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, RepositoryError> {
        let dir = dir.into();
        fs::create_dir_all(&dir).map_err(|e| err(&dir, e))?;
        Ok(Self { dir })
    }

    fn load<T: DeserializeOwned>(&self, file: &str) -> Result<Option<T>, RepositoryError> {
        let path = self.dir.join(file);
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map(Some).map_err(|e| err(&path, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(err(&path, e)),
        }
    }

    /// Write to a temp file and rename, so a crash never leaves half a file.
    fn save<T: Serialize>(&self, file: &str, value: &T) -> Result<(), RepositoryError> {
        let path = self.dir.join(file);
        let tmp = self.dir.join(format!("{file}.tmp"));
        let json = serde_json::to_vec_pretty(value).map_err(|e| err(&path, e))?;
        fs::write(&tmp, json).map_err(|e| err(&tmp, e))?;
        fs::rename(&tmp, &path).map_err(|e| err(&path, e))
    }
}

fn err(path: &Path, e: impl std::fmt::Display) -> RepositoryError {
    RepositoryError(format!("{}: {e}", path.display()))
}

const TIMER: &str = "timer.json";
const BLOCKLIST: &str = "blocklist.json";
const RECORDS: &str = "focus_records.jsonl";

impl TimerRepository for JsonStore {
    fn load(&self) -> Result<Option<PomodoroTimer>, RepositoryError> {
        JsonStore::load(self, TIMER)
    }
    fn save(&self, timer: &PomodoroTimer) -> Result<(), RepositoryError> {
        JsonStore::save(self, TIMER, timer)
    }
}

impl BlocklistRepository for JsonStore {
    fn load(&self) -> Result<Option<Blocklist>, RepositoryError> {
        JsonStore::load(self, BLOCKLIST)
    }
    fn save(&self, list: &Blocklist) -> Result<(), RepositoryError> {
        JsonStore::save(self, BLOCKLIST, list)
    }
}

impl FocusRecordRepository for JsonStore {
    fn all(&self) -> Result<Vec<FocusRecord>, RepositoryError> {
        let path = self.dir.join(RECORDS);
        match fs::read_to_string(&path) {
            // Skip unreadable lines rather than losing all history.
            Ok(text) => Ok(text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(err(&path, e)),
        }
    }

    fn append(&self, record: FocusRecord) -> Result<(), RepositoryError> {
        let path = self.dir.join(RECORDS);
        let mut line = serde_json::to_string(&record).map_err(|e| err(&path, e))?;
        line.push('\n');
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| f.write_all(line.as_bytes()))
            .map_err(|e| err(&path, e))
    }
}

#[cfg(test)]
mod tests {
    use yarita_domain::Timestamp;
    use yarita_domain::blocklist::Site;
    use yarita_domain::session::TimerConfig;

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yarita-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn round_trips_aggregates_and_records() {
        let dir = temp_dir("store");
        let store = JsonStore::open(&dir).unwrap();

        assert_eq!(TimerRepository::load(&store).unwrap(), None);
        let mut timer = PomodoroTimer::new(TimerConfig::default());
        timer.set_strict(true).unwrap();
        timer.start(Timestamp(100)).unwrap();
        TimerRepository::save(&store, &timer).unwrap();
        assert_eq!(TimerRepository::load(&store).unwrap(), Some(timer));

        let mut list = Blocklist::default();
        list.add_site(Site::parse("reddit.com").unwrap()).unwrap();
        BlocklistRepository::save(&store, &list).unwrap();
        assert_eq!(BlocklistRepository::load(&store).unwrap(), Some(list));

        let r = FocusRecord { started_at: Timestamp(5), duration_secs: 1500 };
        store.append(r).unwrap();
        store.append(r).unwrap();
        assert_eq!(store.all().unwrap(), vec![r, r]);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tampered_blocklist_is_rejected_not_silently_accepted() {
        let dir = temp_dir("tamper");
        let store = JsonStore::open(&dir).unwrap();
        fs::write(dir.join(BLOCKLIST), r#"{"sites":["not a site"],"apps":[]}"#).unwrap();
        assert!(BlocklistRepository::load(&store).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
