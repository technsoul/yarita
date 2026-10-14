//! `focus-service --blocked <app path> …`: what Windows runs instead of a
//! blocked app (see the launch blocker). Tells the user and exits.

use std::io::{BufReader, BufWriter};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

use focus_protocol::{Request, Response, SERVICE_ADDR, read_message, write_message};

pub fn run(mut args: impl Iterator<Item = String>) {
    let app = args
        .next()
        .as_deref()
        .and_then(|p| Path::new(p.trim_matches('"')).file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "This app".into());
    let detail = match remaining_minutes() {
        Some(m) => format!("{m} min left in this focus session."),
        None => "Open Focus Timer to see your session.".into(),
    };
    show(&format!("{app} is blocked while you focus.\n\n{detail}"));
}

fn remaining_minutes() -> Option<i64> {
    let stream = TcpStream::connect_timeout(&SERVICE_ADDR.parse().ok()?, Duration::from_millis(500)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    let mut writer = BufWriter::new(stream.try_clone().ok()?);
    write_message(&mut writer, &Request::Snapshot).ok()?;
    match read_message::<Response>(&mut BufReader::new(stream)).ok()?? {
        Response::Ok(s) | Response::Refused { snapshot: s, .. } => Some((s.remaining_secs + 59) / 60),
    }
}

#[cfg(windows)]
fn show(text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
    };
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (text, title) = (wide(text), wide("Focus Timer"));
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND,
        );
    }
}

#[cfg(not(windows))]
fn show(text: &str) {
    eprintln!("{text}");
}
