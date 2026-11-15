use gb_core::{bus::MemoryMap, cpu::Cpu, joypad::Button as EmuButton};
use wasm_bindgen::prelude::*;

const FRAME_M_CYCLES: u32 = 17_556;
const WIDTH: usize = 160;
const HEIGHT: usize = 144;

#[wasm_bindgen]
pub struct Emulator {
    cpu: Cpu<MemoryMap>,
    rgba: Vec<u8>,
}

#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub fn new(rom: &[u8], save: Option<Vec<u8>>) -> Self {
        let mut bus = MemoryMap::new();
        bus.load(rom, save);

        Self {
            cpu: Cpu::new(bus),
            rgba: vec![0; WIDTH * HEIGHT * 4],
        }
    }

    pub fn run_frame(&mut self) {
        let mut cycles = 0;
        while cycles < FRAME_M_CYCLES {
            cycles += self.cpu.step();
        }
    }

    pub fn framebuffer(&self) -> Vec<u8> {
        self.cpu.frame_buffer().to_vec()
    }

    pub fn rgba(&mut self, palette: &[u8]) -> Vec<u8> {
        assert_eq!(palette.len(), 12, "palette must be 4 RGB triples");

        let shades = self.cpu.frame_buffer();
        let drawn = self.cpu.drawn();

        for (i, (&shade, &is_drawn)) in shades.iter().zip(drawn).enumerate() {
            let c = if is_drawn { (shade as usize) & 3 } else { 0 };
            let px = &mut self.rgba[i * 4..i * 4 + 4];
            px[..3].copy_from_slice(&palette[c * 3..c * 3 + 3]);
            px[3] = 255;
        }

        self.rgba.clone()
    }

    pub fn save_data(&self) -> Option<Vec<u8>> {
        self.cpu.dump_ext_ram().map(<[u8]>::to_vec)
    }

    pub fn press(&mut self, button: Button) {
        self.cpu.press(EmuButton::from(button))
    }

    pub fn release(&mut self, button: Button) {
        self.cpu.release(EmuButton::from(button))
    }

    pub fn name(&self) -> String {
        self.cpu.name().clone()
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
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
