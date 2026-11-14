use std::path::PathBuf;

use egui_sdl3::sdl3::{
    VideoSubsystem,
    event::Event,
    pixels::PixelFormat,
    render::Texture,
    render::{Canvas, ScaleMode},
    video::Window,
};
use egui_sdl3::{EguiCanvas, egui};

use egui::{Color32, CornerRadius, FontId, RichText, Stroke, Vec2};

use crate::{
    RESOLUTION_X, RESOLUTION_Y, SCALE, config::Recent, palette::ColorPalette, session::Session,
};

#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub stroke: Color32,
    pub accent: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub on_accent: Color32,
    pub shades: [Color32; 4],
}

fn rgb(c: &[u8; 4]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn luminance(c: Color32) -> f32 {
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

fn contrast(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn chroma(c: Color32) -> u8 {
    c.r().max(c.g()).max(c.b()) - c.r().min(c.g()).min(c.b())
}

fn with_contrast(fg: Color32, bg: Color32, min_ratio: f32) -> Color32 {
    if contrast(fg, bg) >= min_ratio {
        return fg;
    }
    let (away, toward) = if luminance(fg) >= luminance(bg) {
        (Color32::WHITE, Color32::BLACK)
    } else {
        (Color32::BLACK, Color32::WHITE)
    };
    for target in [away, toward] {
        for i in 1..=40 {
            let c = mix(fg, target, i as f32 / 40.0);
            if contrast(c, bg) >= min_ratio {
                return c;
            }
        }
    }
    if contrast(Color32::WHITE, bg) > contrast(Color32::BLACK, bg) {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

impl Theme {
    pub fn from_palette(p: &ColorPalette) -> Self {
        let mut shades = [Color32::BLACK; 4];
        for (dst, src) in shades.iter_mut().zip(&p.colors) {
            *dst = rgb(src);
        }
        shades.sort_by(|a, b| luminance(*a).total_cmp(&luminance(*b)));
        let [dark, mid, mid2, light] = shades;

        let mut bg = mix(dark, mid, 0.15);
        while luminance(bg) > 0.022 {
            bg = mix(bg, Color32::BLACK, 0.12);
        }

        let tint = mix(light, Color32::WHITE, 0.2);
        let panel = mix(bg, tint, 0.07);
        let raised = mix(bg, tint, 0.13);
        let stroke = mix(bg, tint, 0.24);

        let accent_base = [mid, mid2, light]
            .into_iter()
            .max_by_key(|c| (chroma(*c), (luminance(*c) * 1000.0) as u32))
            .unwrap();
        let accent = with_contrast(accent_base, bg, 4.0);

        let text = with_contrast(mix(light, Color32::WHITE, 0.65), bg, 10.0);
        let dim = with_contrast(mix(mix(mid2, light, 0.5), bg, 0.15), bg, 4.8);

        let dark_ink = mix(bg, Color32::BLACK, 0.5);
        let on_accent = if contrast(dark_ink, accent) >= contrast(Color32::WHITE, accent) {
            with_contrast(dark_ink, accent, 4.5)
        } else {
            with_contrast(mix(light, Color32::WHITE, 0.8), accent, 4.5)
        };

        Self {
            bg,
            panel,
            raised,
            stroke,
            accent,
            text,
            dim,
            on_accent,
            shades,
        }
    }

    pub fn clear_color(&self) -> [u8; 4] {
        [self.bg.r(), self.bg.g(), self.bg.b(), 255]
    }

    pub fn apply(&self, ctx: &egui::Context) {
        use egui::{FontFamily, Shadow, TextStyle};

        let mut v = egui::Visuals::dark();
        v.panel_fill = self.bg;
        v.window_fill = self.panel;
        v.window_stroke = Stroke::new(1.0_f32, self.stroke);
        v.window_corner_radius = CornerRadius::same(16);
        v.window_shadow = Shadow {
            offset: [0, 12],
            blur: 40,
            spread: 0,
            color: Color32::from_black_alpha(140),
        };
        v.popup_shadow = Shadow {
            offset: [0, 6],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(110),
        };
        v.menu_corner_radius = CornerRadius::same(10);
        v.extreme_bg_color = mix(self.bg, Color32::BLACK, 0.35);
        v.faint_bg_color = self.panel;
        v.code_bg_color = self.raised;
        v.hyperlink_color = self.accent;
        v.override_text_color = Some(self.text);
        v.selection.bg_fill = mix(self.raised, self.accent, 0.45);
        v.selection.stroke = Stroke::new(1.0_f32, self.text);
        v.text_cursor.stroke = Stroke::new(2.0_f32, self.accent);

        let hover_fill = mix(self.raised, self.accent, 0.14);
        let active_fill = mix(self.raised, self.accent, 0.28);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = self.panel;
        w.noninteractive.weak_bg_fill = self.panel;
        w.noninteractive.bg_stroke = Stroke::new(1.0_f32, self.stroke);
        w.noninteractive.fg_stroke = Stroke::new(1.0_f32, self.text);

        w.inactive.bg_fill = self.raised;
        w.inactive.weak_bg_fill = self.raised;
        w.inactive.bg_stroke = Stroke::new(1.0_f32, self.stroke);
        w.inactive.fg_stroke = Stroke::new(1.0_f32, self.text);

        w.hovered.bg_fill = hover_fill;
        w.hovered.weak_bg_fill = hover_fill;
        w.hovered.bg_stroke = Stroke::new(1.0_f32, self.accent);
        w.hovered.fg_stroke = Stroke::new(1.5_f32, self.text);
        w.hovered.expansion = 0.0;

        w.active.bg_fill = active_fill;
        w.active.weak_bg_fill = active_fill;
        w.active.bg_stroke = Stroke::new(1.5_f32, self.accent);
        w.active.fg_stroke = Stroke::new(1.5_f32, self.text);
        w.active.expansion = 0.0;

        w.open = w.active;
        for s in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            s.corner_radius = CornerRadius::same(10);
        }

        ctx.set_visuals(v);
        ctx.global_style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(10.0, 10.0);
            s.spacing.button_padding = Vec2::new(16.0, 8.0);
            s.spacing.interact_size.y = 34.0;
            s.spacing.window_margin = egui::Margin::same(22);
            s.text_styles = [
                (
                    TextStyle::Heading,
                    FontId::new(24.0, FontFamily::Proportional),
                ),
                (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
                (
                    TextStyle::Button,
                    FontId::new(15.0, FontFamily::Proportional),
                ),
                (
                    TextStyle::Small,
                    FontId::new(12.0, FontFamily::Proportional),
                ),
                (
                    TextStyle::Monospace,
                    FontId::new(14.0, FontFamily::Monospace),
                ),
            ]
            .into();
        });
    }
}

fn section_label(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(11.5)
            .strong()
            .color(theme.dim),
    );
}

fn primary_button(ui: &mut egui::Ui, theme: &Theme, text: &str, size: Vec2) -> egui::Response {
    let button = egui::Button::new(RichText::new(text).strong().color(theme.on_accent))
        .fill(theme.accent)
        .stroke(Stroke::NONE)
        .min_size(size);
    let resp = ui.add(button);
    if resp.hovered() {
        ui.painter().rect_filled(
            resp.rect,
            CornerRadius::same(10),
            Color32::from_white_alpha(18),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn secondary_button(ui: &mut egui::Ui, text: &str, size: Vec2) -> egui::Response {
    ui.add(egui::Button::new(text).min_size(size))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn card<R>(
    ui: &mut egui::Ui,
    theme: &Theme,
    selected: bool,
    margin: egui::Margin,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::Response {
    let bg_idx = ui.painter().add(egui::Shape::Noop);
    let inner = egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add_contents(ui)
    });
    let resp = inner
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let (fill, stroke) = if selected {
        (
            mix(theme.raised, theme.accent, 0.16),
            Stroke::new(1.5_f32, theme.accent),
        )
    } else if resp.hovered() {
        (
            theme.raised,
            Stroke::new(1.0_f32, mix(theme.stroke, theme.accent, 0.6)),
        )
    } else {
        (theme.panel, Stroke::new(1.0_f32, theme.stroke))
    };
    ui.painter().set(
        bg_idx,
        egui::epaint::RectShape::new(
            resp.rect,
            CornerRadius::same(12),
            fill,
            stroke,
            egui::StrokeKind::Inside,
        ),
    );
    resp
}

fn swatches(ui: &mut egui::Ui, colors: &[[u8; 4]], size: f32) {
    let n = colors.len().max(1) as f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(size * n, size), egui::Sense::hover());
    let painter = ui.painter();
    for (i, c) in colors.iter().enumerate() {
        let x0 = rect.left() + size * i as f32;
        let cell = egui::Rect::from_min_size(egui::pos2(x0, rect.top()), Vec2::splat(size));
        let radius = match i {
            0 => CornerRadius {
                nw: 6,
                sw: 6,
                ne: 0,
                se: 0,
            },
            _ if i == colors.len() - 1 => CornerRadius {
                nw: 0,
                sw: 0,
                ne: 6,
                se: 6,
            },
            _ => CornerRadius::ZERO,
        };
        painter.rect_filled(cell, radius, rgb(c));
    }
    painter.rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0_f32, Color32::from_white_alpha(28)),
        egui::StrokeKind::Inside,
    );
}

fn logo(ui: &mut egui::Ui, theme: &Theme) {
    let size = Vec2::new(84.0, 76.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(14), theme.raised);
    p.rect_stroke(
        rect,
        CornerRadius::same(14),
        Stroke::new(1.0_f32, theme.stroke),
        egui::StrokeKind::Inside,
    );
    let screen = rect.shrink2(Vec2::new(12.0, 12.0));
    let band = screen.height() / 4.0;
    for (i, c) in theme.shades.iter().rev().enumerate() {
        let r = egui::Rect::from_min_size(
            egui::pos2(screen.left(), screen.top() + band * i as f32),
            Vec2::new(screen.width(), band),
        );
        let radius = match i {
            0 => CornerRadius {
                nw: 6,
                ne: 6,
                sw: 0,
                se: 0,
            },
            3 => CornerRadius {
                nw: 0,
                ne: 0,
                sw: 6,
                se: 6,
            },
            _ => CornerRadius::ZERO,
        };
        p.rect_filled(r, radius, *c);
    }
}

fn key_hint(ui: &mut egui::Ui, theme: &Theme, key: &str, what: &str) {
    egui::Frame::new()
        .fill(theme.raised)
        .stroke(Stroke::new(1.0_f32, theme.stroke))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(key).size(11.5).monospace().color(theme.text));
        });
    ui.label(RichText::new(what).size(11.5).color(theme.dim));
    ui.add_space(6.0);
}

