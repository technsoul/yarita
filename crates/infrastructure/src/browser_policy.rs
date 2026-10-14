//! Website blocking through browser enterprise policies (Windows).
//!
//! Complements the hosts file:
//! - `URLBlocklist` (Chrome, Edge, Brave) and `WebsiteFilter` (Firefox) cover
//!   every subdomain and show the browser's own "blocked" page.
//! - `DnsOverHttpsMode = off` stops Chromium browsers from skipping the hosts
//!   file through secure DNS.
//!
//! Users may have their own policies, so the blocker only removes entries it
//! added. What it added, and any value it replaced, is remembered under
//! `HKLM\SOFTWARE\Yarita\BrowserPolicy`, which survives crashes and reboots.
//!
//! Browsers that don't read these policies would get around them, so while
//! sites are blocked they are blocked from running too ([`UNSUPPORTED_BROWSERS`]).
//!
//! Chromium browsers pick up changes within seconds. Firefox reads policies
//! at startup, so it needs a restart; the hosts file covers it until then.

use yarita_domain::blocklist::{AppName, Site};

/// Exe names of browsers that ignore the policies below. Browsers that share
/// an exe name with a supported one (Tor Browser and Cent use `firefox.exe`
/// and `chrome.exe`) can't be told apart here; the hosts file still covers them.
pub const UNSUPPORTED_BROWSERS: &[&str] = &[
    // Chromium-based
    "opera", "vivaldi", "arc", "thorium", "iridium", "epic", "slimjet", "dragon", "maxthon",
    "ucbrowser", "avastbrowser", "avgbrowser", "ccleanerbrowser", "sidekick", "duckduckgo",
    "yandex", "coccoc", "browser360", "360chrome", "qqbrowser", "sogouexplorer",
    // Firefox-based
    "waterfox", "librewolf", "floorp", "zen", "mullvadbrowser", "palemoon", "basilisk", "seamonkey",
    // Other engines
    "iexplore", "falkon", "qutebrowser", "midori", "ladybird", "min",
];

pub fn unsupported_browsers() -> Vec<AppName> {
    UNSUPPORTED_BROWSERS.iter().filter_map(|name| AppName::parse(name).ok()).collect()
}

/// Returns `existing` without the entries we added last time (`owned`), plus
/// `ours`. Entries the user added stay in their original order, even when one
/// matches ours, so lifting our block never deletes theirs.
pub fn merge_list(existing: &[String], owned: &[String], ours: &[String]) -> Vec<String> {
    let mut leftover: Vec<&String> = owned.iter().collect();
    let mut out: Vec<String> = Vec::new();
    for entry in existing {
        if let Some(i) = leftover.iter().position(|o| *o == entry) {
            leftover.swap_remove(i);
        } else {
            out.push(entry.clone());
        }
    }
    out.extend(ours.iter().cloned());
    out
}

/// Chromium's filter format: a bare host blocks it and all its subdomains.
pub fn chromium_pattern(site: &Site) -> String {
    site.as_str().to_string()
}

/// Firefox match pattern: `*.` also matches the bare host.
pub fn firefox_pattern(site: &Site) -> String {
    format!("*://*.{}/*", site.as_str())
}

#[cfg(windows)]
pub use imp::BrowserPolicyBlocker;

#[cfg(windows)]
mod imp {
    use yarita_application::ports::{PortError, SiteBlocker};
    use yarita_domain::blocklist::{AppName, Site};
    use windows_registry::Key;

    use super::{chromium_pattern, firefox_pattern, merge_list};
    use crate::registry::{self, Registry, View};

    const STATE_KEY: &str = r"SOFTWARE\Yarita\BrowserPolicy";

    struct Browser {
        /// Prefix for the values we keep under [`STATE_KEY`].
        id: &'static str,
        policy_key: &'static str,
        /// Subkey of `policy_key` holding the list, as values "1", "2", …
        list_key: &'static str,
        pattern: fn(&Site) -> String,
        /// Has a `DnsOverHttpsMode` policy.
        chromium: bool,
    }

