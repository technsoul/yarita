# Yarita

A minimal, cross-platform Pomodoro timer that blocks websites and apps while you focus.
Built in Rust with [egui](https://github.com/emilk/egui), structured with Domain-Driven Design.

- **Pomodoro cycle:** focus, short break and long break, all configurable
- **Website blocking:** system-wide, through the hosts file, so it works in every browser.
  On Windows, browser policies are added too (Chrome, Edge, Brave, Chromium, Firefox): they block every
  subdomain and turn off secure DNS (DNS over HTTPS) so browsers can't skip the hosts file.
  Browsers that ignore these policies (Opera, Vivaldi, …) can't open while sites are blocked.
- **App blocking:** on Windows, blocked apps can't start at all (Image File Execution Options).
  Apps that are already running, or have been renamed, are closed.
- **Strict mode:** no pausing, stopping, unblocking or changing settings until the session ends.
  The window can't be closed either, and the service reopens it if it's killed.
  It survives killing the app and rebooting.
- **Stats:** today, streak, total, and the last 7 days

## Architecture

```
crates/
├── domain/          Pure business rules. No I/O.
│   ├── session/     PomodoroTimer aggregate: phases, cycle, strict-mode invariants
│   ├── blocklist/   Blocklist aggregate, Site and AppName value objects
│   ├── stats/       FocusRecord, StatsSummary
│   ├── events.rs    Domain events
│   └── repositories.rs  Persistence ports
├── application/     Use cases (FocusApp) and outbound ports: Clock, SiteBlocker, AppBlocker
├── infrastructure/  Adapters: hosts file, browser policies, launch blocker (IFEO), process
│                    blocker, JSON store, system clock
├── protocol/        Wire DTOs shared by the service and the UI (JSON lines over localhost TCP)
├── service/         Background process with admin rights. Owns the session and enforces blocking.
└── ui/              egui client. Displays state and sends commands. Holds no rules.
```

Dependencies only point inward: `ui → protocol ← application → domain ← infrastructure`.

**Why a separate service?** Editing the hosts file needs admin rights, and strict mode must hold
even if the window is closed or killed. The service checks every rule, so a modified client still
can't get around strict mode.

## Run in development (no admin needed)

Development uses a fake hosts file, a local state folder and a registry sandbox
(`HKCU\Software\YaritaDev`, which browsers and Windows ignore), so your real system isn't touched:

```powershell
$env:YARITA_HOSTS_FILE = "$PWD\.dev-data\hosts"
$env:YARITA_DATA_DIR   = "$PWD\.dev-data\state"
$env:YARITA_REGISTRY_SANDBOX = "Software\YaritaDev"
New-Item -ItemType Directory -Force .dev-data | Out-Null
if (-not (Test-Path .dev-data\hosts)) { Set-Content .dev-data\hosts "127.0.0.1 localhost" }
cargo build
cargo run -p yarita
```

The UI starts the service automatically. When `YARITA_HOSTS_FILE` is set, it starts the service
without asking for elevation.

## Run for real

```bash
cargo build --release
```

Run `target/release/yarita` and keep `yarita-service` in the same folder.
On first launch you'll get one admin prompt (UAC on Windows, a password prompt on macOS, pkexec on Linux).

| | Hosts file | State |
|---|---|---|
| Windows | `C:\Windows\System32\drivers\etc\hosts` | `C:\ProgramData\yarita` |
| macOS | `/etc/hosts` | `/Library/Application Support/yarita` |
| Linux | `/etc/hosts` | `/var/lib/yarita` |

## Tests

```bash
cargo test --workspace
```

## How Windows blocking works

| What | Where | Notes |
|---|---|---|
| Websites | `hosts` file | Every browser and app |
| Websites | `HKLM\SOFTWARE\Policies\…` `URLBlocklist` (Chromium), `WebsiteFilter` (Firefox) | Covers subdomains. Firefox only picks it up after a restart. While blocking, the browser says it is "managed by your organization". |
| Other browsers | Blocked like apps while any site is blocked | Opera, Vivaldi, Arc, Waterfox, LibreWolf and others that ignore these policies |
| Secure DNS | `DnsOverHttpsMode = off` (Chromium) | Restored to the previous value afterwards |
| Apps | `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\<app>.exe` → `Debugger` | Windows runs `yarita-service --blocked` instead of the app, which shows a message |
| Apps | process list | Closes running and renamed apps every second |

Only entries the app added are removed. Your own policies, and Windows' own IFEO settings, stay put.
What was added is remembered in `HKLM\SOFTWARE\Yarita` and in marker values, so a crash or
reboot is cleaned up the next time the service starts. Windows components, shells, `regedit` and
Yarita itself are never blocked at launch.

## Known limits

- An admin user can always get around blocking, for example by killing the service and editing the
  hosts file. Strict mode makes quitting a session a real hassle; it can't make it impossible.
- Browser tabs that are already open may keep working until reloaded, because of cached connections.
- Launch blocking matches the exe name, so a renamed exe can still start. It's closed within a second.
- Some antivirus tools warn about Image File Execution Options changes, since malware uses them too.
- During strict mode the service may reopen the UI, and that copy runs as administrator.
- The service doesn't start automatically at boot yet. After a reboot, an active session's
  hosts entries stay in place, and the session resumes when you open the app.

## Releases

Pushing a `v*` tag runs [`.github/workflows/release.yml`](.github/workflows/release.yml), which builds
a Windows installer (`.exe`) and portable `.zip`, a Linux `.deb` and a universal macOS `.dmg`,
and attaches them to a GitHub Release:

```bash
git tag v0.1.0
git push origin v0.1.0
```
