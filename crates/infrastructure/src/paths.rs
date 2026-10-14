//! System locations. Both can be overridden through environment variables,
//! which is how development runs without touching the real hosts file.

use std::path::PathBuf;

/// `FOCUS_HOSTS_FILE` overrides the system hosts file.
pub fn hosts_file() -> (PathBuf, bool) {
    if let Some(p) = std::env::var_os("FOCUS_HOSTS_FILE") {
        return (PathBuf::from(p), false);
    }
    #[cfg(windows)]
    let path = {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        PathBuf::from(root).join(r"System32\drivers\etc\hosts")
    };
    #[cfg(not(windows))]
    let path = PathBuf::from("/etc/hosts");
    (path, true)
}

/// Machine-wide state directory (admin-only by default). `FOCUS_DATA_DIR` overrides it.
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("FOCUS_DATA_DIR") {
        return PathBuf::from(p);
    }
    #[cfg(windows)]
    {
        let root = std::env::var_os("ProgramData").unwrap_or_else(|| r"C:\ProgramData".into());
        PathBuf::from(root).join("FocusTimer")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/FocusTimer")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        PathBuf::from("/var/lib/focus-timer")
    }
}

/// `FOCUS_REGISTRY_SANDBOX` moves every registry write from `HKLM` to
/// `HKCU\<value>`, where browsers and Windows don't look.
pub fn registry_sandbox() -> Option<String> {
    std::env::var("FOCUS_REGISTRY_SANDBOX").ok().filter(|s| !s.trim().is_empty())
}