    const BROWSERS: &[Browser] = &[
        Browser {
            id: "Chrome",
            policy_key: r"SOFTWARE\Policies\Google\Chrome",
            list_key: "URLBlocklist",
            pattern: chromium_pattern,
            chromium: true,
        },
        Browser {
            id: "Edge",
            policy_key: r"SOFTWARE\Policies\Microsoft\Edge",
            list_key: "URLBlocklist",
            pattern: chromium_pattern,
            chromium: true,
        },
        Browser {
            id: "Chromium",
            policy_key: r"SOFTWARE\Policies\Chromium",
            list_key: "URLBlocklist",
            pattern: chromium_pattern,
            chromium: true,
        },
        Browser {
            id: "Brave",
            policy_key: r"SOFTWARE\Policies\BraveSoftware\Brave",
            list_key: "URLBlocklist",
            pattern: chromium_pattern,
            chromium: true,
        },
        Browser {
            id: "Firefox",
            policy_key: r"SOFTWARE\Policies\Mozilla\Firefox",
            list_key: r"WebsiteFilter\Block",
            pattern: firefox_pattern,
            chromium: false,
        },
    ];

    const DOH_POLICY: &str = "DnsOverHttpsMode";

    pub struct BrowserPolicyBlocker {
        registry: Registry,
    }

    impl BrowserPolicyBlocker {
        pub fn new(registry: Registry) -> Self {
            Self { registry }
        }

        fn sync(&mut self, sites: &[Site]) -> Result<(), PortError> {
            let state = self.registry.create(STATE_KEY, View::Native)?;
            let mut first_error = None;
            for browser in BROWSERS {
                let ours: Vec<String> = sites.iter().map(browser.pattern).collect();
                let result = self.sync_list(&state, browser, &ours).and_then(|()| {
                    if browser.chromium { self.sync_doh(&state, browser, !ours.is_empty()) } else { Ok(()) }
                });
                if let Err(e) = result {
                    first_error.get_or_insert(e);
                }
            }
            first_error.map_or(Ok(()), Err)
        }

        fn sync_list(&self, state: &Key, browser: &Browser, ours: &[String]) -> Result<(), PortError> {
            let owned_name = format!("{}.Owned", browser.id);
            let owned = state.get_multi_string(&owned_name).unwrap_or_default();
            let list_path = format!(r"{}\{}", browser.policy_key, browser.list_key);
            let list = self.registry.open(&list_path, View::Native)?;
            let existing = list.as_ref().map(read_list).unwrap_or_default();

            let updated = merge_list(&existing, &owned, ours);
            if updated != existing {
                if updated.is_empty() {
                    self.registry.remove_tree(&list_path, View::Native)?;
                    self.remove_empty_keys(&list_path)?;
                } else {
                    let list = match list {
                        Some(key) => key,
                        None => self.registry.create(&list_path, View::Native)?,
                    };
                    write_list(&list, existing.len(), &updated).map_err(|e| registry::err(&list_path, e))?;
                }
            }

            if ours.is_empty() {
                registry::remove_value(state, &owned_name)
            } else if owned != ours {
                state.set_multi_string(owned_name.clone(), ours).map_err(|e| registry::err(&owned_name, e))
            } else {
                Ok(())
            }
        }

        /// While blocking, forces secure DNS off. `<id>.DohSaved` marks that
        /// we did, and `<id>.DohOriginal` holds the value we replaced, if any.
        fn sync_doh(&self, state: &Key, browser: &Browser, blocking: bool) -> Result<(), PortError> {
            let saved_name = format!("{}.DohSaved", browser.id);
            let original_name = format!("{}.DohOriginal", browser.id);
            let saved = state.get_u32(&saved_name).is_ok();
            if blocking {
                let policy = self.registry.create(browser.policy_key, View::Native)?;
                let current = registry::get_string(&policy, DOH_POLICY);
                if !saved {
                    if let Some(value) = &current {
                        state.set_string(&original_name, value).map_err(|e| registry::err(&original_name, e))?;
                    }
                    state.set_u32(&saved_name, 1).map_err(|e| registry::err(&saved_name, e))?;
                }
                if current.as_deref() != Some("off") {
                    policy.set_string(DOH_POLICY, "off").map_err(|e| registry::err(DOH_POLICY, e))?;
                }
            } else if saved {
                if let Some(policy) = self.registry.open(browser.policy_key, View::Native)? {
                    match registry::get_string(state, &original_name) {
                        Some(value) => policy.set_string(DOH_POLICY, value).map_err(|e| registry::err(DOH_POLICY, e))?,
                        None => registry::remove_value(&policy, DOH_POLICY)?,
                    }
                }
                registry::remove_value(state, &original_name)?;
                registry::remove_value(state, &saved_name)?;
                self.remove_empty_keys(browser.policy_key)?;
            }
            Ok(())
        }