fn backdrop(ctx: &egui::Context) {
    let layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new("settings_backdrop"));
    ctx.layer_painter(layer).rect_filled(
        ctx.content_rect(),
        CornerRadius::ZERO,
        Color32::from_black_alpha(150),
    );
}

pub enum UiAction {
    None,
    OpenRom(PathBuf),
    OpenRomDialog,
    _OpenSettings,
    ChoosePalette(usize),
    ChooseDefaultDirectory,
    StartHost,
    JoinHost(String),
}

pub struct SettingsView<'a> {
    pub palettes: &'a [ColorPalette],
    pub selected: usize,
    pub network_host: &'a mut String,
    pub network_port: &'a mut String,
    pub network_status: &'a str,
    pub host_port: Option<u16>,
}

pub struct Screen {
    egui: EguiCanvas,
    rgba: Vec<u8>,
    theme: Theme,
    themed_for: Option<usize>,
    theme_dirty: bool,
}

impl Screen {
    pub fn new(video: &VideoSubsystem, title: &str) -> Self {
        let mut egui = EguiCanvas::new(create_canvas(video, &title.to_owned()).into_window());
        egui.state.sync_window_size(egui.painter.canvas.window());
        Self {
            egui,
            rgba: vec![0; (RESOLUTION_X * RESOLUTION_Y * 4) as usize],
            theme: Theme::from_palette(&ColorPalette::default()),
            themed_for: None,
            theme_dirty: true,
        }
    }

