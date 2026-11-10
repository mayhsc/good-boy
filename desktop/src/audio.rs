use egui_sdl3::sdl3::{
    AudioSubsystem,
    audio::{AudioDevice, AudioFormat, AudioSpec, AudioStreamOwner},
};

use crate::session::Session;

pub struct AudioOutput {
    stream: AudioStreamOwner,
    scratch: Vec<f32>,
    _device: AudioDevice,
}

impl AudioOutput {
    pub fn new(audio: &AudioSubsystem) -> Self {
        let spec = AudioSpec::new(Some(44_100), Some(2), Some(AudioFormat::f32_sys()));
        let device = audio.open_playback_device(&spec).unwrap();
        let stream = audio
            .new_playback_stream(&spec, None)
            .unwrap_or_else(|e| panic!("Failed to create audio playback stream: {e:?}"));

        device.bind_stream(&stream).unwrap();
        stream.resume().unwrap();

        Self {
            stream,
            scratch: Vec::with_capacity(2048),
            _device: device,
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