        /// Removes `path` and its parents while they are empty, up to
        /// `SOFTWARE\Policies`, so the browser stops saying it's managed.
        fn remove_empty_keys(&self, path: &str) -> Result<(), PortError> {
            let mut path = path;
            while !path.eq_ignore_ascii_case(r"SOFTWARE\Policies") {
                match self.registry.open(path, View::Native)? {
                    Some(key) if !registry::is_empty(&key) => break,
                    Some(key) => {
                        drop(key);
                        self.registry.remove_tree(path, View::Native)?;
                    }
                    None => {}
                }
                let Some((parent, _)) = path.rsplit_once('\\') else { break };
                path = parent;
            }
            Ok(())
        }
    }

    impl SiteBlocker for BrowserPolicyBlocker {
        fn block(&mut self, sites: &[Site]) -> Result<(), PortError> {
            self.sync(sites)
        }

        fn unblock_all(&mut self) -> Result<(), PortError> {
            self.sync(&[])
        }

        fn bypassing_apps(&self) -> Vec<AppName> {
            super::unsupported_browsers()
        }
    }

    /// Browsers read list policies as values named "1", "2", … with no gaps.
    fn read_list(key: &Key) -> Vec<String> {
        (1..).map_while(|i| registry::get_string(key, &i.to_string())).collect()
    }

    fn write_list(key: &Key, old_len: usize, entries: &[String]) -> windows_result::Result<()> {
        for (i, entry) in entries.iter().enumerate() {
            key.set_string((i + 1).to_string(), entry)?;
        }
        for i in entries.len() + 1..=old_len {
            let _ = key.remove_value(i.to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn adds_ours_after_user_entries() {
        assert_eq!(merge_list(&v(&["corp.com"]), &[], &v(&["youtube.com"])), v(&["corp.com", "youtube.com"]));
    }

    #[test]
    fn replaces_only_what_we_added() {
        let existing = v(&["corp.com", "youtube.com", "reddit.com"]);
        let owned = v(&["youtube.com", "reddit.com"]);
        assert_eq!(merge_list(&existing, &owned, &v(&["x.com"])), v(&["corp.com", "x.com"]));
        assert_eq!(merge_list(&existing, &owned, &[]), v(&["corp.com"]));
    }

    #[test]
    fn keeps_a_user_entry_that_matches_ours() {
        // The user blocked youtube.com themselves before we added it too.
        let existing = v(&["youtube.com", "youtube.com"]);
        assert_eq!(merge_list(&existing, &v(&["youtube.com"]), &[]), v(&["youtube.com"]));
    }

    #[test]
    fn is_idempotent() {
        let ours = v(&["a.com", "b.com"]);
        let once = merge_list(&v(&["corp.com"]), &[], &ours);
        assert_eq!(merge_list(&once, &ours, &ours), once);
    }

    #[test]
    fn unsupported_browsers_parse_and_exclude_supported_ones() {
        let apps = unsupported_browsers();
        assert_eq!(apps.len(), UNSUPPORTED_BROWSERS.len(), "every name is a valid app name");
        for supported in ["chrome", "msedge", "brave", "firefox"] {
            assert!(!apps.iter().any(|a| a.as_str() == supported), "{supported}");
        }
    }

    #[test]
    fn patterns() {
        let site = Site::parse("youtube.com").unwrap();
        assert_eq!(chromium_pattern(&site), "youtube.com");
        assert_eq!(firefox_pattern(&site), "*://*.youtube.com/*");
    }
}