    pub fn on_event(&mut self, event: &Event) -> bool {
        let window = self.egui.painter.canvas.window();
        let is_mouse = matches!(
            event,
            Event::MouseMotion { .. }
                | Event::MouseButtonDown { .. }
                | Event::MouseButtonUp { .. }
                | Event::MouseWheel { .. }
        );
        if !is_mouse {
            if matches!(event, Event::Window { .. }) {
                let consumed = self.egui.on_event(event).consumed;
                self.egui
                    .state
                    .sync_window_size(self.egui.painter.canvas.window());
                return consumed;
            }
            return self.egui.on_event(event).consumed;
        }

        self.egui.state.sync_window_size(window);
        let (w, _) = window.size();
        let (pw, _) = window.size_in_pixels();
        let scale = if w > 0 { pw as f32 / w as f32 } else { 1.0 };

        let mut e = event.clone();
        match &mut e {
            Event::MouseMotion { x, y, .. }
            | Event::MouseButtonDown { x, y, .. }
            | Event::MouseButtonUp { x, y, .. } => {
                *x *= scale;
                *y *= scale;
            }
            Event::MouseWheel {
                mouse_x, mouse_y, ..
            } => {
                *mouse_x *= scale;
                *mouse_y *= scale;
            }
            _ => {}
        }
        self.egui.on_event(&e).consumed
    }

