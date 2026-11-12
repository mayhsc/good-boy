use std::path::PathBuf;

use egui_sdl3::sdl3;
use gb_core::joypad::Button;
use sdl3::{
    event::Event,
    keyboard::{Keycode, Mod},
};

pub enum Action {
    Quit,
    TogglePause,
    NextPalette,
    DumpVram,
    OpenSettings,
    OpenRom(PathBuf),
    Button(Button, bool),
    StartHost,
}

pub fn translate(event: Event) -> Option<Action> {
    match event {
        Event::Quit { .. } => Some(Action::Quit),
        Event::DropFile { filename, .. } => Some(Action::OpenRom(filename.into())),
        Event::KeyDown {
            keycode: Some(k),
            keymod,
            repeat,
            ..
        } => match k {
            Keycode::H => Some(Action::StartHost),
            Keycode::Escape => Some(Action::Quit),
            Keycode::P if !repeat => Some(Action::TogglePause),
            Keycode::C if !repeat => Some(Action::NextPalette),
            Keycode::K if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                Some(Action::DumpVram)
            }
            Keycode::Comma if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => {
                Some(Action::OpenSettings)
            }
            _ if repeat => None,
            _ => key_to_button(k).map(|b| Action::Button(b, true)),
        },
        Event::KeyUp {
            keycode: Some(k), ..
        } => key_to_button(k).map(|b| Action::Button(b, false)),
        _ => None,
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
