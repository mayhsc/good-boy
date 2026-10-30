#[derive(Debug, Default)]
pub(super) struct PulseChannel {
    nrx0: u8,
    nrx1: u8,
    nrx2: u8,
    nrx3: u8,
    nrx4: u8,
    pub channel_on: bool,
    frequency_timer: u16,
    period_timer: u8,
    volume: u8,
    wave_duty_position: usize,
}

const WAVE_DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

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
                self.volume = self.nrx2 >> 4;
            }
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self) {
        self.frequency_timer -= 1;
        if self.frequency_timer == 0 {
            self.frequency_timer = (2048 - self.frequency()) * 4;
            self.wave_duty_position = (self.wave_duty_position + 1) % 8
        }
    }

    fn frequency(&self) -> u16 {
        (self.nrx4 as u16 & 0x6F) << 8 | self.nrx3 as u16
    }

    pub fn clock_envelop(&mut self) {
        if self.enevelop_period() == 0 {
            return;
        }

        if self.period_timer > 0 {
            self.period_timer -= 1;
        }

        if self.period_timer != 0 {
            return;
        }

        self.period_timer = self.enevelop_period();
        if self.volume < 0xF && self.envelop_dxn_upwards() {
            self.volume += 1;
        } else if self.volume > 0x0 && !self.envelop_dxn_upwards() {
            self.volume -= 1;
        }
    }

    fn enevelop_period(&self) -> u8 {
        self.nrx2 & 0x7
    }

    fn envelop_dxn_upwards(&self) -> bool {
        (self.nrx2 >> 3) & 0x3 != 0
    }

    fn wave_duty_pattern(&self) -> usize {
        ((self.nrx1 >> 6) & 0x3) as usize
    }

    fn duty_cycle_amplitude(&self) -> u8 {
        WAVE_DUTY_TABLE[self.wave_duty_pattern()][self.wave_duty_position]
    }

    pub fn get_dac_output(&self) -> f32 {
        let dac_input = self.duty_cycle_amplitude() as f32 * self.volume as f32;

        (dac_input / 7.5) - 1.0
    }
}