    fn sync_theme(&mut self, view: &SettingsView) {
        if self.themed_for != Some(view.selected) {
            self.theme = Theme::from_palette(&view.palettes[view.selected]);
            self.themed_for = Some(view.selected);
            self.theme_dirty = true;
        }
    }

    pub fn draw(
        &mut self,
        session: &Session,
        palette: &ColorPalette,
        paused: bool,
        settings_open: &mut bool,
        view: &mut SettingsView,
    ) -> UiAction {
        self.sync_theme(view);

        to_rgba(
            session.frame_buffer(),
            &mut self.rgba,
            session.drawn(),
            palette,
        );
        let creator = self.egui.painter.canvas.texture_creator();
        let mut texture = creator
            .create_texture_streaming(PixelFormat::RGBA32, RESOLUTION_X, RESOLUTION_Y)
            .unwrap();
        texture.set_scale_mode(ScaleMode::Nearest);
        draw_screen(&mut self.egui.painter.canvas, &mut texture, &self.rgba);

        let mut action = UiAction::None;
        let theme = &self.theme;
        let dirty = &mut self.theme_dirty;
        self.egui.run(|ctx| {
            if std::mem::replace(dirty, false) {
                theme.apply(ctx);
            }
            if paused && !*settings_open {
                pause_overlay(ctx, theme);
            }
            if *settings_open {
                settings_window(ctx, settings_open, view, theme, &mut action);
            }
        });
        self.egui.paint();
        self.egui.present();
        action
    }

