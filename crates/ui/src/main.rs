#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod client;
mod launcher;
mod theme;
mod views;
mod widgets;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Focus Timer")
            .with_inner_size(app::FULL_SIZE)
            .with_min_inner_size(app::FULL_MIN_SIZE),
        ..Default::default()
    };
    eframe::run_native("Focus Timer", options, Box::new(|cc| Ok(Box::new(app::FocusUi::new(cc)))))
}
