mod app;
mod audio;
mod display;
mod input;
mod palette;
mod save;
mod session;

use std::env;
use std::path::Path;

use egui_sdl3::sdl3;

use crate::app::App;

const RESOLUTION_X: u32 = 160;
const RESOLUTION_Y: u32 = 144;
const SCALE: u32 = 5;
const FRAME_M_CYCLES: u32 = 17_556;

fn main() {
    let sdl = sdl3::init().unwrap();
    let mut args = env::args();
    if std::env::args().any(|arg| arg == "--mute") {
        sdl3::hint::set("SDL_AUDIO_DRIVER", "dummy");
    }

    let mut app = App::new(sdl);

    args.next();
    if let Some(path) = args.next() {
        app.open_rom(Path::new(&path));
    }

    app.run();
}
