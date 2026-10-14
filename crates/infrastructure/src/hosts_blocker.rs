//! Website blocking through the system hosts file.
//!
//! The app owns one marked section of the file and never touches other lines.
//! The file is only rewritten when the section needs to change, so calling
//! `block` every tick is cheap and also undoes manual edits during a session.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use focus_application::ports::{PortError, SiteBlocker};
use focus_domain::blocklist::Site;

const BEGIN: &str = "# >>> focus-timer (managed by Focus Timer, do not edit)";
const END: &str = "# <<< focus-timer";

pub struct HostsFileBlocker {
    path: PathBuf,
    flush_dns: bool,
}

impl HostsFileBlocker {
    pub fn new(path: PathBuf, flush_dns: bool) -> Self {
        Self { path, flush_dns }
    }

    fn sync(&mut self, sites: &[Site]) -> Result<(), PortError> {
        let current = fs::read_to_string(&self.path)
            .map_err(|e| PortError(format!("Cannot read hosts file {}: {e}", self.path.display())))?;
        let updated = render(&current, sites);
        if updated == current {
            return Ok(());
        }
        fs::write(&self.path, updated).map_err(|e| {
            PortError(format!(
                "Cannot edit the hosts file ({e}). Website blocking needs the service to run as administrator."
            ))
        })?;
        if self.flush_dns {
            flush_dns_cache();
        }
        Ok(())
    }
}

impl SiteBlocker for HostsFileBlocker {
    fn block(&mut self, sites: &[Site]) -> Result<(), PortError> {
        self.sync(sites)
    }

    fn unblock_all(&mut self) -> Result<(), PortError> {
        self.sync(&[])
    }
}

/// Returns `existing` with the managed section replaced by entries for
/// `sites` (or removed when `sites` is empty). Other lines are untouched.
pub fn render(existing: &str, sites: &[Site]) -> String {
    let nl = if existing.contains("\r\n") { "\r\n" } else { "\n" };

    let mut kept: Vec<&str> = Vec::new();
    let mut in_section = false;
    for line in existing.lines() {
        match line.trim() {
            BEGIN => in_section = true,
            END if in_section => in_section = false,
            _ if !in_section => kept.push(line),
            _ => {}
        }
    }
    while kept.last().is_some_and(|l| l.trim().is_empty()) {
        kept.pop();
    }

    let mut out = kept.join(nl);
    if !out.is_empty() {
        out.push_str(nl);
    }
    if !sites.is_empty() {
        if !out.is_empty() {
            out.push_str(nl);
        }
        out.push_str(BEGIN);
        out.push_str(nl);
        for site in sites {
            for host in hosts_for(site) {
                out.push_str(&format!("0.0.0.0 {host}{nl}:: {host}{nl}"));
            }
        }
        out.push_str(END);
        out.push_str(nl);
    }
    out
}

fn hosts_for(site: &Site) -> Vec<String> {
    let s = site.as_str();
    vec![s.to_string(), format!("www.{s}"), format!("m.{s}")]
}

fn flush_dns_cache() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = Command::new("ipconfig").arg("/flushdns").creation_flags(CREATE_NO_WINDOW).output();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("dscacheutil").arg("-flushcache").output();
        let _ = Command::new("killall").args(["-HUP", "mDNSResponder"]).output();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("resolvectl").arg("flush-caches").output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sites(names: &[&str]) -> Vec<Site> {
        names.iter().map(|n| Site::parse(n).unwrap()).collect()
    }

    const BASE: &str = "127.0.0.1 localhost\n# my own entry\n10.0.0.5 nas.local\n";

    #[test]
    fn adds_and_removes_section_without_touching_other_lines() {
        let blocked = render(BASE, &sites(&["youtube.com"]));
        assert!(blocked.starts_with(BASE));
        assert!(blocked.contains("0.0.0.0 youtube.com\n"));
        assert!(blocked.contains("0.0.0.0 www.youtube.com\n"));
        assert!(blocked.contains(":: youtube.com\n"));
        assert_eq!(render(&blocked, &[]), BASE);
    }

    #[test]
    fn rendering_is_idempotent() {
        let once = render(BASE, &sites(&["a.com", "b.org"]));
        assert_eq!(render(&once, &sites(&["a.com", "b.org"])), once);
        assert_eq!(render(BASE, &[]), BASE);
    }

    #[test]
    fn replaces_old_section_and_keeps_crlf() {
        let base = BASE.replace('\n', "\r\n");
        let first = render(&base, &sites(&["a.com"]));
        let second = render(&first, &sites(&["b.com"]));
        assert!(!second.contains("a.com"));
        assert!(second.contains("0.0.0.0 b.com\r\n"));
        assert!(!second.replace("\r\n", "").contains('\n'));
        assert_eq!(render(&second, &[]), base);
    }

    #[test]
    fn handles_empty_file() {
        let out = render("", &sites(&["a.com"]));
        assert!(out.starts_with(BEGIN));
        assert_eq!(render(&out, &[]), "");
    }
}
