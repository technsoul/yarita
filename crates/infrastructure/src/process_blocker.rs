//! App blocking by closing matching processes.

use std::path::Path;

use focus_application::ports::{AppBlocker, PortError};
use focus_domain::blocklist::AppName;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub struct ProcessBlocker {
    system: System,
    own_pid: u32,
}

impl ProcessBlocker {
    pub fn new() -> Self {
        Self { system: System::new(), own_pid: std::process::id() }
    }
}

impl Default for ProcessBlocker {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBlocker for ProcessBlocker {
    fn enforce(&mut self, apps: &[AppName]) -> Result<Vec<String>, PortError> {
        if apps.is_empty() {
            return Ok(Vec::new());
        }
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
        );

        let mut closed = Vec::new();
        for (pid, process) in self.system.processes() {
            if pid.as_u32() == self.own_pid {
                continue;
            }
            let name = process.name().to_string_lossy();
            // Linux truncates process names to 15 chars, so also check the
            // executable's file name.
            let exe_stem = process.exe().and_then(Path::file_stem).map(|s| s.to_string_lossy());
            let blocked = apps.iter().any(|app| {
                app.matches_process(&name) || exe_stem.as_deref().is_some_and(|s| app.matches_process(s))
            });
            if blocked && process.kill() {
                closed.push(name.into_owned());
            }
        }
        Ok(closed)
    }

    fn release_all(&mut self) -> Result<(), PortError> {
        Ok(())
    }
}
