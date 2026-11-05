mod app;
mod audio;
mod display;
mod input;
mod palette;
mod save;
mod session;

use std::env;
use std::path::Path;

use crate::app::App;

const RESOLUTION_X: u32 = 160;
const RESOLUTION_Y: u32 = 144;
const SCALE: u32 = 1;
const FRAME_M_CYCLES: u32 = 17_556;

fn main() {
    let sdl = sdl3::init().unwrap();
    let mut app = App::new(&sdl);
    if let Some(path) = env::args().nth(1) {
        app.open_rom(Path::new(&path));
    }
    app.run();
}
