use sdl3::{
    messagebox::{
        ButtonData, ClickedButton, MessageBoxButtonFlag, MessageBoxFlag, show_message_box,
    },
    render::Canvas,
    video::Window,
};

pub fn create_save(data: &[u8]) -> bool {
    if let Some(config_dir) = dirs::config_dir() {
        true
    } else {
        false
    }
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
