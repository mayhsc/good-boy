use crate::apu::{
    noise_channel::NoiseChannel, pulse_channel::PulseChannel, wave_channel::WaveChannel,
};

mod envelop;
mod length;
mod noise_channel;
mod pulse_channel;
mod wave_channel;

const CPU_FREQUENCY: u64 = 4_194_304;
const SAMPLE_RATE: u64 = 44_100;
const BUFFER_CAPACITY: usize = 8192;

#[derive(Default)]
#[rustfmt::skip]
pub(crate) struct Apu {
    pulse_channel1: PulseChannel,
    pulse_channel2: PulseChannel,
    wave_channel: WaveChannel,
    noise_channel: NoiseChannel,
    nr50: u8, nr51: u8,
    power: bool,
    counter: u8,
    sample_counter: u64,
    samples: Vec<f32>,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(BUFFER_CAPACITY),
            nr50: 0x77,
            nr51: 0xF3,
            power: true,
            pulse_channel1: PulseChannel::new(0x80, 0xf3, true),
            ..Default::default()
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF10..=0xFF14 => self.pulse_channel1.read(addr - 0xFF10),
            0xFF16..=0xFF19 => self.pulse_channel2.read(addr - 0xFF15),
            0xFF1A..=0xFF1E => self.wave_channel.read(addr),
            0xFF20..=0xFF23 => self.noise_channel.read(addr),
            0xFF24 => self.nr50,
            0xFF25 => self.nr51,
            0xFF26 => {
                let channel_on = [
                    self.pulse_channel1.channel_on,
                    self.pulse_channel2.channel_on,
                    self.wave_channel.channel_on,
                    self.noise_channel.channel_on,
                ];

                let status = channel_on
                    .iter()
                    .enumerate()
                    .fold(0u8, |a, (i, &on)| a | ((on as u8) << i));
                ((self.power as u8) << 7) | 0x70 | status
            }
            0xFF30..=0xFF3F => self.wave_channel.read(addr),
            _ => 0xFF, // 0xFF15, 0xFF1F, 0xFF27..=0xFF2F
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0xFF26 => {
                let on = v & 0x80 != 0;
                if self.power && !on {
                    self.pulse_channel1.power_off();
                    self.pulse_channel2.power_off();
                    self.wave_channel.power_off();
                    self.noise_channel.power_off();
                    self.nr50 = 0;
                    self.nr51 = 0;
                    self.counter = 0;
                } else if !self.power && on {
                    self.counter = 0
                }
                self.power = on;
                self.wave_channel.power = on;
            }
            0xFF30..=0xFF3F => self.wave_channel.write(addr, v),
            0xFF11 if !self.power => self.pulse_channel1.load_length_only(v),
            0xFF16 if !self.power => self.pulse_channel2.load_length_only(v),
            0xFF1B if !self.power => self.wave_channel.load_length_only(v),
            0xFF20 if !self.power => self.noise_channel.load_length_only(v),

            _ if !self.power => {}
            0xFF10..=0xFF14 => self.pulse_channel1.write(addr - 0xFF10, v),
            0xFF16..=0xFF19 => self.pulse_channel2.write(addr - 0xFF15, v),
            0xFF1A..=0xFF1E => self.wave_channel.write(addr, v),
            0xFF20..=0xFF23 => self.noise_channel.write(addr, v),
            0xFF24 => self.nr50 = v,
            0xFF25 => self.nr51 = v,
            _ => {}
        }
    }

    pub fn tick_frame_sequencer(&mut self) {
        if !self.power {
            return;
        }

        let step = self.counter;

        self.counter = (self.counter + 1) % 8;
        let skip_length = self.counter % 2 == 0;

        self.pulse_channel1.skip_length = skip_length;
        self.pulse_channel2.skip_length = skip_length;
        self.wave_channel.skip_length = skip_length;
        self.noise_channel.skip_length = skip_length;

        if step % 2 == 0 {
            self.pulse_channel1.clock_length();
            self.pulse_channel2.clock_length();
            self.wave_channel.clock_length();
            self.noise_channel.clock_length();
        }
        if step == 2 || step == 6 {
            self.pulse_channel1.clock_sweep();
        }
        if step == 7 {
            self.pulse_channel1.clock_envelop();
            self.pulse_channel2.clock_envelop();
            self.noise_channel.clock_envelop();
        }
    }

    pub fn tick_frequency_timer(&mut self) {
        self.tick_audio_sampling();
        self.pulse_channel1.tick();
        self.pulse_channel2.tick();
        self.wave_channel.tick();
        self.noise_channel.tick();
    }

    fn mix_channels(&self) -> (f32, f32) {
        let channels = [
            self.pulse_channel1.get_dac_output(),
            self.pulse_channel2.get_dac_output(),
            self.wave_channel.get_dac_output(),
            self.noise_channel.get_dac_output(),
        ];

        let mut left = 0.0;
        let mut right = 0.0;

        for (i, sample) in channels.iter().enumerate() {
            if self.nr51 & (1 << i) != 0 {
                right += sample;
            }

            if self.nr51 & (1 << (i + 4)) != 0 {
                left += sample;
            }
        }

        (left / 4.0, right / 4.0)
    }
    fn tick_audio_sampling(&mut self) {
        self.sample_counter += SAMPLE_RATE;

        if self.sample_counter >= CPU_FREQUENCY {
            self.sample_counter -= CPU_FREQUENCY;

            let (l, r) = self.mix_channels();

            self.samples.push(l);
            self.samples.push(r);
        }
    }

    pub fn swap_samples(&mut self, out: &mut Vec<f32>) {
        out.clear();
        std::mem::swap(&mut self.samples, out);
    }
}
