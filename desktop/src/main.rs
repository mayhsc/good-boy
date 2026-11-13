mod app;
mod audio;
mod display;
mod input;
mod network;
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
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--mute") {
        sdl3::hint::set("SDL_AUDIO_DRIVER", "dummy");
    }

    let port: Option<u16> = args
        .iter()
        .skip_while(|arg| *arg != "--port")
        .nth(1)
        .and_then(|val| Some(val.parse().ok()))
        .unwrap_or(None);

    let mut app = App::new(sdl, port);

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--port" {
            iter.next();
        } else if !arg.starts_with("--") {
            app.open_rom(Path::new(arg));
            break;
        }
    }

    app.run();
}
