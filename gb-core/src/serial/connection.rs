use std::net::TcpStream;

#[derive(Debug, Default)]
pub enum Connection {
    #[default]
    None,
    Tcp(TcpLink),
}

pub trait SerialLink {
    fn start_master(&mut self, byte: u8);
    fn poll(&mut self) -> Option<u8>;
    fn reply(&mut self, byte: u8);
}
#[derive(Debug)]
pub struct TcpLink {
    stream: TcpStream,
}

impl SerialLink for TcpLink {
    fn start_master(&mut self, byte: u8) {}

    fn poll(&mut self) -> Option<u8> {
        todo!()
    }

    fn reply(&mut self, byte: u8) {}
}
