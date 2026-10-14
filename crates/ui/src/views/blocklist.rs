use eframe::egui::{self, RichText};
use yarita_protocol::{Request, Snapshot};

use super::lockable_button;
use crate::app::FocusUi;
use crate::theme;

const SITE_SUGGESTIONS: &[&str] =
    &["youtube.com", "reddit.com", "x.com", "instagram.com", "tiktok.com", "facebook.com"];
const APP_SUGGESTIONS: &[&str] = &["discord", "steam", "spotify", "telegram"];

pub fn show(app: &mut FocusUi, ui: &mut egui::Ui, snap: &Snapshot) {
    super::page_header(ui, "Block list", "Sites and apps that are blocked while you focus.");
    if snap.locked {
        ui.label(RichText::new("Strict mode: you can add, but not remove, until the session ends.").color(theme::WARN));
        ui.add_space(6.0);
    }

    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        let edit =
            section(ui, "Websites", "youtube.com", &mut app.site_input, SITE_SUGGESTIONS, &snap.sites, snap.locked);
        edit.send(app, Request::AddSite, Request::RemoveSite);
        if cfg!(windows) {
            ui.label(
                RichText::new(
                    "Works in Chrome, Edge, Brave, Chromium and Firefox. Other browsers, like Opera and \
                     Vivaldi, can't open while websites are blocked.",
                )
                .small()
                .weak(),
            );
        }
    });

    ui.add_space(16.0);
    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        let edit = section(ui, "Apps", "discord", &mut app.app_input, APP_SUGGESTIONS, &snap.apps, snap.locked);
        edit.send(app, Request::AddApp, Request::RemoveApp);
        ui.label(
            RichText::new("Use the program's name, like steam or chrome. Blocked apps can't open during focus, and running ones are closed.")
                .small()
                .weak(),
        );
    });
}

#[derive(Default)]
struct Edit {
    add: Option<String>,
    remove: Option<String>,
}

impl Edit {
    fn send(self, app: &FocusUi, add: fn(String) -> Request, remove: fn(String) -> Request) {
        if let Some(entry) = self.add {
            app.send(add(entry));
        }
        if let Some(entry) = self.remove {
            app.send(remove(entry));
        }
    }
}

fn section(
    ui: &mut egui::Ui,
    title: &str,
    hint: &str,
    input: &mut String,
    suggestions: &[&str],
    entries: &[String],
    locked: bool,
) -> Edit {
    let mut edit = Edit::default();
    super::card_title(ui, title);

    ui.horizontal(|ui| {
        let field =
            ui.add(egui::TextEdit::singleline(input).hint_text(hint).desired_width(ui.available_width() - 70.0));
        let submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (ui.button("Add").clicked() || submitted) && !input.trim().is_empty() {
            edit.add = Some(std::mem::take(input).trim().to_string());
            if submitted {
                field.request_focus();
            }
        }
    });

    let missing: Vec<&str> = suggestions.iter().copied().filter(|s| !entries.iter().any(|e| e == s)).collect();
    if !missing.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for s in missing {
                if ui.small_button(format!("+ {s}")).clicked() {
                    edit.add = Some(s.to_string());
                }
            }
        });
    }

    if entries.is_empty() {
        ui.label(RichText::new("Nothing here yet").weak());
    }
    for entry in entries {
        ui.horizontal(|ui| {
            ui.label(RichText::new(entry).color(theme::TEXT_SOFT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if lockable_button(ui, "Remove", locked).clicked() {
                    edit.remove = Some(entry.clone());
                }
            });
        });
    }
    edit
}
