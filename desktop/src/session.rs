use std::{
    io,
    path::{Path, PathBuf},
};

use gb_core::{bus::MemoryMap, cpu::Cpu, joypad::Button};

use crate::{
    FRAME_M_CYCLES,
    save::{create_save, load_save},
};

pub struct Session {
    cpu: Cpu<MemoryMap>,
    rom_path: PathBuf,
}

impl Session {
    pub fn load(path: &Path) -> io::Result<Self> {
        let rom = std::fs::read(path)?;
        let mut bus = MemoryMap::new();
        bus.load(&rom, load_save(path));
        Ok(Self {
            cpu: Cpu::new(bus),
            rom_path: path.to_owned(),
        })
    }

    pub fn title(&self) -> String {
        self.rom_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_owned()
    }

    pub fn run_frame(&mut self) {
        let mut cycles = 0;
        while cycles < FRAME_M_CYCLES {
            cycles += self.cpu.step();
        }
    }

    pub fn press(&mut self, b: Button, down: bool) {
        if down {
            self.cpu.press(b);
        } else {
            self.cpu.release(b);
        }
    }

    pub fn dump_vram(&self) {
        self.cpu.dump_vram();
    }

    pub fn frame_buffer(&self) -> &[u8] {
        self.cpu.frame_buffer()
    }

    pub fn drawn(&self) -> &[bool] {
        self.cpu.drawn()
    }

    pub fn swap_samples(&mut self, samples: &mut Vec<f32>) {
        self.cpu.swap_samples(samples);
    }

    pub fn save(&self) -> bool {
        match self.cpu.dump_ext_ram() {
            Some(data) => create_save(data, &self.rom_path),
            None => true,
        }
    }
}
