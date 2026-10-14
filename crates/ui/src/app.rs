use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Frame, Layout, Margin, RichText, Sense, Stroke, StrokeKind, vec2};
use focus_protocol::{ConfigDto, PhaseDto, Request, Snapshot};

use crate::client::ServiceClient;
use crate::widgets::{self, Icon};
use crate::{launcher, theme, views};

pub const FULL_SIZE: [f32; 2] = [1180.0, 780.0];
pub const FULL_MIN_SIZE: [f32; 2] = [760.0, 560.0];
const MINI_SIZE: [f32; 2] = [420.0, 440.0];
const MINI_MIN_SIZE: [f32; 2] = [360.0, 400.0];
/// Below this window width the session plan moves under the timer.
const RAIL_MIN_WIDTH: f32 = 1080.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Timer,
    Blocklist,
    Stats,
    Settings,
}

pub struct FocusUi {
    client: ServiceClient,
    pub tab: Tab,
    mini: bool,
    pub site_input: String,
    pub app_input: String,
    pub config_draft: Option<ConfigDto>,
    last_completions: Option<u64>,
    launch_message: Option<String>,
    auto_launched: bool,
}

impl FocusUi {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self {
            client: ServiceClient::spawn(cc.egui_ctx.clone()),
            tab: Tab::Timer,
            mini: false,
            site_input: String::new(),
            app_input: String::new(),
            config_draft: None,
            last_completions: None,
            launch_message: None,
            auto_launched: false,
        }
    }

    pub fn send(&self, request: Request) {
        self.client.send(request);
    }

    /// Switches between the full window and the compact, always-on-top widget.
    pub fn set_mini(&mut self, ctx: &egui::Context, mini: bool) {
        use egui::{ViewportCommand as Cmd, WindowLevel};
        self.mini = mini;
        if mini {
            ctx.send_viewport_cmd(Cmd::Decorations(false));
            ctx.send_viewport_cmd(Cmd::MinInnerSize(MINI_MIN_SIZE.into()));
            ctx.send_viewport_cmd(Cmd::InnerSize(MINI_SIZE.into()));
            ctx.send_viewport_cmd(Cmd::WindowLevel(WindowLevel::AlwaysOnTop));
        } else {
            ctx.send_viewport_cmd(Cmd::WindowLevel(WindowLevel::Normal));
            ctx.send_viewport_cmd(Cmd::Decorations(true));
            ctx.send_viewport_cmd(Cmd::InnerSize(FULL_SIZE.into()));
            ctx.send_viewport_cmd(Cmd::MinInnerSize(FULL_MIN_SIZE.into()));
        }
    }

    fn launch_service(&mut self) {
        self.launch_message = Some(match launcher::start_service() {
            Ok(()) => "Starting the service… approve the admin prompt if one appears.".into(),
            Err(e) => e,
        });
    }

    fn notify_on_completion(&mut self, ctx: &egui::Context, snap: &Snapshot) {
        let previous = self.last_completions.replace(snap.completions);
        if !previous.is_some_and(|p| snap.completions > p) {
            return;
        }
        // `phase` is now the phase that comes next.
        let (title, body) = match snap.phase {
            PhaseDto::Focus => ("Break's over", "Ready for the next focus session?"),
            PhaseDto::ShortBreak => ("Focus session done", "Nice work. Take a short break."),
            PhaseDto::LongBreak => ("Cycle complete", "Great job. Time for a long break."),
        };
        std::thread::spawn(move || {
            let _ = notify_rust::Notification::new().appname("Focus Timer").summary(title).body(body).show();
        });
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
    }

    fn offline_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);
            ui.label(RichText::new("Focus service isn't running").size(20.0).strong().color(theme::TEXT));
            ui.label("The service enforces blocking in the background, even when this window is closed. It needs admin rights to block websites.");
            ui.add_space(12.0);
            if ui.button(RichText::new("Start service").size(16.0)).clicked() {
                self.launch_service();
            }
            if let Some(msg) = &self.launch_message {
                ui.add_space(8.0);
                ui.label(RichText::new(msg).weak());
            }
        });
    }
}

