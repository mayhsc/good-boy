#[derive(Default, Debug)]
pub struct LengthCounter {
    pub counter: u16,
    pub enabled: bool,
}

impl LengthCounter {
    pub fn clock(&mut self) -> bool {
        print!("{} {}\n", self.counter, self.enabled);
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

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
