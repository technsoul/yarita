//! Painted pieces of the design: the dot-matrix clock, round controls, the
//! progress ring and small badges.

use std::f32::consts::TAU;

use eframe::egui::{
    self, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Vec2, WidgetInfo,
    WidgetType, pos2, vec2,
};

use crate::theme;

/// 5×7 dot glyphs for the digits 0–9, one string per row.
const GLYPHS: [[&str; 7]; 10] = [
    ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
    ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
    ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
    ["11111", "00010", "00100", "00010", "00001", "10001", "01110"],
    ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
    ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
    ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
    ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
    ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
    ["01110", "10001", "10001", "01111", "00001", "00010", "01100"],
];

/// Shows `secs` as MM:SS in dot-matrix tiles inside a dark well, filling the
/// available width up to `max_dot` pixels per dot.
pub fn dot_matrix(ui: &mut egui::Ui, secs: i64, max_dot: f32, colon_on: bool) -> Response {
    // Every measurement is a multiple of the dot diameter `d`.
    const GAP: f32 = 0.55; // between dots
    const TILE_PAD: f32 = 2.0; // inside a digit tile
    const ITEM_GAP: f32 = 1.0; // between tiles
    const WELL_PAD: f32 = 1.6; // inside the well
    let tile_w = 5.0 + 4.0 * GAP + 2.0 * TILE_PAD;
    let tile_h = 7.0 + 6.0 * GAP + 2.0 * TILE_PAD;
    let colon_w = 2.2;
    let units_w = 4.0 * tile_w + colon_w + 4.0 * ITEM_GAP + 2.0 * WELL_PAD;
    let units_h = tile_h + 2.0 * WELL_PAD;

    let width = ui.available_width();
    let d = (width / units_w).min(max_dot).max(2.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, units_h * d), Sense::hover());
    let painter = ui.painter_at(rect.expand(12.0));

    painter.rect(
        rect,
        0.3 * units_h * d / 2.0,
        theme::WELL,
        Stroke::new(1.0, Color32::from_rgb(0x1C, 0x1E, 0x22)),
        StrokeKind::Inside,
    );

    let secs = secs.max(0);
    let (m, s) = ((secs / 60).min(99), secs % 60);
    let digits = [m / 10, m % 10, s / 10, s % 10].map(|n| n as usize);
    let content_w = (units_w - 2.0 * WELL_PAD) * d;
    let mut x = rect.center().x - content_w / 2.0;
    let top = rect.top() + WELL_PAD * d;

    for (i, digit) in digits.iter().enumerate() {
        if i == 2 {
            let cx = x + colon_w * d / 2.0;
            let r = 0.55 * d;
            let color = if colon_on { theme::LIGHT } else { theme::FAINT };
            let mid = top + tile_h * d / 2.0;
            painter.circle_filled(pos2(cx, mid - 1.1 * d), r, color);
            painter.circle_filled(pos2(cx, mid + 1.1 * d), r, color);
            x += (colon_w + ITEM_GAP) * d;
        }
        let tile = Rect::from_min_size(pos2(x, top), vec2(tile_w * d, tile_h * d));
        painter.rect_filled(tile.translate(vec2(0.0, 0.25 * d)), d, Color32::from_black_alpha(110));
        painter.rect_filled(tile, d, theme::RAISED);
        painter.line_segment(
            [tile.left_top() + vec2(d, 0.5), tile.right_top() + vec2(-d, 0.5)],
            Stroke::new(1.0, Color32::from_white_alpha(10)),
        );
        for (row, bits) in GLYPHS[*digit].iter().enumerate() {
            for (col, bit) in bits.bytes().enumerate() {
                let c = pos2(
                    tile.left() + (TILE_PAD + col as f32 * (1.0 + GAP) + 0.5) * d,
                    tile.top() + (TILE_PAD + row as f32 * (1.0 + GAP) + 0.5) * d,
                );
                if bit == b'1' {
                    painter.circle_filled(c, 0.85 * d, Color32::from_white_alpha(14));
                    painter.circle_filled(c, d / 2.0, theme::LIGHT);
                } else {
                    painter.circle_filled(c, d / 2.0, theme::DOT_OFF);
                }
            }
        }
        x += (tile_w + ITEM_GAP) * d;
    }

    let label = format!("{:02}:{:02} remaining", m, s);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &label));
    response
}

