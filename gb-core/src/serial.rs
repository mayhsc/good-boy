use crate::serial::connection::{Connection, SerialLink};

mod connection;

#[derive(Debug, Default)]
pub struct SerialPort {
    sb: u8,
    sc: u8,
    link: Connection,
    transfer_bits: u8,
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
            0xFF02 => match self.link {
                Connection::None => 0xFF,
                Connection::Tcp(_) => self.sc,
            },
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF01 => self.sb = value,
            0xFF02 => self.sc = value,
            _ => unreachable!(),
        }
    }

}
