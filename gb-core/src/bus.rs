use crate::{
    IF_ADDR,
    apu::Apu,
    bank::Bank,
    joypad::{Button, Joypad},
    ppu::Ppu,
    timer::Timer,
};

pub trait Bus {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);

    fn peek(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn tick_timers(&mut self, t_cycles: u32) {
        let _ = t_cycles;
    }
}

pub struct MemoryMap {
    bank: Bank,
    wram0: [u8; 0x1000],
    wram1_7: [[u8; 0x1000]; 7],

    io_regs: [u8; 0x0080],
    hram: [u8; 0x007F],
    ie: u8,

    timer: Timer,
    ppu: Ppu,
    apu: Apu,
    joypad: Joypad,

    enable_saves: bool,
}

impl Default for MemoryMap {
    fn default() -> Self {
        Self {
            bank: Bank::new(),

            wram0: [0; 0x1000],
            wram1_7: [[0; 0x1000]; 7],

            io_regs: [0; 0x0080],
            hram: [0; 0x007F],
            ie: 0,

            timer: Timer::new(),
            ppu: Ppu::new(),
            apu: Apu::new(),
            joypad: Joypad::new(),

            enable_saves: false,
        }
    }
}

impl Bus for MemoryMap {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.bank.read(addr),
            0xC000..=0xCFFF => self.wram0[(addr - 0xC000) as usize],
            0xD000..=0xDFFF => {
                let bank = 0;
                self.wram1_7[bank][(addr - 0xD000) as usize]
            }
            0xE000..=0xFDFF => self.read(addr - 0x2000),
            0xFEA0..=0xFEFF => 0xFF,
            0xFF04..=0xFF07 => self.timer.get_timer(addr),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.read(addr),
            0xFF00 => self.joypad.read(),
            0xFF10..=0xFF3F => self.apu.read(addr),
            0xFF00..=0xFF7F => self.io_regs[(addr - 0xFF00) as usize],
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF => self.ie,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.bank.write(addr, value),
            0xC000..=0xCFFF => self.wram0[(addr - 0xC000) as usize] = value,
            0xD000..=0xDFFF => {
                let bank = 0;
                self.wram1_7[bank][(addr - 0xD000) as usize] = value;
            }
            0xE000..=0xFDFF => self.write(addr - 0x2000, value),
            0xFEA0..=0xFEFF => {}
            0xFF04..=0xFF07 => {
                if self.timer.set_timer(addr, value).apu_tick {
                    self.apu.tick_frame_sequencer();
                }
            }
            0xFF46 => {
                let base = (value as u16) << 8;
                for i in 0..0xA0u16 {
                    let b = self.read(base + i);
                    self.ppu.oam_write_raw(i as usize, b);
                }
            }
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.write(addr, value),
            0xFF00 => self.joypad.write(value),
            0xFF10..=0xFF3F => self.apu.write(addr, value),
            0xFF00..=0xFF7F => self.io_regs[(addr - 0xFF00) as usize] = value,
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            0xFFFF => self.ie = value,
        }
    }

    fn tick_timers(&mut self, t_cycles: u32) {
        for _ in 0..t_cycles {
            let ev = self.timer.tick();
            self.apu.tick_frequency_timer();
            if ev.interrupt {
                let f = self.read(IF_ADDR) | (1 << 2);
                self.write(IF_ADDR, f);
            }
            if ev.apu_tick {
                self.apu.tick_frame_sequencer();
            }
            let interrupts = self.ppu.tick();
            if interrupts != 0 {
                let current_if = self.read(IF_ADDR);
                self.write(IF_ADDR, current_if | interrupts);
            }
        }
    }
}

impl MemoryMap {
    pub fn new() -> Self {
        MemoryMap::default()
    }

    pub fn load(&mut self, data: &[u8], save: Option<Vec<u8>>) {
        self.bank.load(data);
        self.enable_saves = matches!(
            data.get(0x147).copied().unwrap_or(0),
            0x03 | 0x06 | 0x09 | 0x0D | 0x0F | 0x10 | 0x13 | 0x1B | 0x1E
        );
        if let Some(save_data) = save {
            self.bank.load_save(&save_data);
        }
    }
    pub fn framebuffer(&self) -> &[u8] {
        &self.ppu.framebuffer()
    }

    pub fn drawn(&self) -> &[bool] {
        &self.ppu.drawn
    }

    pub fn dump_vram(&self) {
        self.ppu.dump_vram().unwrap();
        self.ppu.dump_oam().unwrap();
    }

    pub fn press(&mut self, btn: Button) {
        if self.joypad.press(btn) {
            let _if = self.read(IF_ADDR) | 1 << 4;
            self.write(IF_ADDR, _if);
        }
    }

    pub fn release(&mut self, btn: Button) {
        self.joypad.release(btn);
    }

    pub fn swap_samples(&mut self, out: &mut Vec<f32>) {
        self.apu.swap_samples(out);
    }

    pub fn dump_ext_ram(&self) -> Option<&[u8]> {
        if self.enable_saves {
            Some(self.bank.dump_ext_ram())
        } else {
            None
        }
    }
}
