pub(super) struct Bank {
    rombank00: [u8; 0x4000],
    rombank01_nn: [u8; 0x4000],

    extram: [u8; 0x2000],
}

impl Default for Bank {
    fn default() -> Self {
        Self {
            rombank00: [0; 0x4000],
            rombank01_nn: [0; 0x4000],

            extram: [0; 0x2000],
        }
    }
}

impl Bank {
    pub fn new() -> Self {
        Bank::default()
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rombank00[addr as usize],
            0x4000..=0x7FFF => self.rombank01_nn[(addr - 0x4000) as usize],
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
        let bank00_len = data.len().min(0x4000);
        self.rombank00[..bank00_len].copy_from_slice(&data[..bank00_len]);

        if data.len() > 0x4000 {
            let bank01_len = (data.len() - 0x4000).min(0x4000);
            self.rombank01_nn[..bank01_len].copy_from_slice(&data[0x4000..0x4000 + bank01_len]);
        }

        if data.len() > 0x8000 {
            todo!();
        }
    }
}
