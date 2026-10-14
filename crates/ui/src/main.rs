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
            .with_title("Yarita")
            .with_inner_size(app::FULL_SIZE)
            .with_min_inner_size(app::FULL_MIN_SIZE)
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png"))
                    .expect("bundled icon is a valid PNG"),
            ),
        ..Default::default()
    };
    eframe::run_native("Yarita", options, Box::new(|cc| Ok(Box::new(app::FocusUi::new(cc)))))
}
