//! Starts the background service with admin rights.

use std::path::PathBuf;
use std::process::Command;

fn service_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let name = if cfg!(windows) { "focus-service.exe" } else { "focus-service" };
    let path = exe.with_file_name(name);
    if path.exists() { Ok(path) } else { Err(format!("{} not found", path.display())) }
}

/// Launch the service. In development (`FOCUS_HOSTS_FILE` set) it runs
/// without elevation so the environment overrides are inherited.
pub fn start_service() -> Result<(), String> {
    let path = service_path()?;
    let dev = std::env::var_os("FOCUS_HOSTS_FILE").is_some();
    let mut command = if dev { Command::new(&path) } else { elevated(&path) };
    hide_console(&mut command);
    command.spawn().map(drop).map_err(|e| format!("Could not start the service: {e}"))
}

#[cfg(windows)]
fn elevated(path: &std::path::Path) -> Command {
    // UAC prompt via ShellExecute "runas".
    let quoted = path.display().to_string().replace('\'', "''");
    let mut c = Command::new("powershell");
    c.args([
        "-NoProfile",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &format!("Start-Process -FilePath '{quoted}' -Verb RunAs -WindowStyle Hidden"),
    ]);
    c
}

#[cfg(target_os = "macos")]
fn elevated(path: &std::path::Path) -> Command {
    let p = path.display().to_string().replace('\\', "\\\\").replace('"', "\\\"").replace('\'', "'\\''");
    let script = format!("do shell script \"'{p}' > /dev/null 2>&1 &\" with administrator privileges");
    let mut c = Command::new("osascript");
    c.args(["-e", &script]);
    c
}

#[cfg(all(unix, not(target_os = "macos")))]
fn elevated(path: &std::path::Path) -> Command {
    let mut c = Command::new("pkexec");
    c.arg(path);
    c
}

fn hide_console(_command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        _command.creation_flags(CREATE_NO_WINDOW);
    }
}
