//! App blocking at launch through Image File Execution Options (Windows).
//!
//! For each blocked `name.exe`, the blocker sets a `Debugger` value under
//! `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\name.exe`.
//! Windows then starts `yarita-service.exe --blocked <path> …` instead of the
//! app, which shows a message and exits, so the app never opens.
//!
//! It only matches the file name, so a renamed exe gets through; the process
//! blocker still closes those. Keys may already hold Windows' own settings,
//! so only the `Debugger` value and a marker value are touched, and an app
//! that already has someone else's debugger is left alone.

use yarita_domain::blocklist::AppName;

/// Command-line flag that runs `yarita-service` as the "app blocked" helper.
pub const BLOCKED_FLAG: &str = "--blocked";

/// Programs that must never be blocked: Windows itself, the shells used to
/// start the service, regedit (to recover by hand), and Yarita.
const PROTECTED: &[&str] = &[
    "csrss", "dwm", "explorer", "fontdrvhost", "lsass", "services", "sihost", "smss", "svchost",
    "taskhostw", "wininit", "winlogon", "logonui", "userinit", "ctfmon", "conhost", "consent",
    "runtimebroker", "searchhost", "startmenuexperiencehost", "shellexperiencehost",
    "powershell", "pwsh", "cmd", "regedit", "yarita-service", "yarita",
];

/// The IFEO subkey for `app`, or `None` if it must not be blocked.
pub fn exe_key_name(app: &AppName) -> Option<String> {
    let name = app.as_str();
    if PROTECTED.contains(&name) {
        return None;
    }
    Some(format!("{name}.exe"))
}

#[cfg(windows)]
pub use imp::LaunchBlocker;

#[cfg(windows)]
mod imp {
    use std::collections::BTreeSet;
    use std::path::Path;

    use yarita_application::ports::{AppBlocker, PortError};
    use yarita_domain::blocklist::AppName;

    use super::{BLOCKED_FLAG, exe_key_name};
    use crate::registry::{self, Registry, View};

    const IFEO: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";
    const DEBUGGER: &str = "Debugger";
    const MARKER: &str = "YaritaBlock";
    /// 32-bit programs (Steam, for one) read the 32-bit view when they start an app.
    const VIEWS: [View; 2] = [View::Native, View::Wow32];

    pub struct LaunchBlocker {
        registry: Registry,
        debugger: String,
        /// Exe names we have set, so each tick only touches what changed.
        /// `None` until a first scan finds blocks left by a previous run.
        applied: Option<BTreeSet<String>>,
    }

    impl LaunchBlocker {
        /// `helper` is the executable that tells the user an app is blocked.
        pub fn new(registry: Registry, helper: &Path) -> Self {
            let debugger = format!("\"{}\" {BLOCKED_FLAG}", helper.display());
            Self { registry, debugger, applied: None }
        }

        fn sync(&mut self, wanted: BTreeSet<String>) -> Result<(), PortError> {
            let applied = match self.applied.take() {
                Some(set) => set,
                None => self.scan()?,
            };
            let mut first_error = None;
            let mut now_applied = BTreeSet::new();
            for exe in applied.union(&wanted) {
                let result = if wanted.contains(exe) { self.block(exe) } else { self.unblock(exe).map(|()| false) };
                match result {
                    Ok(true) => {
                        now_applied.insert(exe.clone());
                    }
                    Ok(false) => {}
                    Err(e) => {
                        // Retry from a full scan next time.
                        first_error.get_or_insert(e);
                    }
                }
            }
            if first_error.is_none() {
                self.applied = Some(now_applied);
            }
            first_error.map_or(Ok(()), Err)
        }

        /// Finds exes we blocked earlier, e.g. before a crash.
        fn scan(&self) -> Result<BTreeSet<String>, PortError> {
            let mut found = BTreeSet::new();
            for view in VIEWS {
                let Some(root) = self.registry.open(IFEO, view)? else { continue };
                let names: Vec<String> = root.keys().map(|k| k.collect()).unwrap_or_default();
                for name in names {
                    let marked = root.open(&name).is_ok_and(|key| key.get_u32(MARKER).is_ok());
                    if marked {
                        found.insert(name.to_lowercase());
                    }
                }
            }
            Ok(found)
        }

        /// Sets our debugger for `exe` in every view where no one else has
        /// one. Returns whether any view is now blocked.
        fn block(&self, exe: &str) -> Result<bool, PortError> {
            let path = format!(r"{IFEO}\{exe}");
            let mut blocked = false;
            for view in VIEWS {
                let key = self.registry.create(&path, view)?;
                let ours = key.get_u32(MARKER).is_ok();
                match registry::get_string(&key, DEBUGGER) {
                    Some(_) if !ours => continue,
                    Some(current) if current == self.debugger => {}
                    _ => {
                        // Marker first, so a crash in between never leaves
                        // a debugger we don't recognize as ours.
                        key.set_u32(MARKER, 1).map_err(|e| registry::err(&path, e))?;
                        key.set_string(DEBUGGER, &self.debugger).map_err(|e| registry::err(&path, e))?;
                    }
                }
                blocked = true;
            }
            Ok(blocked)
        }

        fn unblock(&self, exe: &str) -> Result<(), PortError> {
            let path = format!(r"{IFEO}\{exe}");
            for view in VIEWS {
                let Some(key) = self.registry.open(&path, view)? else { continue };
                if key.get_u32(MARKER).is_err() {
                    continue;
                }
                registry::remove_value(&key, DEBUGGER)?;
                registry::remove_value(&key, MARKER)?;
                if registry::is_empty(&key) {
                    drop(key);
                    self.registry.remove_tree(&path, view)?;
                }
            }
            Ok(())
        }
    }

    impl AppBlocker for LaunchBlocker {
        fn enforce(&mut self, apps: &[AppName]) -> Result<Vec<String>, PortError> {
            self.sync(apps.iter().filter_map(exe_key_name).collect())?;
            Ok(Vec::new())
        }

        fn release_all(&mut self) -> Result<(), PortError> {
            // Skip the registry entirely once there is nothing left to undo.
            if self.applied.as_ref().is_some_and(BTreeSet::is_empty) {
                return Ok(());
            }
            self.sync(BTreeSet::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_apps_to_exe_keys() {
        assert_eq!(exe_key_name(&AppName::parse("Discord").unwrap()).as_deref(), Some("discord.exe"));
        assert_eq!(exe_key_name(&AppName::parse("steam.exe").unwrap()).as_deref(), Some("steam.exe"));
    }

    #[test]
    fn never_blocks_system_or_own_programs() {
        for name in ["explorer", "Explorer.exe", "svchost", "powershell", "yarita-service", "regedit"] {
            assert_eq!(exe_key_name(&AppName::parse(name).unwrap()), None, "{name}");
        }
    }
}
