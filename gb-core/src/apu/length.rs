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
}
