use std::{
    fmt,
    fs::{self, File},
    io::{self, BufReader},
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
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub palette: usize,
}

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    Io(io::Error),
    Parse(serde_json::Error),
}

fn base_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap();
    let app_dir = config_dir.join("Good Boy").join("Saves");

    fs::create_dir_all(&app_dir).unwrap();

    app_dir
}

fn save_path(name: &Path) -> PathBuf {
    let base_path = base_path();
    let filename = name.file_name().unwrap();
    let path = base_path.join(filename).with_extension("sav");
    path
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound => write!(f, "Config file does not exist"),
            ConfigError::Io(err) => write!(f, "Failed to open config file: {}", err),
            ConfigError::Parse(err) => write!(f, "Failed to parse config JSON: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn load_config() -> Result<Config, ConfigError> {
    let config_file = base_path().join("config").with_extension("json");

    let file = File::open(&config_file).map_err(|err| {
        if err.kind() == io::ErrorKind::NotFound {
            ConfigError::NotFound
        } else {
            ConfigError::Io(err)
        }
    })?;

    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(ConfigError::Parse)
}
pub fn load_save(name: &Path) -> Option<Vec<u8>> {
    let path = save_path(name);
    fs::read(path).ok()
}

pub fn create_save(data: &[u8], name: &Path) -> bool {
    let path = save_path(name);

    match fs::write(path, data) {
        Ok(_) => true,
        Err(e) => {
            println!("Error: {:?}", e);
            false
        }
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
