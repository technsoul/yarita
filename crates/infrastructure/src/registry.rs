//! Thin layer over the Windows registry used by the policy and launch blockers.
//!
//! Paths are written as if under `HKEY_LOCAL_MACHINE`. In development a
//! sandbox moves everything under `HKEY_CURRENT_USER\<sandbox>\`, where
//! nothing reads it, so the real system is left alone.

use focus_application::ports::PortError;
use windows_registry::{CURRENT_USER, Key, LOCAL_MACHINE};

/// `ERROR_FILE_NOT_FOUND` as an HRESULT.
const NOT_FOUND: i32 = 0x8007_0002_u32 as i32;
const KEY_WOW64_32KEY: u32 = 0x0200;

/// Which registry view to use. 32-bit programs read their own copy of some
/// keys, Image File Execution Options among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Native,
    Wow32,
}

#[derive(Debug, Clone)]
pub struct Registry {
    sandbox: Option<String>,
}

impl Registry {
    pub fn new(sandbox: Option<String>) -> Self {
        Self { sandbox }
    }

    pub fn is_sandboxed(&self) -> bool {
        self.sandbox.is_some()
    }

    fn locate(&self, path: &str) -> (&'static Key, String) {
        match &self.sandbox {
            Some(prefix) => (CURRENT_USER, format!(r"{prefix}\{path}")),
            None => (LOCAL_MACHINE, path.to_string()),
        }
    }

    fn access(&self, view: View) -> u32 {
        // The sandbox lives under HKCU\Software, which has a single view.
        if view == View::Wow32 && self.sandbox.is_none() { KEY_WOW64_32KEY } else { 0 }
    }

    /// Opens a key for reading and writing, creating it if needed.
    pub fn create(&self, path: &str, view: View) -> Result<Key, PortError> {
        let (hive, path) = self.locate(path);
        hive.options().read().write().create().access(self.access(view)).open(&path).map_err(|e| err(&path, e))
    }

    /// Opens an existing key for reading and writing. `None` if it doesn't exist.
    pub fn open(&self, path: &str, view: View) -> Result<Option<Key>, PortError> {
        let (hive, path) = self.locate(path);
        missing_ok(hive.options().read().write().access(self.access(view)).open(&path)).map_err(|e| err(&path, e))
    }

    /// Deletes a key and everything under it. A missing key is fine.
    pub fn remove_tree(&self, path: &str, view: View) -> Result<(), PortError> {
        let Some((parent, name)) = path.rsplit_once('\\') else {
            return Err(PortError(format!("Refusing to delete top-level registry key {path}")));
        };
        match self.open(parent, view)? {
            Some(key) => missing_ok(key.remove_tree(name)).map(drop).map_err(|e| err(path, e)),
            None => Ok(()),
        }
    }
}

/// Reads a string value. `None` if it doesn't exist or isn't a string.
pub fn get_string(key: &Key, name: &str) -> Option<String> {
    key.get_string(name).ok()
}

/// Deletes a value. A missing value is fine.
pub fn remove_value(key: &Key, name: &str) -> Result<(), PortError> {
    missing_ok(key.remove_value(name)).map(drop).map_err(|e| err(name, e))
}

pub fn is_empty(key: &Key) -> bool {
    let no_values = key.values().map(|mut v| v.next().is_none()).unwrap_or(false);
    let no_keys = key.keys().map(|mut k| k.next().is_none()).unwrap_or(false);
    no_values && no_keys
}

pub fn err(what: &str, e: windows_result::Error) -> PortError {
    PortError(format!(
        "Cannot change registry {what} ({}). Blocking needs the service to run as administrator.",
        e.message()
    ))
}

fn missing_ok<T>(r: windows_result::Result<T>) -> windows_result::Result<Option<T>> {
    match r {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.code().0 == NOT_FOUND => Ok(None),
        Err(e) => Err(e),
    }
}
