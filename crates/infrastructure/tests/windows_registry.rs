//! Runs the registry blockers against a throwaway `HKCU` sandbox.
#![cfg(windows)]

use std::path::Path;

use focus_application::ports::{AppBlocker, SiteBlocker};
use focus_domain::blocklist::{AppName, Site};
use focus_infrastructure::{BrowserPolicyBlocker, LaunchBlocker, Registry};
use windows_registry::CURRENT_USER;

struct Sandbox(String);

impl Sandbox {
    fn new(name: &str) -> Self {
        Sandbox(format!(r"Software\FocusTimerTest\{name}-{}", std::process::id()))
    }
    fn registry(&self) -> Registry {
        Registry::new(Some(self.0.clone()))
    }
    fn key(&self, path: &str) -> Option<windows_registry::Key> {
        CURRENT_USER.open(format!(r"{}\{path}", self.0)).ok()
    }
    fn create(&self, path: &str) -> windows_registry::Key {
        CURRENT_USER.create(format!(r"{}\{path}", self.0)).unwrap()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = CURRENT_USER.remove_tree(&self.0);
    }
}

const CHROME: &str = r"SOFTWARE\Policies\Google\Chrome";
const IFEO: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";

fn sites(names: &[&str]) -> Vec<Site> {
    names.iter().map(|n| Site::parse(n).unwrap()).collect()
}

#[test]
fn browser_policies_are_added_and_fully_restored() {
    let sb = Sandbox::new("policy");
    // The user already has a policy of their own.
    let list = sb.create(&format!(r"{CHROME}\URLBlocklist"));
    list.set_string("1", "corp.example").unwrap();
    sb.create(CHROME).set_string("DnsOverHttpsMode", "automatic").unwrap();

    let mut blocker = BrowserPolicyBlocker::new(sb.registry());
    blocker.block(&sites(&["youtube.com", "reddit.com"])).unwrap();
    blocker.block(&sites(&["youtube.com", "reddit.com"])).unwrap(); // idempotent

    let list = sb.key(&format!(r"{CHROME}\URLBlocklist")).unwrap();
    let entries: Vec<String> = (1..=4).filter_map(|i| list.get_string(i.to_string()).ok()).collect();
    assert_eq!(entries, ["corp.example", "youtube.com", "reddit.com"]);
    assert_eq!(sb.key(CHROME).unwrap().get_string("DnsOverHttpsMode").unwrap(), "off");
    let firefox = sb.key(r"SOFTWARE\Policies\Mozilla\Firefox\WebsiteFilter\Block").unwrap();
    assert_eq!(firefox.get_string("1").unwrap(), "*://*.youtube.com/*");

    blocker.block(&sites(&["x.com"])).unwrap();
    let list = sb.key(&format!(r"{CHROME}\URLBlocklist")).unwrap();
    assert_eq!(list.get_string("2").unwrap(), "x.com");
    assert!(list.get_string("3").is_err(), "no stale entries");

    blocker.unblock_all().unwrap();
    let list = sb.key(&format!(r"{CHROME}\URLBlocklist")).unwrap();
    assert_eq!(list.get_string("1").unwrap(), "corp.example");
    assert!(list.get_string("2").is_err());
    assert_eq!(sb.key(CHROME).unwrap().get_string("DnsOverHttpsMode").unwrap(), "automatic");
    assert!(sb.key(r"SOFTWARE\Policies\Microsoft\Edge").is_none(), "keys we created are removed");
    assert!(sb.key(r"SOFTWARE\Policies\Mozilla").is_none());
}

#[test]
fn launch_blocks_are_set_and_removed_without_touching_other_settings() {
    let sb = Sandbox::new("ifeo");
    // Windows keeps its own settings in some IFEO keys.
    sb.create(&format!(r"{IFEO}\steam.exe")).set_u32("MitigationOptions", 7).unwrap();
    // Someone else's debugger is left alone.
    sb.create(&format!(r"{IFEO}\code.exe")).set_string("Debugger", "vsjitdebugger.exe").unwrap();

    let helper = Path::new(r"C:\Focus\focus-service.exe");
    let apps: Vec<AppName> = ["Steam", "discord", "code", "explorer"].iter().map(|a| AppName::parse(a).unwrap()).collect();
    let mut blocker = LaunchBlocker::new(sb.registry(), helper);
    blocker.enforce(&apps).unwrap();

    let debugger = r#""C:\Focus\focus-service.exe" --blocked"#;
    for exe in ["steam.exe", "discord.exe"] {
        assert_eq!(sb.key(&format!(r"{IFEO}\{exe}")).unwrap().get_string("Debugger").unwrap(), debugger, "{exe}");
    }
    assert_eq!(sb.key(&format!(r"{IFEO}\code.exe")).unwrap().get_string("Debugger").unwrap(), "vsjitdebugger.exe");
    assert!(sb.key(&format!(r"{IFEO}\explorer.exe")).is_none(), "protected apps are never blocked");

    // A new blocker (a restarted service) finds and lifts the old blocks.
    let mut restarted = LaunchBlocker::new(sb.registry(), helper);
    restarted.release_all().unwrap();
    let steam = sb.key(&format!(r"{IFEO}\steam.exe")).unwrap();
    assert!(steam.get_string("Debugger").is_err());
    assert_eq!(steam.get_u32("MitigationOptions").unwrap(), 7);
    assert!(sb.key(&format!(r"{IFEO}\discord.exe")).is_none(), "keys we created are removed");
    assert_eq!(sb.key(&format!(r"{IFEO}\code.exe")).unwrap().get_string("Debugger").unwrap(), "vsjitdebugger.exe");
}