    #[allow(deprecated)]
    pub fn home(
        &mut self,
        recent: &[Recent],
        settings_open: &mut bool,
        view: &mut SettingsView,
    ) -> UiAction {
        self.sync_theme(view);

        let mut action = UiAction::None;
        self.egui.clear(self.theme.clear_color());

        let theme = &self.theme;
        let dirty = &mut self.theme_dirty;

        self.egui.run(|ctx| {
            if std::mem::replace(dirty, false) {
                theme.apply(ctx);
            }

            egui::Panel::bottom("hints")
                .frame(
                    egui::Frame::new()
                        .fill(theme.panel)
                        .stroke(Stroke::new(1.0_f32, theme.stroke))
                        .inner_margin(egui::Margin::symmetric(20, 12)),
                )
                .resizable(false)
                .show_separator_line(false)
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        key_hint(ui, theme, "Arrows", "D-pad");
                        key_hint(ui, theme, "Z", "A");
                        key_hint(ui, theme, "X", "B");
                        key_hint(ui, theme, "Enter", "Start");
                        key_hint(ui, theme, "Bksp", "Select");
                        key_hint(ui, theme, "P", "Pause");
                        key_hint(ui, theme, "C", "Palette");
                    });
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(theme.bg))
                .show(ctx, |ui| {
                    let width = 440.0_f32.min(ui.available_width() - 32.0);
                    ui.vertical_centered(|ui| {
                        ui.add_space(48.0);
                        logo(ui, theme);
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new("Good Boy")
                                .font(FontId::proportional(42.0))
                                .strong()
                                .color(theme.text),
                        );
                        ui.add_space(-6.0);
                        ui.label(
                            RichText::new("A Game Boy emulator")
                                .size(15.0)
                                .color(theme.dim),
                        );
                        ui.add_space(26.0);

                        // Buttons, centered as a group.
                        let gap = 10.0;
                        let (open_w, settings_w) = (220.0, 110.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            ui.add_space(
                                ((ui.available_width() - open_w - settings_w - gap) / 2.0).max(0.0),
                            );
                            if primary_button(ui, theme, "Open ROM…", Vec2::new(open_w, 44.0))
                                .clicked()
                            {
                                action = UiAction::OpenRomDialog;
                            }
                            if secondary_button(ui, "Settings", Vec2::new(settings_w, 44.0))
                                .on_hover_text("Cmd/Ctrl + ,")
                                .clicked()
                            {
                                *settings_open = true;
                            }
                        });

                        ui.add_space(36.0);

                        ui.allocate_ui_with_layout(
                            Vec2::new(width, ui.available_height()),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(width);
                                ui.horizontal(|ui| {
                                    section_label(ui, theme, "Recent");
                                    if !recent.is_empty() {
                                        ui.label(
                                            RichText::new(recent.len().to_string())
                                                .size(11.5)
                                                .color(theme.dim),
                                        );
                                    }
                                });
                                ui.add_space(2.0);
                                recent_list(ui, theme, recent, &mut action);
                            },
                        );
                    });
                });

            settings_window(ctx, settings_open, view, theme, &mut action);
        });
        self.egui.paint();
        self.egui.present();
        action
    }

    pub fn set_title(&mut self, title: &str) {
        self.egui
            .painter
            .canvas
            .window_mut()
            .set_title(title)
            .unwrap();
    }

    pub fn confirm_quit_without_saving(&self) -> bool {
        crate::config::show_save_error_dialog(&self.egui.painter.canvas)
    }
}

