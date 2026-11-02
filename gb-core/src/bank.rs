const ROM_BANK_SIZE: usize = 0x4000;

pub(super) struct Bank {
    rom: Vec<u8>,
    extram: [u8; 0x2000],
    rom_bank: u8,
}

impl Default for Bank {
    fn default() -> Self {
        Self {
            rom: Vec::new(),
            extram: [0; 0x2000],
            rom_bank: 1,
        }
    }
}

impl Bank {
    pub fn new() -> Self {
        Bank::default()
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                if addr < ROM_BANK_SIZE as u16 {
                    self.rom[addr as usize]
                } else {
                    let rel_addr = (addr as usize) - ROM_BANK_SIZE;
                    let bank_addr = (self.rom_bank as usize) * ROM_BANK_SIZE + rel_addr;
                    self.rom[bank_addr]
                }
            }
            0xA000..=0xBFFF => self.extram[(addr - 0xA000) as usize],
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x3FFF => {}
            0x4000..=0x7FFF => {}
            0xA000..=0xBFFF => self.extram[(addr - 0xA000) as usize] = value,
            _ => unreachable!(),
        }
    }

    pub fn load(&mut self, data: &[u8]) {
        self.rom.extend_from_slice(data);
    }
}
