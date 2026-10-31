#[derive(Debug, Default)]
pub(super) struct PulseChannel {
    nrx0: u8,
    nrx1: u8,
    nrx2: u8,
    nrx3: u8,
    nrx4: u8,
    pub channel_on: bool,

    // Envelop
    frequency_timer: u16,
    period_timer: u8,
    volume: u8,
    wave_duty_position: usize,

    // Sweep (Channel 1)
    sweep_enabled: bool,
    shadow_frequency: u16,
    sweep_timer: u8,

    // Length
    length_timer: u8,
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
            1 => {
                self.nrx1 = v;
                self.length_timer = 64 - (v & 0x3F);
            }
            2 => {
                self.nrx2 = v;
                if !self.dac_enabled() {
                    self.channel_on = false;
                }
            }
            3 => self.nrx3 = v,
            4 => {
                self.nrx4 = v;

                if v & 0x80 != 0 {
                    self.trigger();
                }
            }
            _ => unreachable!(),
        }
    }

    fn trigger(&mut self) {
        self.channel_on = self.dac_enabled();

        self.frequency_timer = (2048 - self.frequency()) * 4;

        self.volume = self.nrx2 >> 4;

        let p = self.enevelop_period();
        self.period_timer = if p == 0 { 8 } else { p };

        self.wave_duty_position = 0;

        self.shadow_frequency = self.frequency();

        let sweep_period = self.sweep_period();
        self.sweep_timer = if sweep_period == 0 { 8 } else { sweep_period };

        self.sweep_enabled = sweep_period != 0 || self.sweep_shift() != 0;

        if self.length_timer == 0 {
            self.length_timer = 64;
        }

        if self.sweep_shift() != 0 {
            self.calculate_shadow_frequency();
        }
    }

    pub fn tick(&mut self) {
        if self.frequency_timer > 0 {
            self.frequency_timer -= 1;
        }

        if self.frequency_timer == 0 {
            self.frequency_timer = (2048 - self.frequency()) * 4;
            self.wave_duty_position = (self.wave_duty_position + 1) % 8
        }
    }

    fn frequency(&self) -> u16 {
        (self.nrx4 as u16 & 0x07) << 8 | self.nrx3 as u16
    }

    fn set_frequency(&mut self, frequency: u16) {
        self.nrx3 = frequency as u8;
        self.nrx4 = (self.nrx4 & 0xF8) | ((frequency >> 8) as u8 & 0x07);
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
        if self.volume < 0xF && self.envelope_direction_upwards() {
            self.volume += 1;
        } else if self.volume > 0x0 && !self.envelope_direction_upwards() {
            self.volume -= 1;
        }
    }

    fn enevelop_period(&self) -> u8 {
        self.nrx2 & 0x7
    }

    fn envelope_direction_upwards(&self) -> bool {
        self.nrx2 & 0x08 != 0
    }

    pub fn clock_sweep(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }

        if self.sweep_timer != 0 {
            return;
        }

        let period = self.sweep_period();

        self.sweep_timer = if period == 0 { 8 } else { period };

        if !self.sweep_enabled || period == 0 {
            return;
        }

        if self.sweep_shift() == 0 {
            return;
        }

        let new_frequency = self.calculate_shadow_frequency();

        if new_frequency <= 2047 && self.channel_on {
            self.shadow_frequency = new_frequency;
            self.set_frequency(new_frequency);

            self.calculate_shadow_frequency();
        }
    }

    fn sweep_period(&self) -> u8 {
        (self.nrx0 >> 4) & 0x07
    }

    fn calculate_shadow_frequency(&mut self) -> u16 {
        let delta = self.shadow_frequency >> self.sweep_shift();

        let new_frequency = if self.sweep_increasing() {
            self.shadow_frequency.saturating_add(delta)
        } else {
            self.shadow_frequency.saturating_sub(delta)
        };

        if new_frequency > 2047 {
            self.channel_on = false;
        }

        new_frequency
    }

    fn sweep_shift(&self) -> u8 {
        self.nrx0 & 0x07
    }

    fn sweep_increasing(&self) -> bool {
        (self.nrx0 >> 3) & 0x01 == 1
    }

    pub fn clock_length(&mut self) {
        if self.length_timer > 0 && self.length_enabled() {
            self.length_timer -= 1;
            if self.length_timer == 0 {
                self.channel_on = false;
            }
        }
    }

    fn length_enabled(&self) -> bool {
        (self.nrx4 >> 6) & 0x01 != 0
    }

    fn wave_duty_pattern(&self) -> usize {
        ((self.nrx1 >> 6) & 0x3) as usize
    }

    fn duty_cycle_amplitude(&self) -> u8 {
        WAVE_DUTY_TABLE[self.wave_duty_pattern()][self.wave_duty_position]
    }

    fn dac_enabled(&self) -> bool {
        self.nrx2 & 0xF8 != 0
    }

    pub fn get_dac_output(&self) -> f32 {
        if !self.dac_enabled() || !self.channel_on {
            return 0.0;
        }

        let dac_input = self.duty_cycle_amplitude() as f32 * self.volume as f32;

        (dac_input / 7.5) - 1.0
    }
}
