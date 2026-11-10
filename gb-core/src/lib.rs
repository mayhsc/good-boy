pub mod cpu;
pub mod bus;
mod timer;
mod ppu;
pub mod joypad;
mod apu;
mod bank;
mod serial_port;

#[cfg(test)]
mod tests;

const IE_ADDR: u16 = 0xFFFF;
const IF_ADDR: u16 = 0xFF0F;