#[derive(Clone, Copy)]
pub enum Icon {
    Play,
    Pause,
    Stop,
    Skip,
    Lock,
    Expand,
    Collapse,
    Close,
    Timer,
    Chart,
    Shield,
    Sliders,
}

/// Paints a stroke icon designed on a 24×24 grid, centered in `rect`.
pub fn paint_icon(painter: &Painter, icon: Icon, rect: Rect, color: Color32) {
    let k = rect.width().min(rect.height()) / 24.0;
    let o = rect.center() - vec2(12.0, 12.0) * k;
    let p = |x: f32, y: f32| o + vec2(x, y) * k;
    let stroke = Stroke::new(1.8 * k.max(0.7), color);
    let line = |pts: &[(f32, f32)]| {
        painter.add(Shape::line(pts.iter().map(|&(x, y)| p(x, y)).collect(), stroke));
    };
    match icon {
        Icon::Play => {
            painter.add(Shape::convex_polygon(vec![p(8.0, 5.0), p(19.5, 12.0), p(8.0, 19.0)], color, Stroke::NONE));
        }
        Icon::Pause => {
            for x in [6.0, 14.0] {
                painter.rect_filled(Rect::from_min_max(p(x, 5.0), p(x + 4.0, 19.0)), 1.5 * k, color);
            }
        }
        Icon::Stop => {
            painter.rect_stroke(Rect::from_min_max(p(5.0, 5.0), p(19.0, 19.0)), 3.0 * k, stroke, StrokeKind::Middle);
        }
        Icon::Skip => {
            line(&[(5.0, 5.0), (15.0, 12.0), (5.0, 19.0), (5.0, 5.0)]);
            line(&[(19.0, 5.0), (19.0, 19.0)]);
        }
        Icon::Lock => {
            painter.rect_stroke(Rect::from_min_max(p(5.0, 11.0), p(19.0, 21.0)), 2.5 * k, stroke, StrokeKind::Middle);
            let arc: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = TAU / 2.0 + TAU / 2.0 * i as f32 / 16.0;
                    p(12.0 + 4.0 * a.cos(), 8.0 + 4.0 * a.sin())
                })
                .collect();
            line(&[(8.0, 11.0), (8.0, 8.0)]);
            line(&[(16.0, 11.0), (16.0, 8.0)]);
            painter.add(Shape::line(arc, stroke));
        }
        Icon::Expand => {
            line(&[(15.0, 3.0), (21.0, 3.0), (21.0, 9.0)]);
            line(&[(9.0, 21.0), (3.0, 21.0), (3.0, 15.0)]);
            line(&[(21.0, 3.0), (14.0, 10.0)]);
            line(&[(3.0, 21.0), (10.0, 14.0)]);
        }
        Icon::Collapse => {
            line(&[(4.0, 14.0), (10.0, 14.0), (10.0, 20.0)]);
            line(&[(20.0, 10.0), (14.0, 10.0), (14.0, 4.0)]);
            line(&[(10.0, 14.0), (3.0, 21.0)]);
            line(&[(14.0, 10.0), (21.0, 3.0)]);
        }
        Icon::Close => {
            line(&[(6.0, 6.0), (18.0, 18.0)]);
            line(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Icon::Timer => {
            painter.circle_stroke(p(12.0, 13.0), 8.0 * k, stroke);
            line(&[(12.0, 9.0), (12.0, 13.0), (14.5, 15.0)]);
            line(&[(10.0, 2.0), (14.0, 2.0)]);
        }
        Icon::Chart => {
            line(&[(4.0, 20.0), (4.0, 10.0)]);
            line(&[(10.0, 20.0), (10.0, 4.0)]);
            line(&[(16.0, 20.0), (16.0, 13.0)]);
            line(&[(22.0, 20.0), (2.0, 20.0)]);
        }
        Icon::Shield => {
            line(&[(12.0, 3.0), (20.0, 6.0), (20.0, 12.0), (12.0, 21.0), (4.0, 12.0), (4.0, 6.0), (12.0, 3.0)]);
            line(&[(8.5, 12.0), (11.0, 14.5), (15.5, 9.5)]);
        }
        Icon::Sliders => {
            line(&[(4.0, 7.0), (14.0, 7.0)]);
            line(&[(18.0, 7.0), (20.0, 7.0)]);
            line(&[(4.0, 17.0), (8.0, 17.0)]);
            line(&[(12.0, 17.0), (20.0, 17.0)]);
            painter.circle_stroke(p(16.0, 7.0), 2.0 * k, stroke);
            painter.circle_stroke(p(10.0, 17.0), 2.0 * k, stroke);
        }
    }
}

