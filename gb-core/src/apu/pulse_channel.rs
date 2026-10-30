#[derive(Debug, Default)]
pub(super) struct PulseChannel {
    nrx0: u8,
    nrx1: u8,
    nrx2: u8,
    nrx3: u8,
    nrx4: u8,
    pub channel_on: bool,
    frequency_timer: u16,
}

impl PulseChannel {
    pub fn new(nrx1: u8, nrx2: u8, channel_on: bool) -> Self {
        PulseChannel {
            nrx1: nrx1,
            nrx2: nrx2,
            channel_on: channel_on,
            ..Default::default()
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0 => self.nrx0 | 0x80,
            1 => self.nrx1 | 0x3F,
            2 => self.nrx2,
            3 => 0xFF,
            4 => self.nrx4 | 0xBF,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0 => self.nrx0 = v,
            1 => self.nrx1 = v,
            2 => {
                self.nrx2 = v;
                if v & 0xF8 == 0 {
                    self.channel_on = false;
                }
            }
            3 => self.nrx3 = v,
            4 => {
                self.nrx4 = v;
                if v & 0x80 != 0 {
                    self.channel_on = self.nrx2 & 0xF8 != 0;
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self) {
        self.frequency_timer -= 1;
        if self.frequency_timer == 0 {
            self.frequency_timer = (2048 - self.frequency()) * 4;
        }
    }

    fn frequency(&self) -> u16 {
        (self.nrx4 as u16 & 0x6F) << 8 | self.nrx3 as u16
    }
}
