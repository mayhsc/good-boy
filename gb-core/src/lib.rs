pub mod cpu;
pub mod bus;
pub mod timer;
pub mod ppu;
pub mod joypad;
pub mod apu;

#[cfg(test)]
mod tests;

pub const IE_ADDR: u16 = 0xFFFF;
pub const IF_ADDR: u16 = 0xFF0F;

