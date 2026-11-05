
use std::{path::Path, time::Duration};

use sdl3::EventPump;

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
}

impl App {
    pub fn new(sdl: &sdl3::Sdl) -> Self {
        let video = sdl.video().unwrap();
        let audio = sdl.audio().unwrap();
        Self {
            events: sdl.event_pump().unwrap(),
            screen: Screen::new(&video, "good boy"),
            audio: AudioOutput::new(&audio),
            session: None,
            palettes: builtin_palettes(),
            palette: 0,
            paused: false,
        }
    }

    pub fn run(&mut self) {
        loop {
            while let Some(event) = self.events.poll_event() {
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
            Action::OpenRom(path) => self.open_rom(&path),
            Action::Button(b, down) => {
                if let Some(s) = &mut self.session {
                    s.press(b, down)
                }
            }
        }
        true
    }

    fn frame(&mut self) {
        while self.audio.queued_bytes() > 2 * FRAME_BYTES {
            std::thread::sleep(Duration::from_millis(1));
        }
        let Some(session) = self.session.as_mut() else {
            return self.screen.clear();
        };
        if self.paused {
            std::thread::sleep(Duration::from_millis(16));
            return;
        }
        session.run_frame();
        self.audio.submit(session);
        self.screen.draw(session, &self.palettes[self.palette]);
    }

    pub fn open_rom(&mut self, path: &Path) {
        self.try_save();
        if let Ok(s) = Session::load(path) {
            self.screen.set_title(&s.title());
            self.session = Some(s);
        }
    }

    fn try_save(&self) -> bool {
        self.session.as_ref().is_none_or(Session::save)
    }
}
