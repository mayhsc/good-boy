#[derive(Default, Debug)]
pub struct Envelope {
    pub volume: u8,
    timer: u8,
    period: u8,
    up: bool,
}

impl Envelope {
    pub fn load_from_nrx2(&mut self, nrx2: u8) {
        self.period = nrx2 & 7;
        self.up = nrx2 & 8 != 0;
    }

    pub fn trigger(&mut self, nrx2: u8) {
        self.load_from_nrx2(nrx2);
        self.volume = nrx2 >> 4;
        self.timer = if self.period == 0 { 8 } else { self.period };
    }

    pub fn clock(&mut self) {
        if self.period == 0 {
            return;
        }

        self.timer = self.timer.saturating_sub(1);

        if self.timer != 0 {
            return;
        }

        self.timer = self.period;

        if self.up && self.volume < 15 {
            self.volume += 1;
        } else if !self.up && self.volume > 0 {
            self.volume -= 1;
        }
    }
}
