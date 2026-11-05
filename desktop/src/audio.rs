use std::collections::VecDeque;

use sdl3::{
    AudioSubsystem,
    audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream, AudioStreamWithCallback},
};

use crate::session::Session;

pub(crate) struct EmulatorAudio {
    pub samples: VecDeque<f32>,
}

impl AudioCallback<f32> for EmulatorAudio {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        let mut output = vec![0.0f32; requested as usize];

        for sample in output.iter_mut() {
            *sample = self.samples.pop_front().unwrap_or(0.0);
        }

        stream.put_data_f32(&output).unwrap();
    }
}
pub struct AudioOutput {
    stream: AudioStreamWithCallback<EmulatorAudio>,
    scratch: Vec<f32>,
}

impl AudioOutput {
    pub fn new(audio: &AudioSubsystem) -> Self {
        let spec = AudioSpec::new(Some(44_100), Some(2), Some(AudioFormat::f32_sys()));
        let stream = audio
            .open_playback_stream(
                &spec,
                EmulatorAudio {
                    samples: VecDeque::new(),
                },
            )
            .unwrap();
        stream.resume().unwrap();
        Self {
            stream,
            scratch: Vec::with_capacity(2048),
        }
    }

    pub fn submit(&mut self, session: &mut Session) {
        session.swap_samples(&mut self.scratch);
        let _ = self.stream.put_data_f32(&self.scratch);
        self.scratch.clear();
    }

    pub fn queued_bytes(&self) -> i32 {
        self.stream.available_bytes().unwrap()
    }
}
