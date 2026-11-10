#[derive(Default, Debug)]
pub struct LengthCounter {
    pub counter: u16,
    pub enabled: bool,
}

impl LengthCounter {
    pub fn clock(&mut self) -> bool {
        if self.enabled && self.counter > 0 {
            self.counter -= 1;
            return self.counter == 0;
        }
        false
    }

    pub fn on_trigger(&mut self, max: u16) {
        if self.counter == 0 {
            self.counter = max;
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) -> bool {
        let was_not_enabled = !self.enabled;
        self.enabled = enabled;
        was_not_enabled && enabled
    }

    pub fn load(&mut self, value: u16) {
        self.counter = value;
    }
}
