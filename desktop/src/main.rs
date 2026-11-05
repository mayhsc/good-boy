mod audio;
mod display;
mod palette;
mod save;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::{collections::VecDeque, env};

use gb_core::bus::MemoryMap;
use gb_core::cpu::Cpu;
use gb_core::joypad::Button;
use sdl3::{
    audio::{AudioFormat, AudioSpec},
    event::Event,
    keyboard::{Keycode, Mod},
    pixels::{Color, PixelFormat},
    sys::{render::SDL_SetTextureScaleMode, surface::SDL_SCALEMODE_NEAREST},
};

use crate::audio::EmulatorAudio;
use crate::display::{create_canvas, draw_screen, to_rgba};
use crate::palette::builtin_palettes;
use crate::save::{create_save, load_save, show_save_error_dialog};

const RESOLUTION_X: u32 = 160;
const RESOLUTION_Y: u32 = 144;
const SCALE: u32 = 1;
const FRAME_M_CYCLES: u32 = 17_556;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run path/to/game");
        return;
    }
    let mut rom = File::open(&args[1]).expect("Unable to open file");
    let name = get_game_name(&args[1]);
    let rom_path = Path::new(&args[1]);

    let sdl_context = sdl3::init().unwrap();

    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut samples: Vec<f32> = Vec::with_capacity(2048);

    let audio_subsystem = sdl_context.audio().unwrap();

    let spec = AudioSpec::new(Some(44100), Some(2), Some(AudioFormat::f32_sys()));

    let device = audio_subsystem
        .open_playback_stream(
            &spec,
            EmulatorAudio {
                samples: VecDeque::new(),
            },
        )
        .unwrap();

    device.resume().unwrap();
    let mut canvas = create_canvas(&video_subsystem, &name);

    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).unwrap();

    let mut bus = MemoryMap::new();

    bus.load(&buffer, load_save(&rom_path));
    let mut cpu = Cpu::new(bus);

    let mut paused = false;

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormat::RGBA32, RESOLUTION_X, RESOLUTION_Y)
        .unwrap();
    unsafe {
        SDL_SetTextureScaleMode(texture.raw(), SDL_SCALEMODE_NEAREST);
        sdl3::sys::render::SDL_SetRenderVSync(texture_creator.raw(), 1);
    }

    let mut rgba = vec![0u8; (RESOLUTION_X * RESOLUTION_Y * 4) as usize];
    let mut selected_palette = 0;
    let palettes = builtin_palettes();

    'gameloop: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    let success = match cpu.dump_ext_ram() {
                        Some(data) => create_save(data, &rom_path),
                        None => true,
                    };
                    if success || show_save_error_dialog(&canvas) {
                        break 'gameloop;
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::K),
                    keymod,
                    ..
                } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                    cpu.dump_vram();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    ..
                } => {
                    paused = !paused;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::C),
                    repeat: false,
                    ..
                } => selected_palette = (selected_palette + 1) % palettes.len(),

                Event::KeyDown {
                    keycode,
                    repeat: false,
                    ..
                } => handle_key_event(keycode, true, &mut cpu),

                Event::KeyUp { keycode, .. } => {
                    handle_key_event(keycode, false, &mut cpu);
                }
                _ => {}
            }
        }

        let mut cycles = 0;

        if paused {
            std::thread::sleep(std::time::Duration::from_millis(16));
            continue;
        }
        while cycles < FRAME_M_CYCLES {
            cycles += cpu.step();
            if paused {
                break;
            }
        }

        cpu.swap_samples(&mut samples);
        device.put_data_f32(&samples).unwrap();
        samples.clear();

        to_rgba(
            cpu.frame_buffer(),
            &mut rgba,
            cpu.drawn(),
            &palettes[selected_palette],
        );

        draw_screen(&mut canvas, &mut texture, &rgba);

        canvas.present();
    }
}

fn get_game_name(rom_path: &String) -> String {
    let path = Path::new(rom_path);

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    stem.to_owned()
}

fn handle_key_event(keycode: Option<Keycode>, pressed: bool, joypad: &mut Cpu<MemoryMap>) {
    if let Some(key) = keycode {
        if let Some(button) = key_to_button(key) {
            if pressed {
                joypad.press(button);
            } else {
                joypad.release(button);
            }
        }
    }
}

fn key_to_button(key: Keycode) -> Option<Button> {
    match key {
        Keycode::Right => Some(Button::Right),
        Keycode::Left => Some(Button::Left),
        Keycode::Up => Some(Button::Up),
        Keycode::Down => Some(Button::Down),
        Keycode::Z => Some(Button::A),
        Keycode::X => Some(Button::B),
        Keycode::Return => Some(Button::Start),
        Keycode::Backspace => Some(Button::Select),
        _ => None,
    }
}
