use crate::apu::{envelop::Envelope, length::LengthCounter};

#[derive(Debug, Default)]
pub(super) struct NoiseChannel {
    nr41: u8,
    nr42: u8,
    nr43: u8,
    nr44: u8,
    pub channel_on: bool,

    frequency_timer: u32,
    lfsr: u16,
    envelop: Envelope,
    length_counter: LengthCounter,
    pub skip_length: bool,
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
            0xFF20 => {
                self.nr41 = v;
                self.length_counter.load(64 - (v as u16 & 0x3F));
            }
            0xFF21 => {
                self.nr42 = v;

                if !self.dac_enabled() {
                    self.channel_on = false;
                }

                self.envelop.load_from_nrx2(v);
            }
            0xFF22 => self.nr43 = v,
            0xFF23 => {
                self.nr44 = v;
                if self.length_counter.set_enabled(v & 0x40 != 0) && !self.skip_length {
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
        self.channel_on = self.dac_enabled();

        self.envelop.trigger(self.nr42);

        self.lfsr = 0x7FFF;

        self.frequency_timer = self.noise_divisor() << self.clock_shift();

        let reloaded = self.length_counter.counter == 0;
        self.length_counter.on_trigger(64);
        if reloaded && self.length_counter.enabled && !self.skip_length {
            self.length_counter.counter -= 1;
        }
    }

    pub fn tick(&mut self) {
        if self.frequency_timer > 0 {
            self.frequency_timer -= 1;
        }

        if self.frequency_timer == 0 {
            self.frequency_timer = self.noise_divisor() << self.clock_shift();

            let xor_result = (self.lfsr & 0x01) ^ ((self.lfsr >> 1) & 0x01);

            self.lfsr >>= 1;
            self.lfsr |= xor_result << 14;

            if self.nr43 & 0x08 != 0 {
                self.lfsr &= !(1 << 6);
                self.lfsr |= xor_result << 6;
            }
        }
    }

    fn noise_divisor(&self) -> u32 {
        const DIVISORS: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

        DIVISORS[(self.nr43 & 0x07) as usize]
    }

    fn clock_shift(&self) -> u32 {
        (self.nr43 >> 4) as u32
    }

    fn dac_enabled(&self) -> bool {
        self.nr42 & 0xF8 != 0
    }

    pub fn clock_envelop(&mut self) {
        self.envelop.clock();
    }

    pub fn clock_length(&mut self) {
        if self.length_counter.clock() {
            self.channel_on = false;
        }
    }

    pub fn get_dac_output(&self) -> f32 {
        if !self.dac_enabled() || !self.channel_on {
            return 0.0;
        }

        // LFSR bit 0 is inverted to produce the output.
        let dac_input = ((!self.lfsr & 1) as u8) * self.envelop.volume;

        (dac_input as f32 / 7.5) - 1.0
    }

    pub fn load_length_only(&mut self, v: u8) {
        self.length_counter.load(64 - (v as u16 & 0x3F));
    }

    pub fn power_off(&mut self) {
        let counter = self.length_counter.counter;
        *self = NoiseChannel::default();
        self.length_counter.load(counter);
    }
}
