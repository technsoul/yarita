use chrono::{Days, Local};
use eframe::egui::{self, Align2, FontId, RichText, Sense, vec2};
use focus_protocol::Snapshot;

use crate::theme;

pub fn show(ui: &mut egui::Ui, snap: &Snapshot) {
    let s = &snap.stats;
    super::page_header(ui, "Statistics", "Your focus time, day by day.");
    ui.columns(3, |cols| {
        tile(&mut cols[0], "Today", &format!("{}", s.today_sessions), &format!("{} min", s.today_minutes));
        tile(&mut cols[1], "Streak", &format!("{}", s.streak_days), if s.streak_days == 1 { "day" } else { "days" });
        tile(&mut cols[2], "Total", &hours(s.total_minutes), &format!("{} sessions", s.total_sessions));
    });

    ui.add_space(16.0);
    theme::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        super::card_title(ui, "Last 7 days");
        ui.add_space(12.0);
        week_chart(ui, &s.last_7_days_minutes);
    });
}

fn tile(ui: &mut egui::Ui, title: &str, value: &str, detail: &str) {
    theme::card().inner_margin(16).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(RichText::new(title).size(12.0).color(theme::SUBTLE));
        ui.label(RichText::new(value).size(28.0).strong().color(theme::TEXT));
        ui.label(RichText::new(detail).size(12.0).color(theme::MUTED));
    });
}

fn hours(minutes: u32) -> String {
    if minutes < 60 { format!("{minutes}m") } else { format!("{:.1}h", minutes as f32 / 60.0) }
}

fn week_chart(ui: &mut egui::Ui, minutes: &[u32; 7]) {
    let height = 160.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    let max = minutes.iter().copied().max().unwrap_or(0).max(25) as f32;
    let label_h = 18.0;
    let value_h = 16.0;
    let slot = rect.width() / 7.0;
    let bar_w = (slot * 0.55).min(36.0);
    let today = Local::now().date_naive();
    let text_color = theme::SUBTLE;

    for (i, &m) in minutes.iter().enumerate() {
        let x = rect.left() + slot * (i as f32 + 0.5);
        let usable = height - label_h - value_h;
        let h = (m as f32 / max * usable).max(if m > 0 { 3.0 } else { 0.0 });
        let bottom = rect.bottom() - label_h;
        let bar =
            egui::Rect::from_min_max(egui::pos2(x - bar_w / 2.0, bottom - h), egui::pos2(x + bar_w / 2.0, bottom));
        let color = if i == 6 { theme::ACCENT } else { theme::ACCENT_DIM };
        painter.rect_filled(bar, 4, color);
        if m > 0 {
            painter.text(
                egui::pos2(x, bar.top() - 2.0),
                Align2::CENTER_BOTTOM,
                m.to_string(),
                FontId::proportional(11.0),
                text_color,
            );
        }
        let day = today.checked_sub_days(Days::new(6 - i as u64)).unwrap_or(today);
        let label = if i == 6 { "Today".to_string() } else { day.format("%a").to_string() };
        painter.text(
            egui::pos2(x, rect.bottom()),
            Align2::CENTER_BOTTOM,
            label,
            FontId::proportional(12.0),
            text_color,
        );
    }
}
