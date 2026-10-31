#[derive(Debug, Default)]
pub(super) struct NoiseChannel {
    nr41: u8,
    nr42: u8,
    nr43: u8,
    nr44: u8,
    pub channel_on: bool,
}

impl NoiseChannel {
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF20 => 0xFF,
            0xFF21 => self.nr42,
            0xFF22 => self.nr43,
            0xFF23 => self.nr44 | 0xBF,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0xFF20 => self.nr41 = v,
            0xFF21 => {
                self.nr42 = v;
                if v & 0xF8 == 0 {
                    self.channel_on = false;
                }
            }
            0xFF22 => self.nr43 = v,
            0xFF23 => {
                self.nr44 = v;
                if v & 0x80 != 0 {
                    self.channel_on = self.nr42 & 0xF8 != 0;
                }
            }
            _ => unreachable!(),
        }
    }
}
