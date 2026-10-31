#[derive(Debug, Default)]
pub struct WaveChannel {
    nr30: u8,
    nr31: u8,
    nr32: u8,
    nr33: u8,
    nr34: u8,
    wave_ram: [u8; 16],
    pub channel_on: bool,
}

impl WaveChannel {
    pub fn reset(&self) -> Self {
        Self {
            wave_ram: self.wave_ram,
            ..Default::default()
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF1A => self.nr30 | 0x7F,
            0xFF1B => 0xFF,
            0xFF1C => self.nr32 | 0x9F,
            0xFF1D => 0xFF,
            0xFF1E => self.nr34 | 0xBF,
            0xFF30..=0xFF3F => self.wave_ram[(addr - 0xFF30) as usize],
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0xFF1A => {
                self.nr30 = v;
                if v & 0x80 == 0 {
                    self.channel_on = false;
                }
            }
            0xFF1B => self.nr31 = v,
            0xFF1C => self.nr32 = v,
            0xFF1D => self.nr33 = v,
            0xFF1E => {
                self.nr34 = v;
                if v & 0x80 != 0 {
                    self.channel_on = self.nr30 & 0x80 != 0;
                }
            }
            0xFF30..=0xFF3F => self.wave_ram[(addr - 0xFF30) as usize] = v,

            _ => unreachable!(),
        }
    }
}
