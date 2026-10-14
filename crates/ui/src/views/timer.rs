use eframe::egui::{self, Align, Layout, RichText};
use yarita_protocol::{PhaseDto, Request, Snapshot, StatusDto};

use crate::app::{FocusUi, Tab};
use crate::theme;
use crate::widgets::{self, Icon};

const LOCKED_HINT: &str = "Locked by strict mode until this focus session ends";

pub fn show(app: &mut FocusUi, ui: &mut egui::Ui, snap: &Snapshot) {
    ui.spacing_mut().item_spacing.y = 26.0;
    ui.vertical_centered(|ui| {
        let selected = match snap.phase {
            PhaseDto::Focus => 0,
            PhaseDto::ShortBreak => 1,
            PhaseDto::LongBreak => 2,
        };
        widgets::phase_segments(ui, &["Focus", "Short break", "Long break"], selected);
    });

    widgets::dot_matrix(ui, snap.remaining_secs, 15.0, colon_on(snap));

    session_row(ui, snap, 14.0);
    controls(app, ui, snap, 64.0, 92.0);
    strict_card(app, ui, snap);
}

/// The compact, always-on-top widget.
pub fn mini(app: &mut FocusUi, ui: &mut egui::Ui, snap: &Snapshot) {
    ui.spacing_mut().item_spacing.y = 18.0;
    // The widget has no title bar, so its header row moves the window.
    let header = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        widgets::paint_icon(ui.painter(), Icon::Timer, rect, theme::TEXT);
        ui.label(RichText::new(theme::phase_label(snap.phase)).size(16.0).strong().color(theme::TEXT));
        ui.label(RichText::new(status_text(snap)).color(theme::MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // Strict mode keeps the app open, so there is nothing to close.
            if !snap.locked && widgets::icon_button(ui, Icon::Close, "Close").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if widgets::icon_button(ui, Icon::Expand, "Open full app").clicked() {
                app.set_mini(ui.ctx(), false);
            }
        });
    });
    if header.response.interact(egui::Sense::drag()).drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    widgets::dot_matrix(ui, snap.remaining_secs, 7.0, colon_on(snap));
    session_row(ui, snap, 13.0);
    controls(app, ui, snap, 56.0, 76.0);
    if snap.locked {
        ui.label(RichText::new("Strict mode · locked until this session ends").small().color(theme::WARN));
    }
}

/// The right rail: the cycle laid out step by step.
pub fn plan(ui: &mut egui::Ui, snap: &Snapshot) {
    let c = snap.config;
    let n = c.sessions_before_long_break.max(1);
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.label(RichText::new("Session plan").size(15.0).strong().color(theme::TEXT));
    ui.label(
        RichText::new(format!(
            "{n} × {} min focus, {} min breaks, {} min long break at the end",
            c.focus_minutes, c.short_break_minutes, c.long_break_minutes
        ))
        .color(theme::SUBTLE),
    );
    ui.add_space(16.0);

    // Steps alternate focus, break, focus, … ending with the long break.
    let done = snap.completed_in_cycle.min(n);
    let current = match snap.phase {
        PhaseDto::Focus => 2 * done.min(n - 1),
        PhaseDto::ShortBreak => 2 * done.max(1) - 1,
        PhaseDto::LongBreak => 2 * n - 1,
    };
    ui.spacing_mut().item_spacing.y = 6.0;
    for step in 0..2 * n {
        let i = step / 2 + 1;
        let (label, minutes) = match (step % 2, i == n) {
            (0, _) => (format!("Focus {i}"), c.focus_minutes),
            (_, false) => ("Short break".to_string(), c.short_break_minutes),
            (_, true) => ("Long break".to_string(), c.long_break_minutes),
        };
        let is_focus = step % 2 == 0;
        let now = step == current;
        let (dot, color) = match () {
            _ if now => (theme::phase_color(snap.phase), theme::TEXT),
            _ if step < current && is_focus => (theme::ACCENT_DIM, theme::TEXT),
            _ if step < current => (theme::FAINT, theme::SUBTLE),
            _ if is_focus => (theme::FAINT, theme::MUTED),
            _ => (theme::FAINT, theme::SUBTLE),
        };
        let fill = if now { theme::NOW_BG } else { egui::Color32::TRANSPARENT };
        egui::Frame::new().fill(fill).corner_radius(10).inner_margin(egui::Margin::symmetric(12, 9)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.0, dot);
                let text = if now { format!("{label} · now") } else { label };
                ui.label(RichText::new(text).size(13.0).color(color));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("{minutes} min")).monospace().size(12.0).color(theme::SUBTLE));
                });
            });
        });
    }
}

