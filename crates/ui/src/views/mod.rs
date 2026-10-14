pub mod blocklist;
pub mod settings;
pub mod stats;
pub mod timer;

use eframe::egui::{self, Response, RichText};

use crate::theme;

/// A page title with a one-line description under it.
pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(24.0).strong().color(theme::TEXT));
    ui.label(RichText::new(subtitle).color(theme::SUBTLE));
    ui.add_space(14.0);
}

/// A card heading.
pub fn card_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(15.0).strong().color(theme::TEXT));
}

/// A button that is disabled while strict mode locks it, with an explanation on hover.
pub fn lockable_button(ui: &mut egui::Ui, text: &str, locked: bool) -> Response {
    let response = ui.add_enabled(!locked, egui::Button::new(text));
    response.on_disabled_hover_text("Locked by strict mode until this focus session ends")
}
