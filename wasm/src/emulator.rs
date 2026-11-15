use gb_core::{bus::MemoryMap, cpu::Cpu, joypad::Button as EmuButton};
use wasm_bindgen::prelude::*;

const FRAME_M_CYCLES: u32 = 17_556;

#[wasm_bindgen]
pub struct Emulator {
    cpu: Cpu<MemoryMap>,
}

#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub fn new(rom: &[u8]) -> Self {
        let mut bus = MemoryMap::new();
        bus.load(rom, None);

        Self { cpu: Cpu::new(bus) }
    }

    pub fn framebuffer(&self) -> Vec<u8> {
        self.cpu.frame_buffer().to_vec()
    }

    pub fn press(&mut self, button: Button) {
        self.cpu.press(EmuButton::from(button))
    }

    pub fn run_frame(&mut self) {
        let mut cycles = 0;
        while cycles < FRAME_M_CYCLES {
            cycles += self.cpu.step();
        }
    }
}

#[wasm_bindgen]
pub enum Button {
    A,
    B,
    Start,
    Select,
    Up,
    Down,
    Left,
    Right,
}

impl From<Button> for EmuButton {
    fn from(btn: Button) -> Self {
        match btn {
            Button::A => EmuButton::A,
            Button::B => EmuButton::B,
            Button::Start => EmuButton::Start,
            Button::Select => EmuButton::Select,
            Button::Up => EmuButton::Up,
            Button::Down => EmuButton::Down,
            Button::Left => EmuButton::Left,
            Button::Right => EmuButton::Right,
        }
    }
}
