use std::io;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::{path::Path, time::Duration};

use egui_sdl3::sdl3::{EventPump, Sdl};
use gb_core::serial::link::TcpLink;

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
    recent: Vec<std::path::PathBuf>,
    settings_open: bool,
    pending: Option<Receiver<io::Result<TcpLink>>>,
    network_status: String,
}

impl App {
    pub fn new(sdl: Sdl, port: Option<u16>) -> Self {
        let video = sdl.video().unwrap();
        let audio = sdl.audio().unwrap();
        let pending = match port {
            Some(p) => Some(start_join(format!("127.0.0.1:{p}").to_owned())),
            None => None,
        };

        Self {
            events: sdl.event_pump().unwrap(),
            screen: Screen::new(&video, "Good Boy"),
            audio: AudioOutput::new(&audio),
            session: None,
            palettes: builtin_palettes(),
            palette: 0,
            paused: false,
            recent: Vec::new(),
            settings_open: false,
            pending: pending,
            network_status: "".to_owned(),
        }
    }

    pub fn run(&mut self) {
        loop {
            while let Some(event) = self.events.poll_event() {
                self.screen.on_event(&event);
                if let Some(action) = input::translate(event) {
                    if !self.handle(action) {
                        return;
                    } // false = quit
                }
            }
            self.frame();
        }
    }

    fn handle(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return !(self.try_save() || self.screen.confirm_quit_without_saving()),
            Action::TogglePause => self.paused = !self.paused,
            Action::NextPalette => self.palette = (self.palette + 1) % self.palettes.len(),
            Action::DumpVram => {
                if let Some(s) = &self.session {
                    s.dump_vram()
                }
            }
            Action::OpenSettings => self.settings_open = true,
            Action::OpenRom(path) => self.open_rom(&path),
            Action::Button(b, down) => {
                if let Some(s) = &mut self.session {
                    s.press(b, down)
                }
            }
            Action::StartHost => {
                match &mut self.pending {
                    Some(_) => {
                        println!("Host already active");
                    }
                    None => {
                        // Error handling
                        self.pending = Some(start_host(7777));
                    }
                }
            }
        }
        true
    }

    fn frame(&mut self) {
        self.poll_connection();

        while self.audio.queued_bytes() > 2 * FRAME_BYTES {
            std::thread::sleep(Duration::from_millis(1));
        }

        // No game loaded: show the home screen.
        if self.session.is_none() {
            let action = {
                let view = SettingsView {
                    palettes: &self.palettes,
                    selected: self.palette,
                };
                self.screen
                    .home(&self.recent, &mut self.settings_open, &view)
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
            let view = SettingsView {
                palettes: &self.palettes,
                selected: self.palette,
            };
            self.screen.draw(
                self.session.as_ref().unwrap(),
                &self.palettes[self.palette],
                &mut self.settings_open,
                &view,
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
            UiAction::ChoosePalette(index) => self.palette = index,
            UiAction::ChooseDefaultDirectory => {
                println!("Choose default ROM directory requested")
            }
            UiAction::_OpenSettings | UiAction::None => {}
        }
    }

    fn poll_connection(&mut self) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(Ok(link)) => {
                    self.attach_link(link);
                    self.pending = None;
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
        }
    }

    pub fn open_rom(&mut self, path: &Path) {
        self.try_save();
        if let Ok(s) = Session::load(path) {
            self.screen.set_title(&s.title());
            self.recent.retain(|recent| recent != path);
            self.recent.insert(0, path.to_owned());
            self.recent.truncate(8);
            self.session = Some(s);
        }
    }

    fn try_save(&self) -> bool {
        self.session.as_ref().is_none_or(Session::save)
    }
}
