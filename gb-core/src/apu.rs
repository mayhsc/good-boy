pub(crate) struct Apu {
    regs: [u8; 0x17],      // 0xFF10..=0xFF26
    wave_ram: [u8; 16],    // 0xFF30..=0xFF3F
    power: bool,           // NR52 bit 7
    channel_on: [bool; 4], // NR52 bits 0-3 (read-only status)
    counter: u8,
}

// Bits that always read back as 1
const READ_MASK: [u8; 0x17] = [
    0x80, 0x3F, 0x00, 0xFF, 0xBF, // NR10-NR14
    0xFF, 0x3F, 0x00, 0xFF, 0xBF, // unused 0xFF15, NR21-NR24
    0x7F, 0xFF, 0x9F, 0xFF, 0xBF, // NR30-NR34
    0xFF, 0xFF, 0x00, 0x00, 0xBF, // unused 0xFF1F, NR41-NR44
    0x00, 0x00, 0x70, // NR50, NR51, NR52 (status bits added on read)
];

impl Apu {
    pub fn new() -> Self {
        let mut regs = [0u8; 0x17];
        regs[0x00] = 0x80;
        regs[0x01] = 0xBF;
        regs[0x02] = 0xF3;
        regs[0x03] = 0xFF;
        regs[0x04] = 0xBF;
        regs[0x06] = 0x3F;
        regs[0x07] = 0x00;
        regs[0x08] = 0xFF;
        regs[0x09] = 0xBF;
        regs[0x0A] = 0x7F;
        regs[0x0B] = 0xFF;
        regs[0x0C] = 0x9F;
        regs[0x0D] = 0xFF;
        regs[0x0E] = 0xBF;
        regs[0x10] = 0xFF;
        regs[0x11] = 0x00;
        regs[0x12] = 0x00;
        regs[0x13] = 0xBF;
        regs[0x14] = 0x77;
        regs[0x15] = 0xF3;
        Self {
            regs,
            wave_ram: [0; 16],
            power: true,
            channel_on: [true, false, false, false],
            counter: 0,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF10..=0xFF25 => {
                let i = (addr - 0xFF10) as usize;
                self.regs[i] | READ_MASK[i]
            }
            0xFF26 => {
                let status = self
                    .channel_on
                    .iter()
                    .enumerate()
                    .fold(0u8, |acc, (i, &on)| acc | ((on as u8) << i));
                ((self.power as u8) << 7) | 0x70 | status
            }
            0xFF27..=0xFF2F => 0xFF,
            0xFF30..=0xFF3F => self.wave_ram[(addr - 0xFF30) as usize],
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF26 => {
                let on = value & 0x80 != 0;
                if self.power && !on {
                    self.regs[..0x15].fill(0);
                    self.channel_on = [false; 4];
                } else if !self.power && on {
                }
                self.power = on;
            }
            0xFF30..=0xFF3F => {
                self.wave_ram[(addr - 0xFF30) as usize] = value;
            }
            0xFF10..=0xFF25 if self.power => {
                self.regs[(addr - 0xFF10) as usize] = value;
                self.on_register_write(addr, value);
            }
            _ => {}
        }
    }

    fn on_register_write(&mut self, addr: u16, value: u8) {
        let trigger = value & 0x80 != 0;
        match addr {
            0xFF14 if trigger => self.channel_on[0] = self.regs[0x02] & 0xF8 != 0,
            0xFF19 if trigger => self.channel_on[1] = self.regs[0x07] & 0xF8 != 0,
            0xFF1E if trigger => self.channel_on[2] = self.regs[0x0A] & 0x80 != 0,
            0xFF23 if trigger => self.channel_on[3] = self.regs[0x11] & 0xF8 != 0,
            0xFF12 if value & 0xF8 == 0 => self.channel_on[0] = false,
            0xFF17 if value & 0xF8 == 0 => self.channel_on[1] = false,
            0xFF1A if value & 0x80 == 0 => self.channel_on[2] = false,
            0xFF21 if value & 0xF8 == 0 => self.channel_on[3] = false,
            _ => {}
        }
    }

    pub fn tick_frame_sequencer(&mut self) {
        self.counter += 1;
        let counter = self.counter;

        if counter % 2 == 0 {
            // Sound length
        }

        if counter % 4 == 0 {
            // CH1 freq sweep
        }

        if counter == 8  {
            self.counter = 0;
            // Envelope sweep
        }
    }
}
