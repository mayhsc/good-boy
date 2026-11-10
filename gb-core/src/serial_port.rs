#[derive(Debug, Default)]
pub struct SerialPort {
    sb: u8,
    sc: u8,
}

impl SerialPort {
    pub fn new() -> Self {
        Self { sb: 00, sc: 0x7E }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF01 => self.sb,
            0xFF02 => self.sc,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8){
        match addr {
            0xFF01 => self.sb = value,
            0xFF02 => self.sc = value,
            _ => unreachable!(),
        }
    }
}
