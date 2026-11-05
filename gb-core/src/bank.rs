const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

#[derive(Default, Clone, Copy, PartialEq)]
pub(super) enum MBC {
    #[default]
    NONE,
    MBC1,
    MBC3,
    MBC5,
}

pub(super) struct Bank {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mbc: MBC,
    rom_bank_count: usize,
    ram_enabled: bool,
    reg_rom_lo: u8,
    reg_hi: u8,
    reg_ram: u8,
    mode1: bool,
}

impl Default for Bank {
    fn default() -> Self {
        Self {
            rom: Vec::new(),
            ram: Vec::new(),
            mbc: MBC::NONE,
            rom_bank_count: 2,
            ram_enabled: false,
            reg_rom_lo: 1,
            reg_hi: 0,
            reg_ram: 0,
            mode1: false,
        }
    }
}

impl Bank {
    pub fn new() -> Self {
        Bank::default()
    }

    pub fn load(&mut self, data: &[u8]) {
        self.mbc = match data.get(0x147).copied().unwrap_or(0) {
            0x01..=0x03 => MBC::MBC1,
            0x0F..=0x13 => MBC::MBC3,
            0x19..=0x1E => MBC::MBC5,
            _ => MBC::NONE,
        };
        let ram_size = match data.get(0x149).copied().unwrap_or(0) {
            2 => 0x2000,
            3 => 0x8000,
            4 => 0x20000,
            5 => 0x10000,
            _ => 0,
        };
        self.ram = vec![0xFF; ram_size];
        self.rom = data.to_vec();
        self.rom_bank_count = (self.rom.len() / ROM_BANK_SIZE).max(2);
        self.reg_rom_lo = 1;
    }

    pub fn load_save(&mut self, data: &[u8]) {
        let len = self.ram.len().min(data.len());
        self.ram[..len].copy_from_slice(&data[..len]);
    }

    fn rom_bank_0(&self) -> usize {
        if self.mbc == MBC::MBC1 && self.mode1 {
            ((self.reg_hi as usize) << 5) % self.rom_bank_count
        } else {
            0
        }
    }

    fn rom_bank_n(&self) -> usize {
        let bank = match self.mbc {
            MBC::NONE => 1,
            MBC::MBC1 => {
                let lo = if self.reg_rom_lo & 0x1F == 0 {
                    1
                } else {
                    self.reg_rom_lo & 0x1F
                };
                (self.reg_hi as usize) << 5 | lo as usize
            }
            MBC::MBC3 => {
                if self.reg_rom_lo & 0x7F == 0 {
                    1
                } else {
                    (self.reg_rom_lo & 0x7F) as usize
                }
            }
            MBC::MBC5 => (self.reg_hi as usize & 1) << 8 | self.reg_rom_lo as usize,
        };
        bank % self.rom_bank_count
    }

    fn ram_bank(&self) -> usize {
        match self.mbc {
            MBC::NONE => 0,
            MBC::MBC1 => {
                if self.mode1 {
                    self.reg_hi as usize
                } else {
                    0
                }
            }
            MBC::MBC3 => (self.reg_ram & 0x03) as usize,
            MBC::MBC5 => (self.reg_ram & 0x0F) as usize,
        }
    }

    fn ram_index(&self, addr: u16) -> Option<usize> {
        if self.ram.is_empty() {
            return None;
        }
        let i = self.ram_bank() * RAM_BANK_SIZE + (addr - 0xA000) as usize;
        Some(i % self.ram.len())
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self
                .rom
                .get(self.rom_bank_0() * ROM_BANK_SIZE + addr as usize)
                .copied()
                .unwrap_or(0xFF),
            0x4000..=0x7FFF => self
                .rom
                .get(self.rom_bank_n() * ROM_BANK_SIZE + (addr as usize - 0x4000))
                .copied()
                .unwrap_or(0xFF),
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return 0xFF;
                }
                if self.mbc == MBC::MBC3 && (0x08..=0x0C).contains(&self.reg_ram) {}
                self.ram_index(addr).map_or(0xFF, |i| self.ram[i])
            }
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0x0000..=0x7FFF => match self.mbc {
                MBC::NONE => {}
                MBC::MBC1 => self.mbc1_write(addr, v),
                MBC::MBC3 => self.mbc3_write(addr, v),
                MBC::MBC5 => self.mbc5_write(addr, v),
            },
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return;
                }
                if self.mbc == MBC::MBC3 && (0x08..=0x0C).contains(&self.reg_ram) {
                    // self.rtc.write(self.reg_ram, v);
                    return;
                }
                if let Some(i) = self.ram_index(addr) {
                    self.ram[i] = v;
                }
            }
            _ => unreachable!(),
        }
    }

    fn mbc1_write(&mut self, addr: u16, v: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = v & 0x0F == 0x0A,
            0x2000..=0x3FFF => self.reg_rom_lo = v & 0x1F,
            0x4000..=0x5FFF => self.reg_hi = v & 0x03,
            _ => self.mode1 = v & 1 != 0,
        }
    }

    fn mbc3_write(&mut self, addr: u16, v: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = v & 0x0F == 0x0A,
            0x2000..=0x3FFF => self.reg_rom_lo = v & 0x7F,
            0x4000..=0x5FFF => self.reg_ram = v,
            _ => todo!(),
        }
    }

    fn mbc5_write(&mut self, addr: u16, v: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = v == 0x0A,
            0x2000..=0x2FFF => self.reg_rom_lo = v,
            0x3000..=0x3FFF => self.reg_hi = v & 1,
            0x4000..=0x5FFF => self.reg_ram = v & 0x0F,
            _ => {}
        }
    }

    pub fn dump_ext_ram(&self) -> &[u8] {
        &self.ram
    }
}
