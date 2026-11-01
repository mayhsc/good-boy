use std::collections::VecDeque;

use sdl3::audio::{AudioCallback, AudioStream};

pub(crate) struct EmulatorAudio {
    pub samples: VecDeque<f32>,
}

impl AudioCallback<f32> for EmulatorAudio {
    fn callback(
        &mut self,
        stream: &mut AudioStream,
        requested: i32,
    ) {
        let mut output = vec![0.0f32; requested as usize];

        for sample in output.iter_mut() {
            *sample = self.samples.pop_front().unwrap_or(0.0);
        }

        stream.put_data_f32(&output).unwrap();
    }
}