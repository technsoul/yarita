use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Theme, ThemePreference, vec2,
};
use yarita_protocol::PhaseDto;

pub const BG: Color32 = Color32::from_rgb(0x0B, 0x0C, 0x0E);
pub const WELL: Color32 = Color32::from_rgb(0x05, 0x05, 0x06);
pub const SIDEBAR: Color32 = Color32::from_rgb(0x12, 0x13, 0x16);
pub const SURFACE: Color32 = Color32::from_rgb(0x15, 0x17, 0x1A);
pub const RAISED: Color32 = Color32::from_rgb(0x1A, 0x1C, 0x1F);
pub const HOVER: Color32 = Color32::from_rgb(0x22, 0x25, 0x2A);
pub const SELECTED: Color32 = Color32::from_rgb(0x1E, 0x22, 0x28);
pub const NOW_BG: Color32 = Color32::from_rgb(0x12, 0x18, 0x22);
pub const BORDER: Color32 = Color32::from_rgb(0x1F, 0x22, 0x26);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x26, 0x29, 0x2D);
pub const DOT_OFF: Color32 = Color32::from_rgb(0x2A, 0x2D, 0x32);
pub const FAINT: Color32 = Color32::from_rgb(0x3A, 0x3D, 0x43);

pub const TEXT: Color32 = Color32::from_rgb(0xEC, 0xEC, 0xEA);
pub const TEXT_SOFT: Color32 = Color32::from_rgb(0xC9, 0xCB, 0xCF);
pub const MUTED: Color32 = Color32::from_rgb(0xA4, 0xA7, 0xAD);
pub const SUBTLE: Color32 = Color32::from_rgb(0x8B, 0x8F, 0x96);
pub const LIGHT: Color32 = Color32::from_rgb(0xF4, 0xF3, 0xEE);

pub const ACCENT: Color32 = Color32::from_rgb(0x9C, 0xC4, 0xFF);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x4E, 0x79, 0xB5);
pub const ACCENT_EDGE: Color32 = Color32::from_rgb(0x2B, 0x3A, 0x52);
pub const BREAK: Color32 = Color32::from_rgb(0x8F, 0xD6, 0xB4);
pub const WARN: Color32 = Color32::from_rgb(0xF2, 0xA6, 0x5A);

pub fn phase_color(phase: PhaseDto) -> Color32 {
    match phase {
        PhaseDto::Focus => ACCENT,
        PhaseDto::ShortBreak | PhaseDto::LongBreak => BREAK,
    }
}

pub fn phase_label(phase: PhaseDto) -> &'static str {
    match phase {
        PhaseDto::Focus => "Focus",
        PhaseDto::ShortBreak => "Short break",
        PhaseDto::LongBreak => "Long break",
    }
}

/// A rounded, bordered surface used for cards throughout the app.
pub fn card() -> egui::Frame {
    egui::Frame::new().fill(SURFACE).stroke(Stroke::new(1.0, BORDER)).corner_radius(16).inner_margin(Margin::same(18))
}

pub fn apply(ctx: &egui::Context) {
    // The design is dark only.
    ctx.set_theme(ThemePreference::Dark);
    ctx.style_mut_of(Theme::Dark, |style| {
        style.spacing.item_spacing = vec2(10.0, 10.0);
        style.spacing.button_padding = vec2(14.0, 8.0);
        style.spacing.window_margin = Margin::same(16);
        style.spacing.interact_size.y = 32.0;

        style.text_styles.insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Button, FontId::new(14.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = BG;
        v.window_fill = SURFACE;
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.extreme_bg_color = BG;
        v.faint_bg_color = SURFACE;
        v.code_bg_color = RAISED;
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = WARN;
        v.error_fg_color = WARN;
        v.weak_text_color = Some(SUBTLE);
        v.selection.bg_fill = ACCENT_EDGE;
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.window_corner_radius = CornerRadius::same(14);
        v.menu_corner_radius = CornerRadius::same(12);

        let r = CornerRadius::same(10);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = SURFACE;
        w.noninteractive.weak_bg_fill = SURFACE;
        w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        for (state, fill, stroke) in [
            (&mut w.inactive, RAISED, BORDER_STRONG),
            (&mut w.hovered, HOVER, FAINT),
            (&mut w.active, DOT_OFF, ACCENT_DIM),
            (&mut w.open, HOVER, FAINT),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.fg_stroke = Stroke::new(1.5, TEXT);
            state.expansion = 0.0;
        }
        for state in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            state.corner_radius = r;
        }
    });
}
