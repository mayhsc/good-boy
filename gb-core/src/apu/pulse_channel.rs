use crate::apu::{envelop::Envelope, length::LengthCounter};

#[derive(Debug, Default)]
pub(super) struct PulseChannel {
    nrx0: u8,
    nrx1: u8,
    nrx2: u8,
    nrx3: u8,
    nrx4: u8,
    pub channel_on: bool,

    frequency_timer: u16,
    envelop: Envelope,
    length_counter: LengthCounter,
    wave_duty_position: usize,

    // Sweep (Channel 1)
    sweep_enabled: bool,
    shadow_frequency: u16,
    sweep_timer: u8,
    negate_mode: bool,
    pub skip_length: bool,
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
            0 => {
                self.nrx0 = v;
                if self.sweep_increasing() && self.negate_mode {
                    self.channel_on = false;
                }
            }
            1 => {
                self.nrx1 = v;
                self.length_counter.load(64 - (v as u16 & 0x3F));
            }
            2 => {
                self.nrx2 = v;
                if !self.dac_enabled() {
                    self.channel_on = false;
                }
                self.envelop.load_from_nrx2(self.nrx2);
            }
            3 => self.nrx3 = v,
            4 => {
                self.nrx4 = v;
                if (self.length_counter.set_enabled(v & 0x40 != 0)) && !self.skip_length {
                    self.clock_length();
                }

                if v & 0x80 != 0 {
                    self.trigger();
                }
            }
            _ => unreachable!(),
        }
    }

    fn trigger(&mut self) {
        self.negate_mode = false;
        self.channel_on = self.dac_enabled();

        self.envelop.trigger(self.nrx2);

        self.wave_duty_position = 0;

        self.shadow_frequency = self.frequency();

        let sweep_period = self.sweep_period();
        self.sweep_timer = if sweep_period == 0 { 8 } else { sweep_period };

        self.sweep_enabled = sweep_period != 0 || self.sweep_shift() != 0;

        let reloaded = self.length_counter.counter == 0;
        self.length_counter.on_trigger(64);
        if reloaded && self.length_counter.enabled && !self.skip_length {
            self.length_counter.counter -= 1;
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
        self.envelop.clock();
    }

    pub fn clock_sweep(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }

        if self.sweep_timer != 0 {
            return;
        }

        let period = self.sweep_period();

        self.sweep_timer = if period > 0 { period } else { 8 };

        if !self.sweep_enabled || period == 0 {
            return;
        }

        let new_frequency = self.calculate_shadow_frequency();

        if new_frequency <= 2047 && self.sweep_shift() != 0 {
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
            self.negate_mode = true;
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
        (self.nrx0 >> 3) & 0x01 == 0
    }

    pub fn clock_length(&mut self) {
        if self.length_counter.clock() {
            self.channel_on = false;
        }
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

        let dac_input = self.duty_cycle_amplitude() as f32 * self.envelop.volume as f32;

        (dac_input / 7.5) - 1.0
    }

    pub fn load_length_only(&mut self, v: u8) {
        self.length_counter.load(64 - (v as u16 & 0x3F));
    }

    pub fn power_off(&mut self) {
        let counter = self.length_counter.counter;
        *self = PulseChannel::default();
        self.length_counter.load(counter);
    }
}