fn recent_list(ui: &mut egui::Ui, theme: &Theme, recent: &[Recent], action: &mut UiAction) {
    if recent.is_empty() {
        egui::Frame::new()
            .fill(theme.panel)
            .stroke(Stroke::new(1.0_f32, theme.stroke))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(24, 28))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Nothing played yet").strong().size(16.0));
                    ui.add_space(-4.0);
                    ui.label(
                        RichText::new("Open a ROM or drop a .gb file onto this window")
                            .color(theme.dim),
                    );
                });
            });
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            for (i, r) in recent.iter().enumerate() {
                let dir = r
                    .path
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();

                let resp = card(ui, theme, false, egui::Margin::symmetric(12, 10), |ui| {
                    ui.horizontal(|ui| {
                        // Initial badge, colored with a palette shade.
                        let badge_fill = with_contrast(theme.shades[1 + i % 3], theme.panel, 1.6);
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(38.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(9), badge_fill);
                        let initial = r
                            .name
                            .chars()
                            .find(|c| c.is_alphanumeric())
                            .map(|c| c.to_ascii_uppercase().to_string())
                            .unwrap_or_else(|| "?".into());
                        let ink = if contrast(Color32::WHITE, badge_fill)
                            >= contrast(Color32::BLACK, badge_fill)
                        {
                            Color32::WHITE
                        } else {
                            Color32::BLACK
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            initial,
                            FontId::proportional(18.0),
                            ink,
                        );

                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.add(
                                egui::Label::new(RichText::new(&r.name).strong().size(15.0))
                                    .truncate()
                                    .selectable(false),
                            );
                            ui.add(
                                egui::Label::new(RichText::new(&dir).size(12.0).color(theme.dim))
                                    .truncate()
                                    .selectable(false),
                            );
                        });
                    });
                });

                if resp.clicked() {
                    *action = UiAction::OpenRom(r.path.clone());
                }
            }
        });
}

fn pause_overlay(ctx: &egui::Context, theme: &Theme) {
    backdrop(ctx);
    egui::Area::new(egui::Id::new("paused"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(theme.panel)
                .stroke(Stroke::new(1.0_f32, theme.stroke))
                .corner_radius(CornerRadius::same(14))
                .inner_margin(egui::Margin::symmetric(28, 18))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Paused")
                                .size(26.0)
                                .strong()
                                .color(theme.text),
                        );
                        ui.label(RichText::new("Press P to resume").color(theme.dim));
                    });
                });
        });
}

fn settings_window(
    ctx: &egui::Context,
    open: &mut bool,
    view: &mut SettingsView,
    theme: &Theme,
    action: &mut UiAction,
) {
    if !*open {
        return;
    }

    backdrop(ctx);
    let mut close = false;

    egui::Window::new("Settings")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(380.0);

            // Header
            ui.horizontal(|ui| {
                ui.label(RichText::new("Settings").size(22.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let x = egui::Button::new(RichText::new("✕").size(14.0).color(theme.dim))
                        .frame(false)
                        .min_size(Vec2::splat(28.0));
                    if ui
                        .add(x)
                        .on_hover_text("Close (Esc)")
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
            ui.add_space(4.0);

            // Palette
            section_label(ui, theme, "Palette");
            ui.spacing_mut().item_spacing.y = 6.0;
            for (i, pal) in view.palettes.iter().enumerate() {
                let selected = i == view.selected;
                let resp = card(ui, theme, selected, egui::Margin::symmetric(12, 9), |ui| {
                    ui.horizontal(|ui| {
                        swatches(ui, &pal.colors, 20.0);
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(RichText::new(&pal.name).strong())
                                .truncate()
                                .selectable(false),
                        );
                        if selected {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::splat(10.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(rect.center(), 5.0, theme.accent);
                                },
                            );
                        }
                    });
                });
                if resp.clicked() {
                    *action = UiAction::ChoosePalette(i);
                }
            }
            ui.spacing_mut().item_spacing.y = 10.0;

            ui.add_space(10.0);
            section_label(ui, theme, "ROM folder");
            if secondary_button(ui, "Choose folder…", Vec2::new(0.0, 34.0)).clicked() {
                *action = UiAction::ChooseDefaultDirectory;
            }

            ui.add_space(10.0);
            section_label(ui, theme, "Link cable (TCP)");

            egui::Frame::new()
                .fill(theme.bg)
                .stroke(Stroke::new(1.0_f32, theme.stroke))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if let Some(port) = view.host_port {
                        ui.horizontal(|ui| {
                            ui.label("Hosting on port");
                            ui.label(
                                RichText::new(port.to_string())
                                    .monospace()
                                    .strong()
                                    .color(theme.accent),
                            );
                        });
                        ui.label(
                            RichText::new("Share this port with the other player.")
                                .small()
                                .color(theme.dim),
                        );
                    } else {
                        egui::Grid::new("link_grid")
                            .num_columns(2)
                            .spacing([12.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("Host").color(theme.dim));
                                ui.add(
                                    egui::TextEdit::singleline(view.network_host)
                                        .desired_width(200.0)
                                        .margin(egui::Margin::symmetric(8, 6)),
                                );
                                ui.end_row();
                                ui.label(RichText::new("Port").color(theme.dim));
                                ui.add(
                                    egui::TextEdit::singleline(view.network_port)
                                        .hint_text("e.g. 4000")
                                        .desired_width(90.0)
                                        .margin(egui::Margin::symmetric(8, 6)),
                                );
                                ui.end_row();
                            });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if primary_button(ui, theme, "Join game", Vec2::new(0.0, 34.0))
                                .clicked()
                            {
                                match view.network_port.trim().parse::<u16>() {
                                    Ok(port) if !view.network_host.trim().is_empty() => {
                                        *action = UiAction::JoinHost(format!(
                                            "{}:{port}",
                                            view.network_host.trim()
                                        ));
                                    }
                                    _ => println!("Enter a valid host address and port"),
                                }
                            }
                            if secondary_button(ui, "Host game", Vec2::new(0.0, 34.0)).clicked() {
                                *action = UiAction::StartHost;
                            }
                        });
                    }

                    if !view.network_status.is_empty() {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 4.0, theme.accent);
                            ui.label(RichText::new(view.network_status).small().color(theme.dim));
                        });
                    }
                });
        });

    if close {
        *open = false;
    }
}

