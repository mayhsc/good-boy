const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

#[derive(Default)]
enum MBC {
    #[default]
    NONE,
    MBC1,
}

pub(super) struct Bank {
    rom: Vec<u8>,
    extram: [u8; 0x2000],
    rom_bank: u8,
    ram_bank: u8,
    mbc: MBC,
    ram_enabled: bool,
    rom_mode: bool,
}

impl Default for Bank {
    fn default() -> Self {
        Self {
            rom: Vec::new(),
            extram: [0; 0x2000],
            rom_bank: 1,
            ram_bank: 0,
            mbc: MBC::NONE,
            ram_enabled: false,
            rom_mode: true,
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
            0xA000..=0xBFFF => match self.mbc {
                MBC::NONE | MBC::MBC1 => {
                    let rel_addr = (addr - 0xA000) as usize;
                    let bank_addr = (self.ram_bank as usize) * RAM_BANK_SIZE + rel_addr;
                    self.extram[bank_addr]
                }
            },
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => match self.mbc {
                MBC::NONE => {}
                MBC::MBC1 => self.mbc1_write_rom(addr, value),
            },
            0xA000..=0xBFFF => self.extram[(addr - 0xA000) as usize] = value,
            _ => unreachable!(),
        }
    }

    fn mbc1_write_rom(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = value == 0x0A;
            }
            0x2000..=0x3FFF => {
                let bank = value & 0x1F;
                match bank {
                    0x00 | 0x20 | 0x40 | 0x60 => {
                        self.rom_bank = bank + 1;
                    }
                    _ => {
                        self.rom_bank = bank;
                    }
                }
            }
            0x4000..=0x5FFF => {
                let bits = value & 0b11;

                if self.rom_mode {
                    self.rom_bank |= bits << 5;
                } else {
                    self.ram_bank = bits;
                }
            }
            0x6000..=0x7FFF => {
                self.rom_mode = value == 0;
            }
            _ => unreachable!(),
        }
    }

    pub fn load(&mut self, data: &[u8]) {
        self.rom.extend_from_slice(data);
    }
}