fn interact(ui: &mut egui::Ui, size: Vec2, enabled: bool, label: &str) -> (Rect, Response) {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let label = label.to_string();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &label));
    let response = if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response };
    (rect, response)
}

/// A circular secondary control, like Stop or Skip.
pub fn round_button(ui: &mut egui::Ui, diameter: f32, icon: Icon, label: &str, enabled: bool) -> Response {
    let (rect, response) = interact(ui, Vec2::splat(diameter), enabled, label);
    let fill = if response.hovered() && enabled { theme::HOVER } else { theme::RAISED };
    let fg = if enabled { theme::TEXT } else { theme::FAINT };
    let painter = ui.painter();
    painter.circle(rect.center(), diameter / 2.0, fill, Stroke::new(1.0, theme::BORDER_STRONG));
    paint_icon(painter, icon, Rect::from_center_size(rect.center(), Vec2::splat(diameter * 0.36)), fg);
    response.on_hover_text(label)
}

/// The primary play/pause button, wrapped in a ring that shows `remaining`
/// (1.0 = full, 0.0 = done).
pub fn play_button(
    ui: &mut egui::Ui,
    diameter: f32,
    running: bool,
    remaining: f32,
    ring_color: Color32,
    label: &str,
    enabled: bool,
) -> Response {
    let (rect, response) = interact(ui, Vec2::splat(diameter), enabled, label);
    let painter = ui.painter();
    let c = rect.center();
    let ring_w = (diameter * 0.045).max(3.0);
    let ring_r = diameter / 2.0 - ring_w / 2.0 - 1.0;
    painter.circle_stroke(c, ring_r, Stroke::new(ring_w, theme::BORDER));
    if remaining > 0.0 {
        let steps = (96.0 * remaining).ceil().max(2.0) as usize;
        let pts: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = -TAU / 4.0 + TAU * remaining * i as f32 / steps as f32;
                c + ring_r * vec2(a.cos(), a.sin())
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(ring_w, ring_color)));
    }

    let inner = diameter / 2.0 - ring_w - diameter * 0.065;
    painter.circle_filled(c + vec2(0.0, diameter * 0.06), inner + 2.0, Color32::from_black_alpha(90));
    let fill = match (enabled, response.hovered()) {
        (false, _) => theme::MUTED,
        (true, true) => Color32::WHITE,
        (true, false) => theme::LIGHT,
    };
    painter.circle_filled(c, inner, fill);
    let icon = if running { Icon::Pause } else { Icon::Play };
    paint_icon(painter, icon, Rect::from_center_size(c, Vec2::splat(inner * 0.75)), theme::BG);
    response.on_hover_text(label)
}

