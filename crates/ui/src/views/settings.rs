use eframe::egui::{self, RichText};
use yarita_protocol::{Request, Snapshot};

use crate::app::FocusUi;
use crate::theme;

pub fn show(app: &mut FocusUi, ui: &mut egui::Ui, snap: &Snapshot) {
    super::page_header(ui, "Settings", "Session lengths and how blocking works.");
    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        super::card_title(ui, "Timer");
        if snap.locked {
            ui.label(RichText::new("Settings are locked during a strict focus session.").color(theme::WARN));
        }
        ui.add_space(6.0);

        let mut draft = app.config_draft.unwrap_or(snap.config);
        ui.add_enabled_ui(!snap.locked, |ui| {
            egui::Grid::new("config").num_columns(2).spacing([24.0, 12.0]).show(ui, |ui| {
                row(ui, "Focus (minutes)", &mut draft.focus_minutes, 1..=240);
                row(ui, "Short break (minutes)", &mut draft.short_break_minutes, 1..=60);
                row(ui, "Long break (minutes)", &mut draft.long_break_minutes, 1..=120);
                row(ui, "Sessions before long break", &mut draft.sessions_before_long_break, 1..=12);
            });
        });
        app.config_draft = (draft != snap.config).then_some(draft);

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let dirty = app.config_draft.is_some();
            if ui.add_enabled(dirty && !snap.locked, egui::Button::new("Save")).clicked() {
                app.send(Request::UpdateConfig(draft));
                app.config_draft = None;
            }
            if ui.add_enabled(dirty, egui::Button::new("Revert")).clicked() {
                app.config_draft = None;
            }
        });
        ui.label(RichText::new("Changes apply from the next session.").small().weak());
    });

    ui.add_space(16.0);
    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        super::card_title(ui, "About blocking");
        ui.label(
            RichText::new(
                "Websites are blocked through the hosts file and browser policies (Chrome, Edge, Brave, \
             Chromium and Firefox). Other browsers can't open while websites are blocked. Blocked apps can't open during a focus session, and running ones are closed. \
             A background service does this, so it keeps working when this window is closed. \
             Firefox needs a restart to pick up the browser policy.",
            )
            .color(theme::MUTED),
        );
    });
}

fn row(ui: &mut egui::Ui, label: &str, value: &mut u32, range: std::ops::RangeInclusive<u32>) {
    ui.label(RichText::new(label).color(theme::TEXT_SOFT));
    ui.add(egui::DragValue::new(value).range(range).speed(0.2));
    ui.end_row();
}
