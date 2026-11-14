use std::io;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::{path::Path, time::Duration};

use egui_sdl3::sdl3::{EventPump, Sdl};
use gb_core::serial::link::TcpLink;

use crate::config::{Config, Recent, load_config, store_config};
use crate::display::{SettingsView, UiAction};
use crate::network::{start_host, start_join};
use crate::palette::{ColorPalette, builtin_palettes};
use crate::{
    audio::AudioOutput,
    display::Screen,
    input::{self, Action},
    session::Session,
};

const FRAME_BYTES: i32 = 735 * 2 * 4;

pub struct App {
    events: EventPump,
    screen: Screen,
    audio: AudioOutput,
    session: Option<Session>,
    palettes: Vec<ColorPalette>,
    palette: usize,
    paused: bool,
    recent: Vec<Recent>,
    settings_open: bool,
    pending: Option<Receiver<io::Result<TcpLink>>>,
    network_status: String,
    network_host: String,
    network_port: String,
    host_port: Option<u16>,
    link: Option<TcpLink>,
    config: Config,
}

impl App {
    pub fn new(sdl: Sdl, port: Option<u16>) -> Self {
        let video = sdl.video().unwrap();
        let audio = sdl.audio().unwrap();
        let pending = match port {
            Some(p) => Some(start_join(format!("127.0.0.1:{p}").to_owned())),
            None => None,
        };
        let config = match load_config() {
            Ok(c) => c,
            Err(_) => {
                let config = Config::default();
                store_config(&config).unwrap();
                config
            }
        };
        Self {
            events: sdl.event_pump().unwrap(),
            screen: Screen::new(&video, "Good Boy"),
            audio: AudioOutput::new(&audio),
            session: None,
            palettes: builtin_palettes(),
            palette: config.palette,
            paused: false,
            recent: Vec::new(),
            settings_open: false,
            pending: pending,
            network_status: "".to_owned(),
            network_host: "127.0.0.1".to_owned(),
            network_port: port.map_or_else(|| "".to_owned(), |port| port.to_string()),
            host_port: None,
            link: None,
            config: config,
        }
    }

    pub fn run(&mut self) {
        loop {
            while let Some(event) = self.events.poll_event() {
                let consumed = self.screen.on_event(&event);
                if !consumed {
                    if let Some(action) = input::translate(event) {
                        if !self.handle(action) {
                            return;
                        } // false = quit
                    }
                }
            }
            self.frame();
        }
    }

    fn update_config(&mut self, new_config: Config) {
        store_config(&new_config).unwrap();
        self.config = new_config;
    }