/// A small icon-only button with no frame, for title bars.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str) -> Response {
    let (rect, response) = interact(ui, Vec2::splat(32.0), true, label);
    if response.hovered() {
        ui.painter().rect_filled(rect, 8, theme::HOVER);
    }
    let color = if response.hovered() { theme::TEXT } else { theme::SUBTLE };
    paint_icon(ui.painter(), icon, Rect::from_center_size(rect.center(), Vec2::splat(16.0)), color);
    response.on_hover_text(label)
}

/// The "LOOP" outline badge.
pub fn badge(ui: &mut egui::Ui, text: &str, size: f32) {
    let font = FontId::monospace(size);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, theme::ACCENT);
    let pad = vec2(size * 0.8, size * 0.45);
    let (rect, _) = ui.allocate_exact_size(galley.size() + 2.0 * pad, Sense::hover());
    ui.painter().rect_stroke(rect, 7, Stroke::new(1.0, theme::ACCENT_DIM), StrokeKind::Inside);
    ui.painter().galley(rect.min + pad, galley, theme::ACCENT);
}

/// Progress pills: `done` filled, then one for the current item, then empty.
pub fn pips(ui: &mut egui::Ui, total: u32, done: u32, current: bool) {
    let (w, h, gap) = (22.0, 6.0, 6.0);
    let n = total.max(1) as f32;
    let (rect, _) = ui.allocate_exact_size(vec2(n * w + (n - 1.0) * gap, h), Sense::hover());
    for i in 0..total {
        let color = if i < done {
            theme::ACCENT
        } else if i == done && current {
            theme::ACCENT_DIM
        } else {
            theme::DOT_OFF
        };
        let x = rect.left() + i as f32 * (w + gap);
        ui.painter().rect_filled(Rect::from_min_size(pos2(x, rect.top()), vec2(w, h)), 3, color);
    }
}

/// An on/off switch.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool, enabled: bool, label: &str) -> Response {
    let (rect, mut response) = interact(ui, vec2(44.0, 24.0), enabled, label);
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(response.id, *on);
    let track = if *on { theme::ACCENT_DIM } else { theme::DOT_OFF };
    let track = if enabled { track } else { track.gamma_multiply(0.6) };
    ui.painter().rect_filled(rect, 12, track);
    let knob_x = egui::lerp((rect.left() + 12.0)..=(rect.right() - 12.0), t);
    let knob = if enabled { theme::LIGHT } else { theme::MUTED };
    ui.painter().circle_filled(pos2(knob_x, rect.center().y), 9.0, knob);
    response
}

/// Shows which phase is current, styled as a segmented control.
pub fn phase_segments(ui: &mut egui::Ui, labels: &[&str], selected: usize) {
    let font = FontId::proportional(13.0);
    let pad = vec2(16.0, 0.0);
    let h = 36.0;
    let galleys: Vec<_> = labels
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let color = if i == selected { theme::BG } else { theme::MUTED };
            ui.painter().layout_no_wrap(l.to_string(), font.clone(), color)
        })
        .collect();
    let inner_w: f32 =
        galleys.iter().map(|g| g.size().x + 2.0 * pad.x).sum::<f32>() + 4.0 * (labels.len() as f32 - 1.0);
    let (rect, _) = ui.allocate_exact_size(vec2(inner_w + 8.0, h + 8.0), Sense::hover());
    let painter = ui.painter();
    painter.rect(rect, 12, theme::SURFACE, Stroke::new(1.0, theme::BORDER), StrokeKind::Inside);
    let mut x = rect.left() + 4.0;
    for (i, g) in galleys.into_iter().enumerate() {
        let seg = Rect::from_min_size(pos2(x, rect.top() + 4.0), vec2(g.size().x + 2.0 * pad.x, h));
        if i == selected {
            painter.rect_filled(seg, 9, theme::TEXT);
        }
        painter.galley(seg.center() - g.size() / 2.0, g, theme::TEXT);
        x = seg.right() + 4.0;
    }
}
