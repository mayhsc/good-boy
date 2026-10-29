pub struct Timer {
    div: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    overflow_cycles_left: Option<i8>,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            overflow_cycles_left: None,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct TimerEvents {
    pub interrupt: bool,
    pub apu_tick: bool,
}

impl Timer {
    pub fn new() -> Self {
        Timer::default()
    }

    pub fn tick(&mut self) -> TimerEvents {
        let mut ev = TimerEvents::default();
        let old_signal = signal(self.div, self.tac);
        let old_bit = (self.div >> 12) & 1;

        self.div = self.div.wrapping_add(1);

        ev.apu_tick = old_bit == 1 && (self.div >> 12) & 1 == 0;

        if let Some(v) = self.overflow_cycles_left {
            if v == 1 {
                self.tima = self.tma;
                ev.interrupt = true;
                self.overflow_cycles_left = None;
            } else {
                self.overflow_cycles_left = Some(v - 1);
            }
        }

        if old_signal && !signal(self.div, self.tac) {
            self.increment_tima();
        }
        ev
    }

    fn increment_tima(&mut self) {
        let (r, overflow) = self.tima.overflowing_add(1);
        self.tima = r;
        if overflow {
            self.overflow_cycles_left = Some(4);
        }
    }

    pub fn set_timer(&mut self, addr: u16, v: u8) -> TimerEvents {
        let mut ev = TimerEvents::default();
        match addr {
            0xFF04 => {
                ev.apu_tick = (self.div >> 12) & 1 == 1;
                if signal(self.div, self.tac) {
                    self.increment_tima();
                }
                self.div = 0;
            }
            0xFF05 => self.tima = v,
            0xFF06 => self.tma = v,
            0xFF07 => {
                let old = signal(self.div, self.tac);
                self.tac = v;
                if old && !signal(self.div, self.tac) {
                    self.increment_tima();
                }
            }
            _ => unreachable!(),
        }
        ev
    }

    pub fn get_timer(&self, addr: u16) -> u8 {
        match addr {
            0xFF04 => (self.div >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => panic!(),
        }
    }
}

fn signal(div: u16, tac: u8) -> bool {
    let bit = match tac & 3 {
        0 => 9,
        1 => 3,
        2 => 5,
        _ => 7,
    };
    (tac & 0b100 != 0) && (div >> bit) & 1 == 1
}
