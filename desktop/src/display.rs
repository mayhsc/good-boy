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

use crate::{RESOLUTION_X, RESOLUTION_Y, SCALE, config::Recent, palette::ColorPalette, session::Session};

#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color32,
    pub panel: Color32,
    pub hover: Color32,
    pub accent: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub on_accent: Color32,
}

fn rgb(c: &[u8; 4]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn luminance(c: Color32) -> f32 {
    0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32
}

fn ensure_contrast(fg: Color32, bg: Color32) -> Color32 {
    if (luminance(fg) - luminance(bg)).abs() >= 90.0 {
        fg
    } else if luminance(bg) > 128.0 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

impl Theme {
    pub fn from_palette(p: &ColorPalette) -> Self {
        let (c0, c1, c2, c3) = (
            rgb(&p.colors[0]),
            rgb(&p.colors[1]),
            rgb(&p.colors[2]),
            rgb(&p.colors[3]),
        );
        let bg = c3;
        Self {
            bg,
            panel: mix(c3, c2, 0.35),
            hover: mix(c3, c2, 0.7),
            accent: c1,
            text: ensure_contrast(c0, bg),
            dim: ensure_contrast(mix(c1, c2, 0.4), bg),
            on_accent: ensure_contrast(c3, c1),
        }
    }

    pub fn clear_color(&self) -> [u8; 4] {
        [self.bg.r(), self.bg.g(), self.bg.b(), 255]
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut v = egui::Visuals::dark();
        v.panel_fill = self.bg;
        v.window_fill = self.panel;
        v.window_stroke = Stroke::new(1.0f32, self.hover);
        v.window_corner_radius = CornerRadius::same(12);
        v.override_text_color = Some(self.text);
        v.selection.bg_fill = self.accent.gamma_multiply(0.45);
        v.selection.stroke = Stroke::new(1.0f32, self.accent);

        let w = &mut v.widgets;
        w.noninteractive.bg_stroke = Stroke::new(1.0f32, self.hover);
        w.inactive.bg_fill = self.panel;
        w.inactive.weak_bg_fill = self.panel;
        w.hovered.bg_fill = self.hover;
        w.hovered.weak_bg_fill = self.hover;
        w.hovered.bg_stroke = Stroke::new(1.0f32, self.accent);
        w.active.bg_fill = self.hover;
        w.active.weak_bg_fill = self.hover;
        w.active.bg_stroke = Stroke::new(1.5f32, self.accent);
        for s in [&mut w.inactive, &mut w.hovered, &mut w.active] {
            s.corner_radius = CornerRadius::same(8);
        }

        ctx.set_visuals(v);
        ctx.global_style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(10.0, 10.0);
            s.spacing.button_padding = Vec2::new(14.0, 8.0);
            s.spacing.interact_size.y = 32.0;
        });
    }
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
        Self {
            egui: EguiCanvas::new(create_canvas(video, &title.to_owned()).into_window()),
            rgba: vec![0; (RESOLUTION_X * RESOLUTION_Y * 4) as usize],
            theme: Theme::from_palette(&ColorPalette::default()),
            themed_for: None,
            theme_dirty: true,
        }
    }

    pub fn on_event(&mut self, event: &egui_sdl3::sdl3::event::Event) -> bool {
        let scale = self.egui.painter.canvas.window().display_scale(); 
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

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(
                        RichText::new("Good Boy")
                            .font(FontId::proportional(40.0))
                            .strong()
                            .color(theme.accent),
                    );
                    ui.label(RichText::new("Game Boy emulator").color(theme.dim));
                    ui.add_space(24.0);

                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 300.0).max(0.0) / 2.0);
                        let open = egui::Button::new(
                            RichText::new("Open ROM").strong().color(theme.on_accent),
                        )
                        .fill(theme.accent);
                        if ui.add_sized([200.0, 40.0], open).clicked() {
                            action = UiAction::OpenRomDialog;
                        }
                        if ui
                            .add_sized([90.0, 40.0], egui::Button::new("Settings"))
                            .on_hover_text("Cmd/Ctrl + ,")
                            .clicked()
                        {
                            *settings_open = true;
                        }
                    });

                    ui.add_space(28.0);
                    ui.label(RichText::new("RECENT").small().color(theme.dim));
                    ui.add_space(4.0);

                    if recent.is_empty() {
                        egui::Frame::new()
                            .fill(theme.panel)
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(24))
                            .show(ui, |ui| {
                                ui.set_width(380.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new("Nothing played yet").strong());
                                    ui.label(
                                        RichText::new("Drop a .gb file on this window")
                                            .color(theme.dim),
                                    );
                                });
                            });
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for r in recent {
                                // let name = path
                                //     .file_stem()
                                //     .and_then(|n| n.to_str())
                                //     .unwrap_or("Untitled");
                                let name = r.name.clone();
                                let dir = r.path
                                    .parent()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_default();

                                let resp = egui::Frame::new()
                                    .fill(theme.panel)
                                    .corner_radius(CornerRadius::same(10))
                                    .inner_margin(egui::Margin::symmetric(16, 10))
                                    .show(ui, |ui| {
                                        ui.set_width(380.0);
                                        ui.label(RichText::new(name).strong());
                                        ui.label(RichText::new(dir).small().color(theme.dim));
                                    })
                                    .response
                                    .interact(egui::Sense::click());

                                if resp
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    action = UiAction::OpenRom(r.path.clone());
                                }
                            }
                        });
                    }
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