    fn handle(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return !(self.try_save() || self.screen.confirm_quit_without_saving()),
            Action::TogglePause => self.paused = !self.paused,
            Action::NextPalette => self.select_palette(self.palette + 1),
            Action::DumpVram => {
                if let Some(s) = &self.session {
                    s.dump_vram()
                }
            }
            Action::ToggleSettings => self.settings_open = !self.settings_open,
            Action::CloseSettings => self.settings_open = false,
            Action::OpenRom(path) => self.open_rom(&path),
            Action::Button(b, down) => {
                if let Some(s) = &mut self.session {
                    s.press(b, down)
                }
            }
            Action::StartHost => match &mut self.pending {
                Some(_) => {
                    println!("Host already active");
                }
                None => match start_host() {
                    Ok((port, receiver)) => {
                        self.host_port = Some(port);
                        self.network_status = "Waiting for another player...".to_owned();
                        self.pending = Some(receiver);
                    }
                    Err(error) => {
                        self.network_status = format!("Could not host: {error}");
                    }
                },
            },
            Action::JoinHost(address) => {
                if self.pending.is_some() {
                    self.network_status = "A connection is already in progress".to_owned();
                } else {
                    self.host_port = None;
                    self.network_status = format!("Connecting to {address}...");
                    self.pending = Some(start_join(address));
                }
            }
        }
        true
    }

    fn frame(&mut self) {
        if self.pending.is_some() {
            self.poll_connection();
        }
        while self.audio.queued_bytes() > 2 * FRAME_BYTES {
            std::thread::sleep(Duration::from_millis(1));
        }

        // No game loaded: show the home screen.
        if self.session.is_none() {
            let action = {
                let mut view = SettingsView {
                    palettes: &self.palettes,
                    selected: self.palette,
                    network_host: &mut self.network_host,
                    network_port: &mut self.network_port,
                    network_status: &self.network_status,
                    host_port: self.host_port,
                };
                self.screen
                    .home(&self.recent, &mut self.settings_open, &mut view)
            };
            self.handle_ui_action(action);
            return;
        }

        if self.paused {
            std::thread::sleep(Duration::from_millis(16));
            return;
        }

        // Keep the game frozen while the settings window is open, but keep drawing.
        let session = self.session.as_mut().unwrap();
        if !self.settings_open {
            session.run_frame();
            self.audio.submit(session);
        } else {
            std::thread::sleep(Duration::from_millis(16));
        }

        let action = {
            let mut view = SettingsView {
                palettes: &self.palettes,
                selected: self.palette,
                network_host: &mut self.network_host,
                network_port: &mut self.network_port,
                network_status: &self.network_status,
                host_port: self.host_port,
            };
            self.screen.draw(
                self.session.as_ref().unwrap(),
                &self.palettes[self.palette],
                &mut self.settings_open,
                &mut view,
            )
        };
        self.handle_ui_action(action);
    }

    fn handle_ui_action(&mut self, action: UiAction) {
        match action {
            UiAction::OpenRom(path) => self.open_rom(&path),
            UiAction::OpenRomDialog => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Game Boy", &["gb", "gbc"])
                    .pick_file()
                {
                    self.open_rom(&path);
                }
            }
            UiAction::ChoosePalette(index) => self.select_palette(index),
            UiAction::ChooseDefaultDirectory => {
                println!("Choose default ROM directory requested")
            }
            UiAction::StartHost => {
                let _ = self.handle(Action::StartHost);
            }
            UiAction::JoinHost(address) => {
                let _ = self.handle(Action::JoinHost(address));
            }
            UiAction::_OpenSettings | UiAction::None => {}
        }
    }

    fn select_palette(&mut self, palette: usize) {
        let palette = (palette) % self.palettes.len();
        self.palette = palette;
        self.update_config(Config {
            palette: palette,
            ..self.config.clone()
        })
    }

    fn poll_connection(&mut self) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(Ok(link)) => {
                    self.attach_link(link);
                    self.pending = None;
                    self.network_status = "Connected".to_owned();
                }
                Ok(Err(e)) => {
                    self.network_status = format!("Link failed: {e}");
                    self.pending = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.network_status = format!("Disconnected")
                }
            }
        }
    }

    fn attach_link(&mut self, link: TcpLink) {
        if let Some(s) = &mut self.session {
            s.attach_link(link);
        } else {
            self.link = Some(link);
        }
    }

    pub fn open_rom(&mut self, path: &Path) {
        self.try_save();
        if let Ok(s) = Session::load(path, self.link.take()) {
            let title = s.title();
            self.screen.set_title(&title);

            let recent = Recent {
                name: title,
                path: path.to_path_buf(),
                thumbnail: "".to_owned(),
            };
            self.store_to_recents(recent.clone());

            self.recent.retain(|recent| recent.path != path);
            self.recent.insert(0, recent);
            self.recent.truncate(8);
            self.session = Some(s);
        }
    }

    fn store_to_recents(&mut self, rom: Recent) {
        self.update_config(Config {
            recents: rom,
            ..self.config.clone()
        });
    }

    fn try_save(&self) -> bool {
        self.session.as_ref().is_none_or(Session::save)
    }
}
