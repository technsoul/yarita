//! Background service: owns the session and enforces blocking.
//!
//! Runs with admin rights (needed for the hosts file and registry). The UI is
//! just a client, so closing or killing the UI never ends a strict session,
//! and during one the service reopens the UI if it disappears.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod blocked;
mod handler;

use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{process, thread};

use yarita_application::ports::AppBlocker;
use yarita_application::{Dependencies, FocusApp};
use yarita_infrastructure::{
    AppBlockers, BLOCKED_FLAG, HostsFileBlocker, JsonStore, ProcessBlocker, SiteBlockers, SystemClock, paths,
};
use yarita_protocol::{Request, SERVICE_ADDR, read_message, write_message};

type SharedApp = Arc<Mutex<FocusApp>>;

/// How long a strict session may go without a connected UI before it is reopened.
const UI_GRACE: Duration = Duration::from_secs(3);
/// Time a freshly started UI gets to connect.
const UI_STARTUP: Duration = Duration::from_secs(10);

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some(BLOCKED_FLAG) {
        blocked::run(args);
        return;
    }

    let listener = match TcpListener::bind(SERVICE_ADDR) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("yarita-service: {SERVICE_ADDR} is in use ({e}); another instance is probably running");
            process::exit(0);
        }
    };

    let data_dir = paths::data_dir();
    let store = JsonStore::open(&data_dir).unwrap_or_else(|e| fail(&e.to_string()));
    let (hosts_path, flush_dns) = paths::hosts_file();
    eprintln!("yarita-service: data in {}, hosts file {}", data_dir.display(), hosts_path.display());

    let app = FocusApp::load(Dependencies {
        timers: Box::new(store.clone()),
        blocklists: Box::new(store.clone()),
        records: Box::new(store),
        sites: Box::new(site_blockers(HostsFileBlocker::new(hosts_path, flush_dns))),
        apps: Box::new(app_blockers()),
        clock: Box::new(SystemClock),
    })
    .unwrap_or_else(|e| fail(&e.to_string()));
    let app: SharedApp = Arc::new(Mutex::new(app));

    let ticker = Arc::clone(&app);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        lock(&ticker).tick();
    });

    let clients = Arc::new(AtomicUsize::new(0));
    let watched = (Arc::clone(&app), Arc::clone(&clients));
    thread::spawn(move || keep_ui_open(&watched.0, &watched.1));

    eprintln!("yarita-service: listening on {SERVICE_ADDR}");
    for stream in listener.incoming().flatten() {
        let app = Arc::clone(&app);
        let clients = Arc::clone(&clients);
        thread::spawn(move || {
            clients.fetch_add(1, Ordering::SeqCst);
            serve(stream, &app);
            clients.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

#[cfg(windows)]
fn site_blockers(hosts: HostsFileBlocker) -> SiteBlockers {
    use yarita_infrastructure::{BrowserPolicyBlocker, Registry};
    let registry = Registry::new(paths::registry_sandbox());
    SiteBlockers(vec![Box::new(hosts), Box::new(BrowserPolicyBlocker::new(registry))])
}

#[cfg(not(windows))]
fn site_blockers(hosts: HostsFileBlocker) -> SiteBlockers {
    SiteBlockers(vec![Box::new(hosts) as Box<dyn yarita_application::ports::SiteBlocker>])
}

/// Blocking at launch stops most apps from opening at all; closing running
/// processes catches the rest (renamed exes, apps open before the session).
#[cfg(windows)]
fn app_blockers() -> AppBlockers {
    use yarita_infrastructure::{LaunchBlocker, Registry};
    let mut blockers: Vec<Box<dyn AppBlocker>> = Vec::new();
    match std::env::current_exe() {
        Ok(me) => {
            let registry = Registry::new(paths::registry_sandbox());
            blockers.push(Box::new(LaunchBlocker::new(registry, &me)));
        }
        Err(e) => eprintln!("yarita-service: launch blocking off, can't find own path ({e})"),
    }
    blockers.push(Box::new(ProcessBlocker::new()));
    AppBlockers(blockers)
}

#[cfg(not(windows))]
fn app_blockers() -> AppBlockers {
    AppBlockers(vec![Box::new(ProcessBlocker::new()) as Box<dyn AppBlocker>])
}

/// During strict focus, reopens the UI when no client has been connected for
/// a moment, e.g. after it was killed from Task Manager.
fn keep_ui_open(app: &SharedApp, clients: &AtomicUsize) {
    let Some(ui) = ui_path() else { return };
    let mut missing = Duration::ZERO;
    loop {
        thread::sleep(Duration::from_secs(1));
        let locked = lock(app).is_locked();
        if !locked || clients.load(Ordering::SeqCst) > 0 {
            missing = Duration::ZERO;
            continue;
        }
        missing += Duration::from_secs(1);
        if missing < UI_GRACE {
            continue;
        }
        eprintln!("yarita-service: strict session without a UI, reopening it");
        if let Err(e) = Command::new(&ui).spawn() {
            eprintln!("yarita-service: cannot reopen the UI: {e}");
        }
        missing = Duration::ZERO;
        thread::sleep(UI_STARTUP);
    }
}

fn ui_path() -> Option<PathBuf> {
    let name = if cfg!(windows) { "yarita.exe" } else { "yarita" };
    let path = std::env::current_exe().ok()?.with_file_name(name);
    path.exists().then_some(path)
}

fn serve(stream: TcpStream, app: &SharedApp) {
    let Ok(read_half) = stream.try_clone() else { return };
    let mut reader = BufReader::new(read_half);
    let mut writer = BufWriter::new(stream);
    while let Ok(Some(request)) = read_message::<Request>(&mut reader) {
        let response = handler::handle(&mut lock(app), request);
        if write_message(&mut writer, &response).is_err() {
            break;
        }
    }
}

/// A panic in one client thread must not take enforcement down with it.
fn lock(app: &SharedApp) -> std::sync::MutexGuard<'_, FocusApp> {
    app.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn fail(msg: &str) -> ! {
    eprintln!("yarita-service: {msg}");
    process::exit(1);
}
