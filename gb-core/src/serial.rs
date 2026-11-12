use std::time::Duration;

use crate::serial::link::{SerialLink};

mod link;

const BYTE_CYCLES: u32 = 4096;


#[derive(Debug, Default)]
pub struct SerialPort {
    sb: u8,
    sc: u8,
    link: Option<Box<dyn SerialLink>>,
    pending: Option<u8>,
    cycles: u32,
}

impl SerialPort {
    pub fn new() -> Self {
        Self {
            sc: 0x7E,
            ..Default::default()
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF01 => self.sb,
            0xFF02 => self.sc,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0xFF01 => self.sb = v,
            0xFF02 => self.write_sc(v),
            _ => unreachable!(),
        }
    }

    pub fn set_link(&mut self, link: Option<Box<dyn SerialLink>>) {
        self.link = link
    }

    pub fn tick(&mut self) -> bool {
        let Some(link) = self.link.as_mut() else {
            return false;
        };

        if self.sc & 0x80 == 0 && !link.is_connected() {
            return false;
        }

        if self.sc & 0x81 == 0x81 {
            self.cycles += 1;

            if self.cycles < BYTE_CYCLES {
                return false;
            }

            let incoming = link
                .recv_timeout(Duration::from_millis(200))
                .unwrap_or(0xFF);

            return self.finish(incoming);
        }

        self.cycles = (self.cycles + 1) % 256;

        if self.cycles != 0 {
            return false;
        }

        if let Some(b) = link.try_recv() {
            self.pending = Some(b);
        }

        if self.sc & 0x81 == 0x80 {
            if let Some(b) = self.pending.take() {
                link.send(self.sb);
                return self.finish(b);
            }
        }

        false
    }

    fn finish(&mut self, incoming: u8) -> bool {
        self.sb = incoming;
        self.sc &= !0x80;
        true
    }

    fn write_sc(&mut self, v: u8) {
        self.sc = v | 0x7E;
        if v & 0x81 == 0x81 {
            self.cycles = 0;
            let Some(link) = self.link.as_mut() else {
                return;
            };
            link.send(self.sb);
        }
    }
}
