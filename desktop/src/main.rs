mod audio;

use std::fs::File;
use std::io::Read;
use std::{collections::VecDeque, env};

use gb_core::bus::MemoryMap;
use gb_core::cpu::Cpu;
use gb_core::joypad::Button;
use sdl3::{
    VideoSubsystem,
    audio::{AudioFormat, AudioSpec},
    event::Event,
    keyboard::{Keycode, Mod},
    pixels::{Color, PixelFormat},
    render::{Canvas, Texture},
    sys::{render::SDL_SetTextureScaleMode, surface::SDL_SCALEMODE_NEAREST},
    video::Window,
};

use crate::audio::EmulatorAudio;

const RESOLUTION_X: u32 = 160;
const RESOLUTION_Y: u32 = 144;
const SCALE: u32 = 5;
const FRAME_M_CYCLES: u32 = 17_556;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run path/to/game");
        return;
    }

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
    let mut canvas = create_canvas(&video_subsystem);

    let mut rom = File::open(&args[1]).expect("Unable to open file");
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).unwrap();

    let mut bus = MemoryMap::new();
    bus.load(&buffer);
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

    'gameloop: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'gameloop;
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

        to_rgba(cpu.frame_buffer(), &mut rgba, cpu.drawn());

        draw_screen(&mut canvas, &mut texture, &rgba);

        canvas.present();
    }
}

fn create_canvas(video_subsystem: &VideoSubsystem) -> Canvas<Window> {
    const WINDOW_WIDTH: u32 = (RESOLUTION_X as u32) * SCALE;
    const WINDOW_HEIGHT: u32 = (RESOLUTION_Y as u32) * SCALE;

    let window = video_subsystem
        .window("Good Boy", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .opengl()
        // .resizable()
        .build()
        .unwrap();

    let canvas = window.into_canvas();
    // canvas.set_logical_size(
    //     RESOLUTION_X,
    //     RESOLUTION_Y,
    //     SDL_RendererLogicalPresentation::INTEGER_SCALE,
    // );

    canvas
}

fn draw_screen(canvas: &mut Canvas<Window>, texture: &mut Texture<'_>, rgba: &[u8]) {
    assert_eq!(rgba.len(), (RESOLUTION_X * RESOLUTION_Y * 4) as usize);
    texture
        .update(None, rgba, (RESOLUTION_X * 4) as usize)
        .unwrap();
    canvas.clear();
    canvas.copy(texture, None, None).unwrap();
}

fn to_rgba(shades: &[u8], out: &mut [u8], drawn: &[bool]) {
    // const PALETTE: [[u8; 4]; 4] = [
    //     [0xC8, 0xCC, 0xC8, 0xFF],
    //     [0xA0, 0xA4, 0xA0, 0xFF],
    //     [0x68, 0x6C, 0x68, 0xFF],
    //     [0x30, 0x34, 0x30, 0xFF],
    // ];
    const PALETTE: [[u8; 4]; 4] = [
        [0xFF, 0xF6, 0xD3, 0xFF],
        [0xF9, 0xA8, 0x75, 0xFF],
        [0xEB, 0xCB, 0x6F, 0xFF],
        [0x7C, 0x3F, 0x58, 0xFF],
    ];

    for (i, (&shade, &is_drawn)) in shades.iter().zip(drawn).enumerate() {
        let rgba = if is_drawn {
            &PALETTE[(shade & 3) as usize]
        } else {
            &[0x00, 0x00, 0x00, 0xFF]
        };

        out[i * 4..i * 4 + 4].copy_from_slice(rgba);
    }
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
