use crate::apu::pulse_channel::PulseChannel;

mod pulse_channel;

const CPU_FREQUENCY: f64 = 4_194_304.0;
const SAMPLE_RATE: f64 = 44_100.0;

#[derive(Default)]
#[rustfmt::skip]
pub(crate) struct Apu {
    pulse_channel1: PulseChannel,
    pulse_channel2: PulseChannel,
    nr30: u8, nr31: u8, nr32: u8, nr33: u8, nr34: u8,
    nr41: u8, nr42: u8, nr43: u8, nr44: u8,
    nr50: u8, nr51: u8,
    wave_ram: [u8; 16],
    power: bool,
    channel_on: [bool; 2],
    counter: u8,
    sample_counter: f64,
    audio_buffer: Vec<f32>
}

impl Apu {
    pub fn new() -> Self {
        Self {
            nr50: 0x77,
            nr51: 0xF3,
            power: true,
            channel_on: [false, false], // NR52 reads 0xF1
            pulse_channel1: PulseChannel::new(0x80, 0xf3, true),
            ..Default::default()
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF10..=0xFF14 => self.pulse_channel1.read(addr - 0xFF10),
            0xFF16..=0xFF19 => self.pulse_channel2.read(addr - 0xFF15),

            0xFF1A => self.nr30 | 0x7F,
            0xFF1B => 0xFF,
            0xFF1C => self.nr32 | 0x9F,
            0xFF1D => 0xFF,
            0xFF1E => self.nr34 | 0xBF,

            0xFF20 => 0xFF,
            0xFF21 => self.nr42,
            0xFF22 => self.nr43,
            0xFF23 => self.nr44 | 0xBF,

            0xFF24 => self.nr50,
            0xFF25 => self.nr51,
            0xFF26 => {
                let channel_on = [
                    self.pulse_channel1.channel_on,
                    self.pulse_channel2.channel_on,
                    self.channel_on[0],
                    self.channel_on[1],
                ];

                let status = channel_on
                    .iter()
                    .enumerate()
                    .fold(0u8, |a, (i, &on)| a | ((on as u8) << i));
                ((self.power as u8) << 7) | 0x70 | status
            }
            0xFF30..=0xFF3F => self.wave_ram[(addr - 0xFF30) as usize],
            _ => 0xFF, // 0xFF15, 0xFF1F, 0xFF27..=0xFF2F
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0xFF26 => {
                let on = v & 0x80 != 0;
                if self.power && !on {
                    *self = Apu {
                        wave_ram: self.wave_ram,
                        ..Default::default()
                    };
                } else if !self.power && on {
                    // TODO: reset frame sequencer step to 0
                }
                self.power = on;
            }
            0xFF30..=0xFF3F => self.wave_ram[(addr - 0xFF30) as usize] = v,
            _ if !self.power => {}
            0xFF10..=0xFF14 => self.pulse_channel1.write(addr - 0xFF10, v),
            0xFF16..=0xFF19 => self.pulse_channel2.write(addr - 0xFF15, v),
            0xFF1A => {
                self.nr30 = v;
                if v & 0x80 == 0 {
                    self.channel_on[0] = false;
                }
            }
            0xFF1B => self.nr31 = v,
            0xFF1C => self.nr32 = v,
            0xFF1D => self.nr33 = v,
            0xFF1E => {
                self.nr34 = v;
                if v & 0x80 != 0 {
                    self.channel_on[0] = self.nr30 & 0x80 != 0;
                }
            }
            0xFF20 => self.nr41 = v,
            0xFF21 => {
                self.nr42 = v;
                if v & 0xF8 == 0 {
                    self.channel_on[1] = false;
                }
            }
            0xFF22 => self.nr43 = v,
            0xFF23 => {
                self.nr44 = v;
                if v & 0x80 != 0 {
                    self.channel_on[1] = self.nr42 & 0xF8 != 0;
                }
            }
            0xFF24 => self.nr50 = v,
            0xFF25 => self.nr51 = v,
            _ => {}
        }
    }

    pub fn tick_frame_sequencer(&mut self) {
        let step = self.counter;
        self.counter = (self.counter + 1) % 8;
        if step % 2 == 0 { /* length */ }
        if step == 2 || step == 6 { /* sweep */ }
        if step == 7 {
            self.pulse_channel1.clock_envelop();
            self.pulse_channel2.clock_envelop();
        }
    }

    pub fn tick_frequency_timer(&mut self) {
        self.tick_audio_sampling();
        self.pulse_channel1.tick();
        self.pulse_channel2.tick();
    }

    fn get_audio_output(&self) -> f32 {
        self.pulse_channel2.get_dac_output()
    }

    pub fn tick_audio_sampling(&mut self) {
        self.sample_counter += SAMPLE_RATE;

        if self.sample_counter >= CPU_FREQUENCY {
            self.sample_counter -= CPU_FREQUENCY;

            let sample = self.get_audio_output();

            self.audio_buffer.push(sample);
        }
    }
}
