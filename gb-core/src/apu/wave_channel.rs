use crate::apu::length::LengthCounter;

#[derive(Debug, Default)]
pub(super) struct WaveChannel {
    nr30: u8,
    nr31: u8,
    nr32: u8,
    nr33: u8,
    nr34: u8,

    wave_ram: [u8; 16],

    pub channel_on: bool,

    frequency_timer: u16,
    wave_position: usize,
    length_counter: LengthCounter,
    pub skip_length: bool,

    pub power: bool,
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
            0xFF30..=0xFF3F => {
                self.wave_ram[(addr - 0xFF30) as usize] = v;
            }
            _ if !self.power => {}
            0xFF1A => {
                self.nr30 = v;

                if v & 0x80 == 0 {
                    self.channel_on = false;
                }
            }
            0xFF1B => {
                self.nr31 = v;
                self.length_counter.load(256 - v as u16);
                println!("  load ch -> {}", self.length_counter.counter);
            }
            0xFF1C => self.nr32 = v,
            0xFF1D => self.nr33 = v,
            0xFF1E => {
                self.nr34 = v;
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

        self.wave_position = 0;

        self.frequency_timer = (2048 - self.frequency()) * 2;

        let reloaded = self.length_counter.counter == 0;
        self.length_counter.on_trigger(256);
        if reloaded && self.length_counter.enabled && !self.skip_length {
            self.length_counter.counter -= 1;
        }
    }

    pub fn tick(&mut self) {
        if self.frequency_timer > 0 {
            self.frequency_timer -= 1;
        }

        if self.frequency_timer == 0 {
            self.frequency_timer = (2048 - self.frequency()) * 2;

            self.wave_position = (self.wave_position + 1) % 32;
        }
    }

    fn frequency(&self) -> u16 {
        ((self.nr34 as u16 & 0x07) << 8) | self.nr33 as u16
    }

    fn dac_enabled(&self) -> bool {
        self.nr30 & 0x80 != 0
    }

    fn volume_code(&self) -> u8 {
        (self.nr32 >> 5) & 0x03
    }

    fn wave_sample(&self) -> u8 {
        let byte = self.wave_ram[self.wave_position / 2];

        if self.wave_position & 1 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        }
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

        let sample = self.wave_sample();

        let volume = self.volume_code();

        let dac_input = match volume {
            0 => 0,
            1 => sample,
            2 => sample >> 1,
            3 => sample >> 2,
            _ => unreachable!(),
        };

        (dac_input as f32 / 7.5) - 1.0
    }

    pub fn load_length_only(&mut self, v: u8) {
        self.length_counter.load(256 - v as u16);
    }

    pub fn power_off(&mut self) {
        let counter = self.length_counter.counter;
        *self = WaveChannel::reset(&self);
        self.length_counter.load(counter);
    }
}