fn swatches(ui: &mut egui::Ui, colors: &[[u8; 4]], size: f32) {
    ui.spacing_mut().item_spacing.x = 0.0;
    for c in colors {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        ui.painter().rect_filled(
            rect,
            CornerRadius::ZERO,
            Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
        );
    }
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

    egui::Window::new("Settings")
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(RichText::new("PALETTE").small().color(theme.dim));

            for (i, pal) in view.palettes.iter().enumerate() {
                let selected = i == view.selected;
                let resp = egui::Frame::new()
                    .fill(if selected {
                        theme.accent.gamma_multiply(0.15)
                    } else {
                        theme.hover
                    })
                    .stroke(if selected {
                        Stroke::new(1.5f32, theme.accent)
                    } else {
                        Stroke::NONE
                    })
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            swatches(ui, &pal.colors, 22.0);
                            ui.add_space(8.0);
                            ui.label(RichText::new(&pal.name).strong());
                        });
                    })
                    .response
                    .interact(egui::Sense::click());

                if resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    *action = UiAction::ChoosePalette(i);
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.label(RichText::new("ROM FOLDER").small().color(theme.dim));
            if ui.button("Choose folder…").clicked() {
                *action = UiAction::ChooseDefaultDirectory;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.label(RichText::new("TCP LINK CABLE").small().color(theme.dim));

            if let Some(port) = view.host_port {
                ui.label(format!("Hosting on port {port}"));
                ui.label(
                    RichText::new("Share this port with the other player.")
                        .small()
                        .color(theme.dim),
                );
            } else {
                ui.horizontal(|ui| {
                    ui.label("Host");
                    ui.add(egui::TextEdit::singleline(view.network_host).desired_width(185.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Port");
                    ui.add(egui::TextEdit::singleline(view.network_port).desired_width(80.0));
                });
                ui.horizontal(|ui| {
                    if ui.button("Host game").clicked() {
                        *action = UiAction::StartHost;
                    }
                    if ui.button("Join game").clicked() {
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
                });
            }

            if !view.network_status.is_empty() {
                ui.label(RichText::new(view.network_status).small().color(theme.dim));
            }
        });
}

pub fn create_canvas(video_subsystem: &VideoSubsystem, name: &String) -> Canvas<Window> {
    const WINDOW_WIDTH: u32 = (RESOLUTION_X as u32) * SCALE;
    const WINDOW_HEIGHT: u32 = (RESOLUTION_Y as u32) * SCALE;

    let window = video_subsystem
        .window(name, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        // .high_pixel_density()
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
