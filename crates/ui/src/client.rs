//! Connection to the background service, run on its own thread.

use std::io::{BufReader, BufWriter};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use focus_protocol::{Request, Response, SERVICE_ADDR, Snapshot, read_message, write_message};

const POLL_EVERY: Duration = Duration::from_millis(500);

#[derive(Default)]
pub struct ClientState {
    pub snapshot: Option<Snapshot>,
    pub connected: bool,
    /// Set once the first connection attempt has finished.
    pub attempted: bool,
    pub refusal: Option<(String, Instant)>,
}

pub struct ServiceClient {
    tx: Sender<Request>,
    state: Arc<Mutex<ClientState>>,
}

impl ServiceClient {
    pub fn spawn(ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(ClientState::default()));
        let worker_state = Arc::clone(&state);
        thread::spawn(move || run(rx, &worker_state, &ctx));
        Self { tx, state }
    }

    pub fn send(&self, request: Request) {
        let _ = self.tx.send(request);
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut ClientState) -> R) -> R {
        f(&mut self.state.lock().unwrap_or_else(|p| p.into_inner()))
    }
}

fn run(rx: Receiver<Request>, state: &Mutex<ClientState>, ctx: &egui::Context) {
    let addr: SocketAddr = SERVICE_ADDR.parse().expect("valid address");
    loop {
        let session = TcpStream::connect_timeout(&addr, Duration::from_millis(300))
            .and_then(|stream| serve(stream, &rx, state, ctx));
        match session {
            Ok(Exit::UiClosed) => return,
            Ok(Exit::Disconnected) | Err(_) => {
                let mut s = state.lock().unwrap_or_else(|p| p.into_inner());
                s.connected = false;
                s.attempted = true;
                drop(s);
                ctx.request_repaint();
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

enum Exit {
    UiClosed,
    Disconnected,
}

fn serve(
    stream: TcpStream,
    rx: &Receiver<Request>,
    state: &Mutex<ClientState>,
    ctx: &egui::Context,
) -> std::io::Result<Exit> {
    stream.set_nodelay(true)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);
    // Show state right away instead of waiting for the first poll.
    let mut pending = Some(Request::Snapshot);
    loop {
        let request = match pending.take().map_or_else(|| rx.recv_timeout(POLL_EVERY), Ok) {
            Ok(r) => r,
            Err(RecvTimeoutError::Timeout) => Request::Snapshot,
            Err(RecvTimeoutError::Disconnected) => return Ok(Exit::UiClosed),
        };
        write_message(&mut writer, &request)?;
        let Some(response) = read_message::<Response>(&mut reader)? else {
            return Ok(Exit::Disconnected);
        };
        let mut s = state.lock().unwrap_or_else(|p| p.into_inner());
        s.connected = true;
        s.attempted = true;
        match response {
            Response::Ok(snapshot) => s.snapshot = Some(snapshot),
            Response::Refused { reason, snapshot } => {
                s.snapshot = Some(snapshot);
                s.refusal = Some((reason, Instant::now()));
            }
        }
        drop(s);
        ctx.request_repaint();
    }
}