/// The blocking summary card at the bottom of the right rail.
pub fn blocking_card(app: &mut FocusUi, ui: &mut egui::Ui, snap: &Snapshot) {
    theme::card().inner_margin(16).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Blocking").strong().color(theme::TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (text, color) = if snap.blocking { ("On", theme::ACCENT) } else { ("Off", theme::SUBTLE) };
                ui.label(RichText::new(text).size(12.0).color(color));
            });
        });
        if let Some(err) = &snap.enforcement_error {
            ui.label(RichText::new(err).size(13.0).color(theme::WARN));
        } else if snap.sites.is_empty() && snap.apps.is_empty() {
            ui.label(RichText::new("Nothing to block yet.").size(13.0).color(theme::MUTED));
        } else {
            ui.label(
                RichText::new(format!(
                    "{} site{} · {} app{}",
                    snap.sites.len(),
                    plural(snap.sites.len()),
                    snap.apps.len(),
                    plural(snap.apps.len())
                ))
                .size(13.0)
                .color(theme::MUTED),
            );
        }
        if ui.link("Edit block list").clicked() {
            app.tab = Tab::Blocklist;
        }
    });
}

fn colon_on(snap: &Snapshot) -> bool {
    snap.status != StatusDto::Running || snap.remaining_secs % 2 == 1
}

fn status_text(snap: &Snapshot) -> &'static str {
    match snap.status {
        StatusDto::Idle => "Ready",
        StatusDto::Running => "",
        StatusDto::Paused => "Paused",
    }
}

fn session_row(ui: &mut egui::Ui, snap: &Snapshot, size: f32) {
    let n = snap.config.sessions_before_long_break.max(1);
    let done = snap.completed_in_cycle.min(n);
    let (session, current) = match snap.phase {
        PhaseDto::Focus => ((done + 1).min(n), true),
        _ => (done.max(1), false),
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        widgets::badge(ui, "LOOP", size - 3.0);
        ui.label(RichText::new(format!("{session}/{n} Session")).size(size).strong().color(theme::TEXT));
        widgets::pips(ui, n, done, current);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{} focus today", duration(snap.stats.today_minutes)))
                    .size(size)
                    .color(theme::MUTED),
            );
        });
    });
}

fn controls(app: &FocusUi, ui: &mut egui::Ui, snap: &Snapshot, small: f32, big: f32) {
    let locked = snap.locked;
    let active = snap.status != StatusDto::Idle;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        let stop = widgets::round_button(ui, small, Icon::Stop, "Stop and reset", active && !locked);
        if stop.clicked() {
            app.send(Request::Stop);
        }
        if locked && active {
            stop.on_disabled_hover_text(LOCKED_HINT);
        }
        let skip = widgets::round_button(ui, small, Icon::Skip, "Skip to next", !locked);
        if skip.clicked() {
            app.send(Request::Skip);
        }
        if locked {
            skip.on_disabled_hover_text(LOCKED_HINT);
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (label, request, enabled) = match snap.status {
                StatusDto::Idle => ("Start", Request::Start, true),
                StatusDto::Running => ("Pause", Request::Pause, !locked),
                StatusDto::Paused => ("Resume", Request::Resume, true),
            };
            let remaining = if snap.total_secs > 0 { snap.remaining_secs as f32 / snap.total_secs as f32 } else { 1.0 };
            let running = snap.status == StatusDto::Running;
            let color = theme::phase_color(snap.phase);
            let play = widgets::play_button(ui, big, running, remaining.clamp(0.0, 1.0), color, label, enabled);
            if play.clicked() {
                app.send(request);
            }
            if !enabled {
                play.on_disabled_hover_text(LOCKED_HINT);
            }
        });
    });
}

fn strict_card(app: &FocusUi, ui: &mut egui::Ui, snap: &Snapshot) {
    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
            let icon_color = if snap.strict { theme::ACCENT } else { theme::TEXT };
            widgets::paint_icon(ui.painter(), Icon::Lock, rect, icon_color);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new("Strict mode").size(15.0).color(theme::TEXT));
                let detail = if snap.locked {
                    RichText::new("Locked until this session ends").color(theme::WARN)
                } else {
                    RichText::new("No pausing, stopping or unblocking until focus ends").color(theme::SUBTLE)
                };
                ui.label(detail.size(13.0));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut strict = snap.strict;
                let response = widgets::toggle(ui, &mut strict, !snap.locked, "Strict mode");
                if response.changed() {
                    app.send(Request::SetStrict(strict));
                }
                if snap.locked {
                    response.on_hover_text("Can't turn strict mode off during a focus session");
                }
            });
        });
    });
}

pub fn duration(minutes: u32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}