impl eframe::App for FocusUi {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let (snapshot, connected, attempted, refusal) = self.client.with_state(|s| {
            if s.refusal.as_ref().is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(4)) {
                s.refusal = None;
            }
            (s.snapshot.clone(), s.connected, s.attempted, s.refusal.as_ref().map(|(r, _)| r.clone()))
        });

        // Strict mode: the window stays open until the session ends. This
        // covers the title bar, Alt+F4, the taskbar and the widget's button.
        let locked = connected && snapshot.as_ref().is_some_and(|s| s.locked);
        if locked && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.client.with_state(|s| {
                s.refusal = Some(("Strict mode is on: Focus Timer stays open until this session ends.".into(), Instant::now()));
            });
            ctx.request_repaint();
        }

        if attempted && !connected && !self.auto_launched {
            self.auto_launched = true;
            self.launch_service();
        }
        if connected {
            self.launch_message = None;
        }
        // Keep the "service offline" view responsive while reconnecting.
        ctx.request_repaint_after(Duration::from_secs(1));

        let page = Frame::new().fill(theme::BG).inner_margin(Margin::symmetric(40, 32));
        let (Some(snap), true) = (snapshot, connected) else {
            egui::CentralPanel::default().frame(page).show(ui, |ui| {
                if attempted {
                    self.offline_view(ui)
                } else {
                    ui.centered_and_justified(|ui| ui.spinner());
                }
            });
            return;
        };
        self.notify_on_completion(&ctx, &snap);

        if let Some(reason) = refusal {
            egui::Panel::bottom("refusal")
                .frame(Frame::new().fill(theme::SURFACE).inner_margin(Margin::symmetric(20, 10)))
                .show(ui, |ui| {
                    ui.label(RichText::new(reason).color(theme::WARN));
                });
        }

        if self.mini {
            // The window is frameless: the widget fills it edge to edge.
            egui::CentralPanel::default().frame(Frame::new().fill(theme::SIDEBAR).inner_margin(20)).show(ui, |ui| {
                views::timer::mini(self, ui, &snap);
            });
            return;
        }

        egui::Panel::left("nav")
            .exact_size(248.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(theme::BG).inner_margin(14))
            .show(ui, |ui| self.sidebar(ui, &snap));

        let wide = ctx.content_rect().width() >= RAIL_MIN_WIDTH;
        let show_rail = self.tab == Tab::Timer && wide;
        if show_rail {
            egui::Panel::right("plan")
                .exact_size(300.0)
                .resizable(false)
                .frame(Frame::new().fill(theme::BG).inner_margin(Margin::symmetric(24, 32)))
                .show(ui, |ui| {
                    egui::Panel::bottom("blocking")
                        .frame(Frame::NONE)
                        .show_separator_line(false)
                        .resizable(false)
                        .show(ui, |ui| views::timer::blocking_card(self, ui, &snap));
                    views::timer::plan(ui, &snap);
                });
        }

        egui::CentralPanel::default().frame(page).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                centered_column(ui, |ui| match self.tab {
                    Tab::Timer => {
                        views::timer::show(self, ui, &snap);
                        if !show_rail {
                            views::timer::blocking_card(self, ui, &snap);
                            views::timer::plan(ui, &snap);
                        }
                    }
                    Tab::Blocklist => views::blocklist::show(self, ui, &snap),
                    Tab::Stats => views::stats::show(ui, &snap),
                    Tab::Settings => views::settings::show(self, ui, &snap),
                })
            });
        });
    }
}

impl FocusUi {
    fn sidebar(&mut self, ui: &mut egui::Ui, snap: &Snapshot) {
        Frame::new()
            .fill(theme::SIDEBAR)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(22)
            .inner_margin(Margin::symmetric(14, 18))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                ui.spacing_mut().item_spacing.y = 2.0;

                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    let (rect, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                    ui.painter().rect_filled(rect, 11, theme::ACCENT);
                    widgets::paint_icon(ui.painter(), Icon::Timer, rect.shrink(8.0), theme::BG);
                    ui.label(RichText::new("Focus Timer").size(17.0).strong().color(theme::TEXT));
                });
                ui.add_space(22.0);

                for (tab, icon, label) in [
                    (Tab::Timer, Icon::Timer, "Focus"),
                    (Tab::Stats, Icon::Chart, "Statistics"),
                    (Tab::Blocklist, Icon::Shield, "Block list"),
                ] {
                    if nav_item(ui, icon, label, self.tab == tab).clicked() {
                        self.tab = tab;
                    }
                }

                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    today_card(ui, snap);
                    ui.add_space(8.0);
                    if nav_item(ui, Icon::Sliders, "Settings", self.tab == Tab::Settings).clicked() {
                        self.tab = Tab::Settings;
                    }
                    if nav_item(ui, Icon::Collapse, "Mini widget", false)
                        .on_hover_text("A small timer that stays on top of other windows")
                        .clicked()
                    {
                        self.set_mini(ui.ctx(), true);
                    }
                    ui.add_space(6.0);
                    ui.separator();
                });
            });
    }
}

fn nav_item(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::click());
    let name = label.to_string();
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &name));
    let painter = ui.painter();
    if selected {
        painter.rect(rect, 12, theme::SELECTED, Stroke::new(1.0, theme::ACCENT_EDGE), StrokeKind::Inside);
    } else if response.hovered() {
        painter.rect_filled(rect, 12, theme::RAISED);
    }
    let (icon_color, text_color) = match (selected, response.hovered()) {
        (true, _) => (theme::ACCENT, theme::TEXT),
        (false, true) => (theme::TEXT, theme::TEXT),
        (false, false) => (theme::MUTED, theme::MUTED),
    };
    let icon_rect = egui::Rect::from_center_size(rect.left_center() + vec2(21.0, 0.0), vec2(18.0, 18.0));
    widgets::paint_icon(painter, icon, icon_rect, icon_color);
    painter.text(
        rect.left_center() + vec2(42.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        text_color,
    );
    if selected {
        painter.circle_filled(rect.right_center() - vec2(15.0, 0.0), 3.0, theme::ACCENT);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn today_card(ui: &mut egui::Ui, snap: &Snapshot) {
    Frame::new().fill(theme::BG).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(14).inner_margin(10).show(
        ui,
        |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 16.0, theme::ACCENT_EDGE);
                widgets::paint_icon(ui.painter(), Icon::Chart, rect.shrink(9.0), theme::ACCENT);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new("Today").size(13.0).strong().color(theme::TEXT));
                    let n = snap.stats.today_sessions;
                    let plural = if n == 1 { "" } else { "s" };
                    let minutes = views::timer::duration(snap.stats.today_minutes);
                    ui.label(RichText::new(format!("{n} session{plural} · {minutes}")).size(11.0).color(theme::SUBTLE));
                });
            });
        },
    );
}

/// Keeps content in a readable column when the window is wide or maximized.
fn centered_column(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    const MAX_WIDTH: f32 = 760.0;
    let width = ui.available_width().min(MAX_WIDTH);
    let margin = (ui.available_width() - width) / 2.0;
    ui.horizontal_top(|ui| {
        ui.add_space(margin);
        ui.vertical(|ui| {
            ui.set_width(width);
            add(ui);
        });
    });
}
