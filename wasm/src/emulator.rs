use gb_core::{bus::MemoryMap, cpu::Cpu};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Emulator {
    cpu: Cpu<MemoryMap>,
}

impl Emulator {
    pub fn new(rom: &[u8]) -> Self {
        let mut bus = MemoryMap::new();
        bus.load(rom, None);

        Self { cpu: Cpu::new(bus) }
    }
}
