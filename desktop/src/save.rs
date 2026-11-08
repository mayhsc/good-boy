use std::{
    fs,
    path::{Path, PathBuf},
};

use egui_sdl3::sdl3;
use sdl3::{
    messagebox::{
        ButtonData, ClickedButton, MessageBoxButtonFlag, MessageBoxFlag, show_message_box,
    },
    render::Canvas,
    video::Window,
};

fn save_path(name: &Path) -> PathBuf {
    name.with_extension("sav")
}

pub fn load_save(name: &Path) -> Option<Vec<u8>> {
    let path = save_path(name);
    fs::read(path).ok()
}

pub fn create_save(data: &[u8], name: &Path) -> bool {
    let path = save_path(name);
    fs::write(path, data).is_ok()
}

pub fn show_save_error_dialog(canvas: &Canvas<Window>) -> bool {
    let buttons = [
        ButtonData {
            flags: MessageBoxButtonFlag::RETURNKEY_DEFAULT,
            button_id: 1,
            text: "Quit",
        },
        ButtonData {
            flags: MessageBoxButtonFlag::ESCAPEKEY_DEFAULT,
            button_id: 2,
            text: "Cancel",
        },
    ];

    let result = show_message_box(
        MessageBoxFlag::ERROR,
        &buttons,
        "Emulator Save Error",
        "Could not save. Would you like to close the window regardless?",
        canvas.window(),
        None,
    );

    match result {
        Ok(ClickedButton::CustomButton(button)) => match button.button_id {
            1 => true,
            2 => false,
            _ => unreachable!(),
        },
        Err(_) => false,
        _ => false,
    }
}