pub fn create_canvas(video_subsystem: &VideoSubsystem, name: &String) -> Canvas<Window> {
    const WINDOW_WIDTH: u32 = (RESOLUTION_X as u32) * SCALE;
    const WINDOW_HEIGHT: u32 = (RESOLUTION_Y as u32) * SCALE;

    let window = video_subsystem
        .window(name, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .high_pixel_density()
        .opengl()
        .build()
        .unwrap();

    video_subsystem.text_input().start(&window);
    window.into_canvas()
}

pub fn draw_screen(canvas: &mut Canvas<Window>, texture: &mut Texture, rgba: &[u8]) {
    assert_eq!(rgba.len(), (RESOLUTION_X * RESOLUTION_Y * 4) as usize);
    texture
        .update(None, rgba, (RESOLUTION_X * 4) as usize)
        .unwrap();
    canvas.clear();
    canvas.copy(texture, None, None).unwrap();
}

pub fn to_rgba(shades: &[u8], out: &mut [u8], drawn: &[bool], palette: &ColorPalette) {
    for (i, (&shade, &is_drawn)) in shades.iter().zip(drawn).enumerate() {
        let rgba = if is_drawn {
            &palette.colors[(shade as usize) % palette.colors.len()]
        } else {
            &[0, 0, 0, 255]
        };

        out[i * 4..i * 4 + 4].copy_from_slice(rgba);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::builtin_palettes;

    #[test]
    fn every_palette_theme_is_readable() {
        for p in builtin_palettes() {
            let t = Theme::from_palette(&p);
            let checks = [
                ("text/bg", contrast(t.text, t.bg), 7.0),
                ("text/raised", contrast(t.text, t.raised), 7.0),
                ("dim/bg", contrast(t.dim, t.bg), 4.5),
                ("dim/panel", contrast(t.dim, t.panel), 4.0),
                ("accent/bg", contrast(t.accent, t.bg), 3.0),
                ("on_accent/accent", contrast(t.on_accent, t.accent), 4.5),
            ];
            for (name, ratio, min) in checks {
                println!("{:<28} {name:<18} {ratio:5.2}", p.name);
                assert!(
                    ratio >= min,
                    "{}: {name} contrast {ratio:.2} < {min}",
                    p.name
                );
            }
        }
    }
}
